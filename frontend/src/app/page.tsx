import React from 'react';
import { LibraryHero } from '@/components/ui/library-hero';
import { sampleBooks } from '@/lib/constants';
import BookCard from '@/components/ui/BookCard';

const Page = () => {
  return (
    <div className="flex flex-col gap-12">
      <LibraryHero />

      <section className="max-w-7xl mx-auto px-4 grid grid-cols-2 min-[480px]:grid-cols-3 md:grid-cols-4 gap-4 pb-16">
        {sampleBooks.map((book) => (
          <BookCard
            key={book._id}
            title={book.title}
            author={book.author}
            coverURL={book.coverURL}
            slug={book.slug}
          />
        ))}
      </section>
    </div>
  )
}

export default Page
