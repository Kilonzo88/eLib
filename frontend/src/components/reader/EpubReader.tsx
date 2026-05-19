'use client';

import { useEffect, useRef, useState } from 'react';
import ePub, { Book, Rendition } from 'epubjs';

interface EpubReaderProps {
    data: ArrayBuffer;
    onTextSelect?: (text: string, cfiRange: string) => void;
}

export default function EpubReader({ data, onTextSelect }: EpubReaderProps) {
    const viewerRef = useRef<HTMLDivElement>(null);
    const bookRef = useRef<Book | null>(null);
    const renditionRef = useRef<Rendition | null>(null);
    const [isLoading, setIsLoading] = useState(true);
    const [error, setError] = useState<string | null>(null);

    useEffect(() => {
        if (!viewerRef.current || !data || data.byteLength === 0) return;

        let destroyed = false;

        const initBook = async () => {
            try {
                // Pass ArrayBuffer directly — epub.js unpacks the ZIP in memory,
                // no external network requests whatsoever
                const book = ePub(data);
                bookRef.current = book;

                const rendition = book.renderTo(viewerRef.current!, {
                    width: '100%',
                    height: '100%',
                    spread: 'none',
                    flow: 'scrolled-doc',
                });
                renditionRef.current = rendition;

                // Apply reading-friendly styles
                rendition.themes.default({
                    'body': {
                        'font-family': '"Georgia", "Times New Roman", serif !important',
                        'line-height': '1.8 !important',
                        'color': 'var(--foreground, #1a1a1a) !important',
                        'padding': '0 1rem !important',
                        'max-width': '100% !important',
                    },
                    'p': {
                        'margin-bottom': '1em !important',
                    },
                    'h1, h2, h3, h4': {
                        'margin-top': '1.5em !important',
                        'margin-bottom': '0.5em !important',
                    },
                    'img': {
                        'max-width': '100% !important',
                        'height': 'auto !important',
                    }
                });

                // Text selection handler for AI context
                rendition.on('selected', (cfiRange: string, contents: { window: Window }) => {
                    const selection = contents.window.getSelection();
                    if (selection && onTextSelect) {
                        onTextSelect(selection.toString(), cfiRange);
                    }
                });

                await rendition.display();

                if (!destroyed) {
                    setIsLoading(false);
                }
            } catch (err) {
                if (!destroyed) {
                    console.error('[EpubReader] Load error:', err);
                    setError(err instanceof Error ? err.message : 'Failed to load EPUB');
                    setIsLoading(false);
                }
            }
        };

        initBook();

        return () => {
            destroyed = true;
            if (renditionRef.current) {
                renditionRef.current.destroy();
            }
            if (bookRef.current) {
                bookRef.current.destroy();
            }
        };
    }, [data, onTextSelect]);

    if (error) {
        return (
            <div className="flex flex-col items-center justify-center py-24 text-center rounded-2xl bg-[var(--card)] border border-[var(--border)]">
                <div className="mb-4 text-5xl opacity-20">📖</div>
                <p className="text-lg font-medium text-[var(--destructive)]">Failed to load EPUB</p>
                <p className="mt-2 text-sm text-[var(--muted-foreground)] max-w-md">{error}</p>
            </div>
        );
    }

    return (
        <div className="relative w-full rounded-2xl bg-[var(--card)] border border-[var(--border)] shadow-sm overflow-hidden">
            {isLoading && (
                <div className="absolute inset-0 z-10 flex flex-col items-center justify-center gap-4 bg-[var(--card)]">
                    <div className="w-10 h-10 border-4 border-[var(--primary)] border-t-transparent rounded-full animate-spin" />
                    <p className="text-sm text-[var(--muted-foreground)]">Loading book…</p>
                </div>
            )}
            <div
                ref={viewerRef}
                className="w-full min-h-[75vh] max-h-[85vh] overflow-y-auto"
                style={{ opacity: isLoading ? 0 : 1, transition: 'opacity 0.3s ease' }}
            />
            {/* Navigation Overlay */}
            {!isLoading && (
                <div className="absolute bottom-4 right-4 left-4 flex justify-between pointer-events-none">
                    <button
                        onClick={() => renditionRef.current?.prev()}
                        className="pointer-events-auto bg-[var(--card)] border border-[var(--border)] shadow-md px-4 py-2 rounded-full text-sm font-medium hover:bg-[var(--accent)] transition-colors"
                    >
                        ← Prev
                    </button>
                    <button
                        onClick={() => renditionRef.current?.next()}
                        className="pointer-events-auto bg-[var(--card)] border border-[var(--border)] shadow-md px-4 py-2 rounded-full text-sm font-medium hover:bg-[var(--accent)] transition-colors"
                    >
                        Next →
                    </button>
                </div>
            )}
        </div>
    );
}
