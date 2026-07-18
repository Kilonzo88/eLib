'use client';

import React, { useEffect, useState } from 'react';
import { useUser } from '@clerk/nextjs';
import BookCard from '@/components/ui/BookCard';
import Link from 'next/link';

interface UserBook {
    _id?: string;
    title: string;
    author?: string;
    slug: string;
    cover_url?: string;
    processing_status?: string;
}

export function MyLibrarySection() {
    const { isSignedIn } = useUser();
    const [myBooks, setMyBooks] = useState<UserBook[]>([]);
    const [myBooksLoading, setMyBooksLoading] = useState(true);

    useEffect(() => {
        if (!isSignedIn) return;
        setMyBooksLoading(true);
        fetch('/api/my-books')
            .then((r) => r.json())
            .then((data) => setMyBooks(Array.isArray(data) ? data : []))
            .catch(() => setMyBooks([]))
            .finally(() => setMyBooksLoading(false));
    }, [isSignedIn]);

    if (!isSignedIn) return null;

    return (
        <section className="flex flex-col gap-6">
            <div className="flex items-center justify-between">
                <h2 className="text-2xl font-serif font-bold text-[var(--foreground)]">
                    My Library
                </h2>
                <Link
                    href="/books/new"
                    className="text-sm font-medium text-[var(--primary)] hover:opacity-75 transition-opacity"
                >
                    + Add new book
                </Link>
            </div>

            {myBooksLoading ? (
                <div className="grid grid-cols-2 min-[480px]:grid-cols-3 md:grid-cols-4 gap-4">
                    {Array.from({ length: 4 }).map((_, i) => (
                        <div
                            key={i}
                            className="h-[260px] rounded-xl bg-[var(--muted)] animate-pulse"
                        />
                    ))}
                </div>
            ) : myBooks.length === 0 ? (
                <div className="flex flex-col items-center justify-center gap-4 rounded-2xl border border-dashed border-[var(--border)] py-16 text-center">
                    <p className="text-[var(--muted-foreground)] text-sm">
                        You haven&apos;t uploaded any books yet.
                    </p>
                    <Link
                        href="/books/new"
                        className="inline-flex items-center gap-2 rounded-lg bg-[var(--primary)] text-[var(--primary-foreground)] px-5 py-2 text-sm font-medium hover:opacity-90 transition-opacity"
                    >
                        Upload your first book
                    </Link>
                </div>
            ) : (
                <div className="grid grid-cols-2 min-[480px]:grid-cols-3 md:grid-cols-4 gap-4">
                    {myBooks.map((book) => (
                        <BookCard
                            key={book.slug}
                            title={book.title}
                            author={book.author ?? 'Unknown Author'}
                            coverURL={book.cover_url ?? 'https://placehold.co/400x600?text=No+Cover'}
                            slug={book.slug}
                            processingStatus={book.processing_status}
                        />
                    ))}
                </div>
            )}
        </section>
    );
}
