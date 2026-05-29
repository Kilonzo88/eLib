import React from 'react';
import { redirect } from 'next/navigation';
import { auth } from '@clerk/nextjs/server';
import { LibraryHero } from '@/components/ui/library-hero';
import { MyLibrarySection } from '@/components/ui/MyLibrarySection';
import BookCard from '@/components/ui/BookCard';
import { GUTENBERG_CACHE_TIME } from '@/lib/constants';

interface GutenbergBook {
    _id: string;
    title: string;
    author: string;
    slug: string;
    coverURL: string;
}

const GUTENBERG_LIMIT = 32;

export default async function Page() {
    const { userId } = await auth();


    // SSR fetch the Gutenberg catalogue internally from backend
    let gutenbergBooks: GutenbergBook[] = [];
    try {
        const apiUrl = process.env.API_URL || 'http://localhost:8081';
        const res = await fetch(`${apiUrl}/api/books/public`, {
            cache: 'no-store',
        });
        if (res.ok) {
            const data = await res.json();
            if (data && Array.isArray(data)) {
                gutenbergBooks = data.slice(0, GUTENBERG_LIMIT).map((b: any) => ({
                    _id: b._id || b.slug,
                    title: b.title,
                    author: b.author || 'Unknown Author',
                    slug: b.slug,
                    coverURL: b.cover_url || 'https://placehold.co/400x600?text=No+Cover',
                }));
            }
        }
    } catch (e) {
        // Leave empty on failure
    }

    return (
        <div className="flex flex-col gap-12">
            <LibraryHero isSignedIn={!!userId} />

            <MyLibrarySection />

            {/* ── Public Catalogue (Gutenberg) ────────────────────────── */}
            <section className="flex flex-col gap-6 pb-16">
                <h2 className="text-2xl font-serif font-bold text-[var(--foreground)]">
                    Explore Public Books
                </h2>
                {gutenbergBooks.length === 0 ? (
                    <div className="flex flex-col items-center justify-center py-12 text-center text-[var(--muted-foreground)] border border-dashed border-[var(--border)] rounded-2xl">
                        <p>No public books available right now.</p>
                    </div>
                ) : (
                    <div className="grid grid-cols-2 min-[480px]:grid-cols-3 md:grid-cols-4 gap-4">
                        {gutenbergBooks.map((book) => (
                            <BookCard
                                key={book._id}
                                title={book.title}
                                author={book.author}
                                coverURL={book.coverURL}
                                slug={book.slug}
                            />
                        ))}
                    </div>
                )}
            </section>
        </div>
    );
}

