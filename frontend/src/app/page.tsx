import React from 'react';
import { LibraryHero } from '@/components/ui/library-hero';
import { sampleBooks, GUTENBERG_CACHE_TIME, GUTENBERG_BOOK_LIMIT } from '@/lib/constants';
import BookCard from '@/components/ui/BookCard';

export const revalidate = GUTENBERG_CACHE_TIME;

const Page = async () => {
  let displayBooks = sampleBooks;
  try {
    const res = await fetch('https://gutendex.com/books/?page=1', { next: { revalidate: GUTENBERG_CACHE_TIME } });
    const data = await res.json();
    
    if (data && data.results) {
      displayBooks = data.results.slice(0, GUTENBERG_BOOK_LIMIT).map((b: any) => ({
        _id: b.id.toString(),
        title: b.title,
        author: b.authors && b.authors.length > 0 ? b.authors[0].name : 'Unknown Author',
        slug: b.id.toString(),
        coverURL: b.formats['image/jpeg'] || 'https://covers.openlibrary.org/b/id/1-L.jpg',
        coverColor: '#f8f4e9'
      }));
    }
  } catch (error) {
    console.error("Failed to fetch Gutenberg books, falling back to sampleBooks", error);
  }

  return (
    <div className="flex flex-col gap-12">
      <LibraryHero />

      <section className="max-w-7xl mx-auto px-4 grid grid-cols-2 min-[480px]:grid-cols-3 md:grid-cols-4 gap-4 pb-16">
        {displayBooks.map((book) => (
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
