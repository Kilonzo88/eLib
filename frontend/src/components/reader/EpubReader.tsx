'use client';

import { useEffect, useRef, useState, useCallback } from 'react';
import ePub, { Book, Rendition } from 'epubjs';
import { useRouter } from 'next/navigation';
import { cn } from '@/lib/utils';

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

    const [currentPage, setCurrentPage] = useState(1);
    const [totalPages, setTotalPages] = useState(0);
    const [showOverlay, setShowOverlay] = useState(false);
    
    const router = useRouter();
    const overlayTimerRef = useRef<NodeJS.Timeout | null>(null);
    const onTextSelectRef = useRef(onTextSelect);

    useEffect(() => {
        onTextSelectRef.current = onTextSelect;
    }, [onTextSelect]);

    const handleTap = useCallback(() => {
        setShowOverlay(prev => {
            const nextState = !prev;
            if (overlayTimerRef.current) clearTimeout(overlayTimerRef.current);
            if (nextState) {
                overlayTimerRef.current = setTimeout(() => setShowOverlay(false), 3000);
            }
            return nextState;
        });
    }, []);

    useEffect(() => {
        return () => {
            if (overlayTimerRef.current) clearTimeout(overlayTimerRef.current);
        };
    }, []);

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
                    manager: 'continuous',
                    flow: 'scrolled',
                });
                renditionRef.current = rendition;

                // Page calculation
                book.ready.then(() => {
                    return book.locations.generate(1600); // 1600 chars per page approx
                }).then((locations) => {
                    if (locations && typeof locations.length === 'number') {
                        setTotalPages(locations.length);
                    }
                }).catch(err => console.error("Error generating locations", err));

                rendition.on('relocated', (location: any) => {
                    if (book.locations.length() > 0) {
                        try {
                            const percent = book.locations.percentageFromCfi(location.start.cfi);
                            const calcPage = Math.abs(Math.round(percent * book.locations.length()));
                            setCurrentPage(calcPage > 0 ? calcPage : 1);
                        } catch (e) {}
                    }
                });

                // Tapping strategy for epub.js
                rendition.on('click', () => handleTap());
                rendition.on('touchstart', () => handleTap());

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
                    if (selection && onTextSelectRef.current) {
                        onTextSelectRef.current(selection.toString(), cfiRange);
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
    }, [data]);

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
        <div className="fixed inset-0 z-50 md:relative md:inset-auto md:z-10 w-full md:rounded-2xl md:bg-[var(--card)] md:border md:border-[var(--border)] md:shadow-sm overflow-hidden flex flex-col h-[100dvh] md:h-[85vh] bg-[#f8f9fa] dark:bg-[#0f172a]">
            {isLoading && (
                <div className="absolute inset-0 z-30 flex flex-col items-center justify-center gap-4 bg-[var(--card)]">
                    <div className="w-10 h-10 border-4 border-[var(--primary)] border-t-transparent rounded-full animate-spin" />
                    <p className="text-sm text-[var(--muted-foreground)]">Loading book…</p>
                </div>
            )}

            {/* Desktop Toolbar */}
            <div className="hidden md:flex relative z-20 flex-shrink-0 items-center justify-between px-4 py-3 bg-[var(--background)]/90 backdrop-blur-md border-b border-[var(--border)]">
                <span className="text-[10px] font-bold text-[var(--primary)] uppercase tracking-[0.2em]">
                    Page {currentPage} {totalPages > 0 ? `of ${totalPages}` : ''}
                </span>
            </div>

            {/* Mobile Overlay Toolbar (tap to show) */}
            <div 
                className={cn(
                    "md:hidden absolute top-0 left-0 right-0 z-40 flex items-center justify-between px-4 py-3 bg-black/70 backdrop-blur-md text-white transition-opacity duration-300",
                    showOverlay ? "opacity-100" : "opacity-0 pointer-events-none"
                )}
            >
                <div className="flex items-center gap-4">
                    <button 
                        onClick={(e) => { e.stopPropagation(); router.back(); }} 
                        className="p-2 -ml-2 text-white hover:opacity-75 relative z-50 cursor-pointer"
                    >
                        <svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"><path d="m15 18-6-6 6-6"/></svg>
                    </button>
                    <span className="text-[10px] font-bold uppercase tracking-[0.2em] mt-[2px]">
                        {currentPage} {totalPages > 0 ? `/ ${totalPages}` : ''}
                    </span>
                </div>
            </div>

            {/* Scrollable Document Container */}
            <div className="flex-1 w-full overflow-hidden relative z-10" onClick={handleTap}>
                <div
                    ref={viewerRef}
                    className="w-full h-full"
                    style={{ opacity: isLoading ? 0 : 1, transition: 'opacity 0.3s ease' }}
                />
            </div>

        </div>
    );
}
