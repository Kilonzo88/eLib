'use client';

import Image from 'next/image';
import { useParams, notFound } from 'next/navigation';
import { useEffect, useState, useCallback } from 'react';
import { useAuth, SignIn } from '@clerk/nextjs';
import dynamic from 'next/dynamic';
import { cn } from '@/lib/utils';

// Dynamic imports to avoid SSR issues with epub.js and pdf.js
const EpubReader = dynamic(() => import('@/components/reader/EpubReader'), { ssr: false });
const PdfReader = dynamic(() => import('@/components/reader/PdfReader'), { ssr: false });

interface BookData {
    title: string;
    author: string;
    coverURL: string;
    source: 'gutenberg' | 'user';
    fileData: ArrayBuffer | null;
    fileType: 'epub' | 'pdf';
    error?: string;
}

export default function BookReaderPage() {
    const params = useParams();
    const slug = typeof params?.slug === 'string' ? params.slug : '';
    const { userId, getToken, isLoaded } = useAuth();
    const [book, setBook] = useState<BookData | null | undefined>(undefined); // undefined = loading

    const loadBook = useCallback(async () => {
        if (!slug || !isLoaded || !userId) return;

        const isGutenberg = /^\d+$/.test(slug);
        const actualSlug = isGutenberg ? `gutenberg-${slug}` : slug;

        try {
            const token = await getToken();
            const headers: Record<string, string> = token ? { Authorization: `Bearer ${token}` } : {};

            // Fetch book metadata and raw file bytes in parallel through our backend
            const [bookRes, fileRes] = await Promise.all([
                fetch(isGutenberg ? `/api/books/gutenberg/${slug}` : `/api/books/${slug}`, {
                    headers,
                    cache: 'no-store'
                }),
                fetch(`/api/books/${actualSlug}/file`, {
                    headers,
                })
            ]);

            if (!bookRes.ok) {
                const errorText = await bookRes.text().catch(() => `Backend error ${bookRes.status}`);
                setBook({
                    title: 'Error',
                    author: 'System',
                    coverURL: '',
                    source: isGutenberg ? 'gutenberg' : 'user',
                    fileData: null,
                    fileType: 'epub',
                    error: errorText,
                });
                return;
            }

            const bookJson = await bookRes.json();

            // Determine file type from Content-Type header
            const contentType = fileRes.headers.get('content-type') || '';
            const fileType = contentType.includes('epub') ? 'epub' : 'pdf';

            // Get raw bytes as ArrayBuffer — no blob URLs, no external fetches
            let fileData: ArrayBuffer | null = null;
            if (fileRes.ok) {
                fileData = await fileRes.arrayBuffer();
            }

            setBook({
                title: bookJson.title ?? 'Unknown Title',
                author: bookJson.author ?? 'Unknown Author',
                coverURL: bookJson.cover_url ?? '',
                source: isGutenberg ? 'gutenberg' : 'user',
                fileData,
                fileType: fileType as 'epub' | 'pdf',
            });

        } catch (e: unknown) {
            setBook({
                title: 'Network Error',
                author: 'System',
                coverURL: '',
                source: 'user',
                fileData: null,
                fileType: 'pdf',
                error: e instanceof Error ? e.message : String(e),
            });
        }
    }, [slug, isLoaded, userId, getToken]);

    // Initial load
    useEffect(() => {
        loadBook();
    }, [loadBook]);

    // Handle text selection from readers (for AI context)
    const handleTextSelect = useCallback((text: string, context: string | number) => {
        console.log('[AI Context] Selected text:', text, 'Context:', context);
    }, []);

    if (!isLoaded) {
        return (
            <div className="flex items-center justify-center py-32 text-[var(--muted-foreground)]">
                <div className="flex flex-col items-center gap-4">
                    <div className="w-8 h-8 border-4 border-[var(--primary)] border-t-transparent rounded-full animate-spin" />
                    <span>Loading library access…</span>
                </div>
            </div>
        );
    }

    if (!userId) {
        return (
            <div className="flex flex-col items-center justify-center py-12 md:py-20">
                <div className="w-full max-w-[400px] flex flex-col items-center">
                    <div className="text-center mb-10 px-4">
                        <h2 className="text-3xl font-serif font-bold text-[var(--foreground)] mb-3">
                            Log in or Sign Up in Seconds!
                        </h2>
                        <p className="text-[var(--muted-foreground)] text-sm max-w-[280px] mx-auto leading-relaxed">
                            Use your email or social logins to access your digital library
                        </p>
                    </div>
                    
                    <div className="w-full bg-[var(--card)] border border-[var(--border)] rounded-[2.5rem] p-3 md:p-6 shadow-2xl shadow-[rgba(0,0,0,0.05)] ring-1 ring-black/[0.02] flex items-center justify-center overflow-hidden">
                        <div className="w-full flex justify-center scale-95 md:scale-100 origin-center">
                            <SignIn routing="hash" />
                        </div>
                    </div>

                    <p className="mt-8 text-xs text-[var(--muted-foreground)] opacity-60">
                        Secure authentication powered by Clerk
                    </p>
                </div>
            </div>
        );
    }

    if (book === undefined) {
        return (
            <div className="flex items-center justify-center py-32 text-[var(--muted-foreground)]">
                <div className="flex flex-col items-center gap-4">
                    <div className="w-8 h-8 border-4 border-[var(--primary)] border-t-transparent rounded-full animate-spin" />
                    <span>Preparing your library…</span>
                </div>
            </div>
        );
    }

    if (book === null) return notFound();

    if (book.error) {
        return (
            <div className="flex flex-col items-center justify-center py-24 text-center">
                <div className="mb-4 text-6xl opacity-20">🚫</div>
                <p className="text-xl font-bold text-[var(--destructive)]">Could not load book</p>
                <p className="max-w-md mt-2 text-sm text-[var(--muted-foreground)]">
                    {book.error}
                </p>
            </div>
        );
    }

    const { title, author, coverURL, source, fileData, fileType } = book;

    return (
        <div className="flex flex-col md:gap-6 md:-mt-4 h-full">
            <header className="hidden md:flex items-center gap-6 bg-[var(--card)] border border-[var(--border)] rounded-2xl p-5 shadow-sm transition-all hover:shadow-md">
                {coverURL && (
                    <div className="relative w-18 h-28 shadow-xl rounded-lg overflow-hidden flex-shrink-0">
                        <Image
                            src={coverURL}
                            alt={title}
                            fill
                            unoptimized
                            className="object-cover"
                        />
                    </div>
                )}
                <div className="flex flex-col gap-1 min-w-0">
                    <h1 className="font-serif text-2xl font-bold text-[var(--foreground)] leading-tight line-clamp-2">
                        {title}
                    </h1>
                    <p className="text-sm text-[var(--muted-foreground)]">by {author}</p>
                    <div className="flex items-center gap-2 mt-1">
                        <span className="text-[10px] uppercase tracking-wider font-bold px-1.5 py-0.5 bg-[var(--muted)] text-[var(--muted-foreground)] rounded">
                            {source === 'gutenberg' ? 'Public' : 'Personal'}
                        </span>
                        <span className="text-[10px] uppercase tracking-wider font-bold px-1.5 py-0.5 bg-[var(--muted)] text-[var(--muted-foreground)] rounded">
                            {fileType.toUpperCase()}
                        </span>
                    </div>
                </div>
            </header>

            {/* Reader */}
            {fileData ? (
                fileType === 'epub' ? (
                    <EpubReader data={fileData} onTextSelect={(text, cfi) => handleTextSelect(text, cfi)} />
                ) : (
                    <PdfReader data={fileData} onTextSelect={(text, page) => handleTextSelect(text, page)} />
                )
            ) : (
                <div className="flex flex-col items-center justify-center gap-6 py-32 text-center bg-[var(--card)] border border-[var(--border)] rounded-2xl shadow-inner">
                    <div className="w-16 h-16 border-4 border-[var(--muted)] border-t-[var(--primary)] rounded-full animate-spin" />
                    <div className="flex flex-col gap-2">
                        <p className="text-xl font-serif text-[var(--foreground)]">
                            Preparing your book…
                        </p>
                        <p className="text-sm text-[var(--muted-foreground)] max-w-xs mx-auto">
                            The file is being fetched. This may take a moment for larger books.
                        </p>
                    </div>
                </div>
            )}
        </div>
    );
}

