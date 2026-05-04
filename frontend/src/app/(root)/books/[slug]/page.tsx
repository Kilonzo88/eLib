import Image from 'next/image';
import { notFound } from 'next/navigation';
import { GUTENBERG_CACHE_TIME } from '@/lib/constants';
import { BookReaderParams } from '@/types';

export const revalidate = GUTENBERG_CACHE_TIME;

async function fetchBook(id: string) {
  const res = await fetch(`https://gutendex.com/books/${id}`, {
    next: { revalidate: GUTENBERG_CACHE_TIME },
  });
  if (!res.ok) return null;
  return res.json();
}

export default async function BookReaderPage({ params }: BookReaderParams) {
  const { slug } = await params;
  const book = await fetchBook(slug);

  if (!book) notFound();

  const title: string = book.title ?? 'Unknown Title';
  const author: string =
    book.authors && book.authors.length > 0
      ? book.authors[0].name
      : 'Unknown Author';
  const coverURL: string =
    book.formats?.['image/jpeg'] ?? '';

  // Prefer the dedicated HTML reader; fall back to plain text
  const readerURL: string =
    book.formats?.['text/html'] ??
    book.formats?.['text/plain; charset=utf-8'] ??
    book.formats?.['text/plain'] ??
    '';

  return (
    <div className="flex flex-col gap-6 -mt-4">
      {/* ── Book Header ─────────────────────────────── */}
      <header className="flex items-center gap-6 bg-[var(--card)] border border-[var(--border)] rounded-2xl p-5 shadow-sm">
        {coverURL && (
          <Image
            src={coverURL}
            alt={title}
            width={72}
            height={108}
            unoptimized
            className="rounded-lg object-cover shadow-md flex-shrink-0"
          />
        )}
        <div className="flex flex-col gap-1 min-w-0">
          <h1 className="font-serif text-2xl font-bold text-[var(--foreground)] leading-tight line-clamp-2">
            {title}
          </h1>
          <p className="text-sm text-[var(--muted-foreground)]">by {author}</p>
          <p className="text-xs text-[var(--muted-foreground)] mt-1">
            Public domain · Project Gutenberg
          </p>
        </div>
      </header>

      {/* ── Reader ──────────────────────────────────── */}
      {readerURL ? (
        <div className="w-full rounded-2xl overflow-hidden border border-[var(--border)] shadow-sm">
          <iframe
            src={readerURL}
            title={title}
            className="w-full"
            style={{ height: 'calc(100vh - 220px)', minHeight: '500px' }}
          />
        </div>
      ) : (
        <div className="flex flex-col items-center justify-center gap-3 py-24 text-center text-[var(--muted-foreground)]">
          <p className="text-lg font-serif">No readable format available for this book.</p>
          <a
            href={`https://www.gutenberg.org/ebooks/${slug}`}
            target="_blank"
            rel="noopener noreferrer"
            className="text-sm underline hover:text-[var(--primary)] transition-colors"
          >
            View on Project Gutenberg ↗
          </a>
        </div>
      )}
    </div>
  );
}
