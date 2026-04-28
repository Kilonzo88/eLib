import React from 'react'
import { BookCardProps } from '@/types';
import Image from 'next/image';
import Link from 'next/link';

const BookCard = ({ title, author, coverURL, slug }: BookCardProps) => {
    return (
        <Link href={`/books/${slug}`}>
            <article className="book-card group relative flex flex-col items-center gap-3 transition-all duration-300 hover:-translate-y-2">
                <figure className="book-card-figure relative">
                    <div className="book-card-cover-wrapper relative overflow-hidden rounded-xl shadow-[0_10px_20px_rgba(104,81,65,0.15)] transition-all duration-300 group-hover:shadow-[0_20px_35px_rgba(104,81,65,0.25)]">
                        <Image
                            src={coverURL}
                            alt={title}
                            width={133}
                            height={200}
                            className="book-card-cover h-[200px] w-[133px] object-cover transition-transform duration-500 group-hover:scale-105"
                        />
                        <div className="absolute inset-0 bg-primary/5 group-hover:bg-transparent transition-colors duration-300" />
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