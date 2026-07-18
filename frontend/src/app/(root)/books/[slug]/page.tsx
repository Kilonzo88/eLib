'use client';

import Image from 'next/image';
import { useParams, notFound } from 'next/navigation';
import { useEffect, useState, useCallback, useRef } from 'react';
import { useAuth, SignIn } from '@clerk/nextjs';
import dynamic from 'next/dynamic';
import { cn } from '@/lib/utils';
import BookChat from '@/components/chat/BookChat';
import DraggableAiButton from '@/components/ui/DraggableAiButton';

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
    const [selectedText, setSelectedText] = useState<string | null>(null);
    const [isMobileChatOpen, setIsMobileChatOpen] = useState(false);
    
    // Drawer resize state
    const [drawerHeight, setDrawerHeight] = useState(35); // 35vh default
    const [isDragging, setIsDragging] = useState(false);
    const dragStartY = useRef(0);
    const startHeight = useRef(35);

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
        // eslint-disable-next-line react-hooks/set-state-in-effect
        loadBook();
    }, [loadBook]);

    // Handle text selection from readers (for AI context)
    const handleTextSelect = useCallback((text: string, context: string | number) => {
        console.log('[AI Context] Selected text:', text, 'Context:', context);
        if (text && text.trim().length > 0) {
            setSelectedText(text.trim());
        }
    }, []);

    // Touch Handlers for Dragging Drawer
    const startDrag = useCallback((clientY: number) => {
        setIsDragging(true);
        dragStartY.current = clientY;
        startHeight.current = drawerHeight;
    }, [drawerHeight]);

    const onDragMove = useCallback((clientY: number) => {
        if (!isDragging) return;
        const deltaY = dragStartY.current - clientY;
        const deltaVh = (deltaY / window.innerHeight) * 100;
        let newHeight = startHeight.current + deltaVh;
        if (newHeight > 90) newHeight = 90; // max expansion
        if (newHeight < 15) {
            setIsMobileChatOpen(false);
            setDrawerHeight(35);
            setIsDragging(false);
            return;
        }
        setDrawerHeight(newHeight);
    }, [isDragging]);

    const stopDrag = useCallback(() => {
        setIsDragging(false);
        if (drawerHeight < 25) {
            setIsMobileChatOpen(false);
            setDrawerHeight(35);
        } else if (drawerHeight < 50) {
            setDrawerHeight(35); // snap back to 35vh default
        } else {
            setDrawerHeight(85); // snap to expanded mode 85vh
        }
    }, [drawerHeight]);

    // Handle mouse events if user drags the handle with a mouse
    useEffect(() => {
        const handleMouseUp = () => stopDrag();
        const handleMouseMove = (e: MouseEvent) => onDragMove(e.clientY);
        if (isDragging) {
            window.addEventListener('mouseup', handleMouseUp);
            window.addEventListener('mousemove', handleMouseMove);
        }
        return () => {
            window.removeEventListener('mouseup', handleMouseUp);
            window.removeEventListener('mousemove', handleMouseMove);
        };
    }, [isDragging, onDragMove, stopDrag]);

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
        <div className="flex flex-col gap-6 md:-mt-4 h-full">
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

            <div className="flex flex-col lg:flex-row gap-6 items-start h-full pb-8">
                {/* Reader Column */}
                <div className="flex-1 w-full min-w-0 lg:max-w-[calc(100%-374px)]">
                    {fileData ? (
                        fileType === 'epub' ? (
                            <EpubReader data={fileData} onTextSelect={handleTextSelect} />
                        ) : (
                            <PdfReader data={fileData} onTextSelect={handleTextSelect} />
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

                {/* Chat Companion Column & Mobile FAB */}
                {fileData && (
                    <>
                        {/* Mobile Backdrop */}
                        <div 
                            className={cn(
                                "lg:hidden fixed inset-0 z-[60] transition-opacity duration-300 pointer-events-none",
                                isMobileChatOpen ? "opacity-100" : "opacity-0"
                            )} 
                        />

                        {/* Mobile FAB */}
                        <DraggableAiButton onClick={() => setIsMobileChatOpen(true)} />

                        {/* Chat Container (Drawer on Mobile, Sidebar on Desktop) */}
                        <div 
                            className={cn(
                                "flex-shrink-0 z-[70]",
                                isDragging ? "" : "transition-all duration-300 ease-[cubic-bezier(0.175,0.885,0.32,1.2)]",
                                "lg:w-[350px] lg:sticky lg:top-6 lg:block lg:transform-none lg:z-auto lg:!h-auto lg:!transition-none",
                                "fixed left-[3%] right-[3%] w-[94%] mx-auto bg-[var(--card)]/90 backdrop-blur-xl rounded-[24px] shadow-[0_-10px_40px_rgba(0,0,0,0.15)] lg:bg-transparent lg:shadow-none lg:rounded-none lg:left-auto lg:right-auto lg:w-auto",
                                "pb-[env(safe-area-inset-bottom)]",
                                isMobileChatOpen ? "bottom-4 translate-y-0 opacity-100" : "-bottom-full translate-y-[20%] opacity-0 lg:bottom-auto lg:translate-y-0 lg:opacity-100"
                            )}
                            style={{ 
                                height: 'auto',
                                maxHeight: '100%',
                                ...(isMobileChatOpen || isDragging ? { height: `calc(${drawerHeight}vh + env(safe-area-inset-bottom))` } : {}) 
                            }}
                        >
                            {/* Mobile Handle (The Pill) */}
                            <div 
                                className="lg:hidden flex justify-center pt-3 pb-2 w-full cursor-grab active:cursor-grabbing touch-none" 
                                onTouchStart={(e) => startDrag(e.touches[0].clientY)}
                                onTouchMove={(e) => onDragMove(e.touches[0].clientY)}
                                onTouchEnd={stopDrag}
                                onMouseDown={(e) => startDrag(e.clientY)}
                            >
                                <div className="w-[40px] h-[4px] bg-[var(--muted-foreground)]/30 rounded-full" />
                            </div>
                            
                            <div className="h-[calc(100%-24px)] lg:h-auto overflow-hidden">
                                <BookChat
                                    slug={slug}
                                    selectedText={selectedText}
                                    onClearSelection={() => setSelectedText(null)}
                                    className="h-full lg:h-[550px] border-0 rounded-none bg-transparent lg:border lg:border-[var(--border)] lg:bg-[var(--card)] lg:rounded-2xl"
                                />
                            </div>
                        </div>
                    </>
                )}
            </div>
        </div>
    );
}

