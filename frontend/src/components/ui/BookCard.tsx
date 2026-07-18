'use client';

import React, { useState } from 'react'
import { BookCardProps } from '@/types';
import Image from 'next/image';
import Link from 'next/link';

const BookCard = ({ title, author, coverURL, slug, processingStatus }: BookCardProps) => {
    const [imgError, setImgError] = useState(false);

    // If it's a placeholder URL or undefined, treat it as an error/no-cover immediately
    const hasRealCover = coverURL && !coverURL.includes('placehold.co') && !coverURL.includes('1-L.jpg');
    const showPlaceholder = !hasRealCover || imgError;

    // Curated color palettes for the virtual covers
    const palettes = [
        'from-[#2c3e50] to-[#34495e]', // Midnight Blue
        'from-[#4a148c] to-[#7b1fa2]', // Deep Purple
        'from-[#1b5e20] to-[#388e3c]', // Forest Green
        'from-[#b71c1c] to-[#d32f2f]', // Crimson Red
        'from-[#e65100] to-[#f57c00]', // Burnt Orange
        'from-[#01579b] to-[#0288d1]', // Ocean Blue
        'from-[#37474f] to-[#546e7a]', // Slate Grey
        'from-[#880e4f] to-[#c2185b]', // Wine Pink
    ];

    // Simple hash function to consistently pick a color for the same book
    const getPalette = (str: string) => {
        let hash = 0;
        for (let i = 0; i < str.length; i++) {
            hash = str.charCodeAt(i) + ((hash << 5) - hash);
        }
        return palettes[Math.abs(hash) % palettes.length];
    };

    const currentPalette = getPalette(slug || title);

    return (
        <Link href={`/books/${slug}`}>
            <article className="book-card group relative flex flex-col items-center gap-3 transition-all duration-300 hover:-translate-y-2">
                <figure className="book-card-figure relative w-full flex justify-center">
                    <div className="book-card-cover-wrapper relative overflow-hidden rounded-xl shadow-[0_10px_20px_rgba(104,81,65,0.15)] transition-all duration-300 group-hover:shadow-[0_20px_35px_rgba(104,81,65,0.25)] bg-[#f8f4e9] w-[133px] h-[200px] flex-shrink-0">
                        {processingStatus && processingStatus !== "ready" && (
                            <div className="absolute top-2 right-2 z-10 bg-yellow-500/90 text-white text-[9px] font-bold px-1.5 py-0.5 rounded shadow-sm animate-pulse">
                                PROCESSING
                            </div>
                        )}
                        {!showPlaceholder ? (
                            <Image
                                src={coverURL}
                                alt={title}
                                fill
                                unoptimized={true}
                                className="object-cover transition-transform duration-500 group-hover:scale-105"
                                onError={() => setImgError(true)}
                            />
                        ) : (
                            <div className={`absolute inset-0 bg-gradient-to-br ${currentPalette} flex flex-col items-center justify-center p-3 text-center transition-transform duration-500 group-hover:scale-105`}>
                                {/* Spine detail */}
                                <div className="absolute left-0 top-0 bottom-0 w-2 bg-black/20" />
                                <div className="absolute left-2 top-0 bottom-0 w-[1px] bg-white/10" />
                                
                                <h3 className={`text-white/90 font-serif font-bold leading-tight line-clamp-4 mt-2 ${
                                    (!title || title.length > 40) ? 'text-xs' : 'text-sm'
                                }`}>
                                    {title || 'Unknown Title'}
                                </h3>
                                {author && (
                                    <div className="mt-auto pt-2 border-t border-white/20 w-3/4">
                                        <p className="text-white/60 font-sans text-[10px] uppercase tracking-wider line-clamp-2">
                                            {author}
                                        </p>
                                    </div>
                                )}
                            </div>
                        )}
                        <div className="absolute inset-0 bg-primary/5 group-hover:bg-transparent transition-colors duration-300 pointer-events-none" />
                    </div>
                </figure>

                <figcaption className="book-card-meta text-center px-2">
                    <h3 className="book-card-title font-serif font-bold text-[oklch(0.192_0.025_55)] line-clamp-1">
                        {title}
                    </h3>
                    <p className="book-card-author text-xs font-medium text-[oklch(0.510_0.030_62)] mt-0.5">
                        by {author}
                    </p>
                </figcaption>
            </article>
        </Link>
    );
};

export default BookCard;