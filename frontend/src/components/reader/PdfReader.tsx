'use client';

import { useEffect, useRef, useState, useCallback } from 'react';
import * as pdfjsLib from 'pdfjs-dist';
import { useRouter } from 'next/navigation';
import { cn } from '@/lib/utils';

// Configure worker
pdfjsLib.GlobalWorkerOptions.workerSrc = `https://cdn.jsdelivr.net/npm/pdfjs-dist@${pdfjsLib.version}/build/pdf.worker.min.mjs`;

interface PdfReaderProps {
    data: ArrayBuffer;
    onTextSelect?: (text: string, pageNumber: number) => void;
}

export default function PdfReader({ data, onTextSelect }: PdfReaderProps) {
    const wrapperRef = useRef<HTMLDivElement>(null);
    const pagesContainerRef = useRef<HTMLDivElement>(null);
    const pdfDocRef = useRef<pdfjsLib.PDFDocumentProxy | null>(null);
    
    const [pdfDoc, setPdfDoc] = useState<pdfjsLib.PDFDocumentProxy | null>(null);
    const renderedPagesRef = useRef<Set<number>>(new Set());
    const observerRef = useRef<IntersectionObserver | null>(null);
    const pageObserverRef = useRef<IntersectionObserver | null>(null);
    const visiblePagesRef = useRef<Map<number, number>>(new Map());

    const [totalPages, setTotalPages] = useState(0);
    const [isLoading, setIsLoading] = useState(true);
    const [error, setError] = useState<string | null>(null);
    
    // Core Layout State
    const [baseWidth, setBaseWidth] = useState<number | null>(null);
    const [userScale, setUserScale] = useState(1.0);
    const [currentPage, setCurrentPage] = useState(1);
    const [showOverlay, setShowOverlay] = useState(false);

    const router = useRouter();
    const overlayTimerRef = useRef<NodeJS.Timeout | null>(null);

    const handleTap = useCallback(() => {
        setShowOverlay(prev => {
            const nextState = !prev;
            if (overlayTimerRef.current) clearTimeout(overlayTimerRef.current);
            
            if (nextState) {
                overlayTimerRef.current = setTimeout(() => {
                    setShowOverlay(false);
                }, 3000);
            }
            return nextState;
        });
    }, []);

    // Clear timer on unmount
    useEffect(() => {
        return () => {
            if (overlayTimerRef.current) clearTimeout(overlayTimerRef.current);
        };
    }, []);

    // Track responsive container width
    useEffect(() => {
        const wrapper = wrapperRef.current;
        if (!wrapper) return;
        
        const resizeObserver = new ResizeObserver((entries) => {
            for (const entry of entries) {
                // On mobile, use 100% width (0 padding). On desktop, leave 48px combined padding.
                const padding = window.innerWidth < 768 ? 0 : 48;
                setBaseWidth(Math.floor(entry.contentRect.width - padding));
            }
        });
        
        resizeObserver.observe(wrapper);
        return () => resizeObserver.disconnect();
    }, []);

    // Render a single page into its placeholder div
    const renderPage = useCallback(async (pageNum: number, placeholder: HTMLDivElement) => {
        const pdfDoc = pdfDocRef.current;
        if (!pdfDoc || !baseWidth) return;

        if (renderedPagesRef.current.has(pageNum)) return;
        renderedPagesRef.current.add(pageNum);

        try {
            const page = await pdfDoc.getPage(pageNum);
            
            // Determine scale to fit baseWidth exactly
            // ── Canvas layer — use dpr for high-resolution rendering
            const dpr = Math.min(window.devicePixelRatio || 1, 3);
            
            // ── Scale: fit the page to the container width 
            const unscaledViewport = page.getViewport({ scale: 1 });
            const fitScale = baseWidth / unscaledViewport.width;
            const finalScale = fitScale * userScale;
            
            // ── DPR: ONE place only — in the viewport scale, nowhere else 
            const viewport = page.getViewport({ scale: finalScale * dpr });

            const canvas = document.createElement('canvas');
            canvas.className = 'pdf-page-canvas block';
            
            // ── Canvas physical buffer = full DPR resolution 
            canvas.width = Math.floor(viewport.width);
            canvas.height = Math.floor(viewport.height);
            
            // ── CSS display size = layout pixels (no DPR here) 
            canvas.style.width = `${Math.floor(viewport.width / dpr)}px`;
            canvas.style.height = `${Math.floor(viewport.height / dpr)}px`;

            // Setup placeholder exact pixel dimensions
            placeholder.style.width = `${Math.floor(viewport.width / dpr)}px`;
            placeholder.style.height = `${Math.floor(viewport.height / dpr)}px`;
            placeholder.innerHTML = ''; // Clear previous loading text

            const ctx = canvas.getContext('2d', { alpha: false, willReadFrequently: false })!;

            await page.render({ 
                canvasContext: ctx, 
                viewport,
                intent: "display",
                // @ts-ignore
                canvas: canvas,
            }).promise;

            placeholder.appendChild(canvas);

            // ── Render Text Layer ──
            const textContent = await page.getTextContent();
            const textLayerDiv = document.createElement('div');
            textLayerDiv.className = 'textLayer block';
            textLayerDiv.style.width = `${Math.floor(viewport.width / dpr)}px`;
            textLayerDiv.style.height = `${Math.floor(viewport.height / dpr)}px`;
            textLayerDiv.style.setProperty('--scale-factor', (viewport.scale / dpr).toString());

            placeholder.appendChild(textLayerDiv);

            const textLayer = new pdfjsLib.TextLayer({
                textContentSource: textContent,
                container: textLayerDiv,
                viewport: page.getViewport({ scale: finalScale }),
            });
            await textLayer.render();
        } catch (err) {
            console.error(`[PdfReader] Error rendering page ${pageNum}:`, err);
        }
    }, [baseWidth, userScale]);

    // Effect 1: Initialize PDF document from ArrayBuffer
    useEffect(() => {
        if (!data || data.byteLength === 0) return;

        let cancelled = false;

        const initPdf = async () => {
            try {
                setIsLoading(true);
                const loadingTask = pdfjsLib.getDocument({
                    data: new Uint8Array(data.slice(0)),
                    cMapUrl: '/cmaps/',
                    cMapPacked: true,
                    standardFontDataUrl: '/standard_fonts/',
                    useSystemFonts: false,
                });

                const doc = await loadingTask.promise;
                if (cancelled) {
                    doc.destroy();
                    return;
                }

                pdfDocRef.current = doc;
                setPdfDoc(doc);
                setTotalPages(doc.numPages);
                setIsLoading(false);
            } catch (err) {
                if (!cancelled) {
                    console.error('[PdfReader] Load error:', err);
                    setError(err instanceof Error ? err.message : 'Failed to load PDF');
                    setIsLoading(false);
                }
            }
        };

        initPdf();

        return () => {
            cancelled = true;
            if (pdfDocRef.current) {
                pdfDocRef.current.destroy();
                pdfDocRef.current = null;
            }
        };
    }, [data]);

    // Effect 2: Setup placeholders & intersection observer
    useEffect(() => {
        if (!pdfDoc || !baseWidth) return;

        // Reset tracking because scale or doc changed, forcing clean re-renders
        renderedPagesRef.current.clear();
        visiblePagesRef.current.clear();
        
        const container = pagesContainerRef.current;
        if (!container) return;
        container.innerHTML = '';

        const placeholders: HTMLDivElement[] = [];
        for (let i = 1; i <= pdfDoc.numPages; i++) {
            const pageDiv = document.createElement('div');
            // Desktop visible shadow, mobile zero shadow to maximize screen real estate
            pageDiv.className = 'pdf-page-placeholder relative bg-white md:shadow-md mb-2 md:mb-6 shrink-0 flex items-center justify-center';
            pageDiv.dataset.page = String(i);
            
            // Standard A4 aspect ratio approximation to prevent layout shift before render
            const approxHeight = baseWidth * 1.414 * userScale;
            
            pageDiv.style.cssText = `
                width: ${baseWidth * userScale}px;
                height: ${approxHeight}px;
                color: var(--muted-foreground, #71717a);
                font-size: 14px;
            `;
            pageDiv.textContent = `Loading Page ${i}...`;
            container.appendChild(pageDiv);
            placeholders.push(pageDiv);
        }

        // Observer for rendering pages ahead of scroll
        const renderObserver = new IntersectionObserver(
            (entries) => {
                for (const entry of entries) {
                    if (entry.isIntersecting) {
                        const pageNum = parseInt((entry.target as HTMLDivElement).dataset.page || '0');
                        if (pageNum > 0) {
                            renderPage(pageNum, entry.target as HTMLDivElement);
                        }
                    }
                }
            },
            { rootMargin: '800px', threshold: 0.01 }
        );
        observerRef.current = renderObserver;

        // Observer for tracking current page visible to the user
        const pageObserver = new IntersectionObserver(
            (entries) => {
                let changed = false;
                entries.forEach(entry => {
                    const pageNum = parseInt((entry.target as HTMLDivElement).dataset.page || '0');
                    if (pageNum > 0) {
                        if (entry.isIntersecting) {
                            visiblePagesRef.current.set(pageNum, entry.intersectionRatio);
                        } else {
                            visiblePagesRef.current.delete(pageNum);
                        }
                        changed = true;
                    }
                });

                if (changed && visiblePagesRef.current.size > 0) {
                    let maxRatio = -1;
                    let bestPage = -1;
                    visiblePagesRef.current.forEach((ratio, page) => {
                        if (ratio > maxRatio) {
                            maxRatio = ratio;
                            bestPage = page;
                        }
                    });
                    if (bestPage > 0) setCurrentPage(bestPage);
                }
            },
            { threshold: [0, 0.25, 0.5, 0.75, 1.0] }
        );
        pageObserverRef.current = pageObserver;

        for (const p of placeholders) {
            renderObserver.observe(p);
            pageObserver.observe(p);
        }

        return () => {
            if (observerRef.current) observerRef.current.disconnect();
            if (pageObserverRef.current) pageObserverRef.current.disconnect();
        };
    }, [pdfDoc, baseWidth, userScale, renderPage]);

    // Listen for text selection
    useEffect(() => {
        const handleSelection = () => {
            const selection = window.getSelection();
            if (selection && selection.toString().trim() && onTextSelect) {
                const anchorNode = selection.anchorNode;
                const pageEl = anchorNode?.parentElement?.closest('.pdf-page-placeholder');
                const pageNum = pageEl ? parseInt(pageEl.getAttribute('data-page') || '0') : 0;
                onTextSelect(selection.toString(), pageNum);
            }
        };

        document.addEventListener('mouseup', handleSelection);
        // Also support touchend for mobile selection
        document.addEventListener('touchend', handleSelection);
        
        return () => {
            document.removeEventListener('mouseup', handleSelection);
            document.removeEventListener('touchend', handleSelection);
        };
    }, [onTextSelect]);

    if (error) {
        return (
            <div className="flex flex-col items-center justify-center py-24 text-center rounded-2xl bg-[var(--card)] border border-[var(--border)]">
                <div className="mb-4 text-5xl opacity-20">📄</div>
                <p className="text-lg font-medium text-[var(--destructive)]">Failed to load PDF</p>
                <p className="mt-2 text-sm text-[var(--muted-foreground)] max-w-md">{error}</p>
            </div>
        );
    }

    return (
        <div className="fixed inset-0 z-50 md:relative md:inset-auto md:z-10 w-full md:rounded-2xl md:bg-[var(--card)] md:border md:border-[var(--border)] md:shadow-sm overflow-hidden flex flex-col h-[100dvh] md:h-[85vh] bg-[#f8f9fa] dark:bg-[#0f172a]">
            <style>{`
                .textLayer {
                    position: absolute;
                    left: 0;
                    top: 0;
                    right: 0;
                    bottom: 0;
                    overflow: hidden;
                    line-height: 1.0;
                }
                .textLayer > span,
                .textLayer > br {
                    color: transparent;
                    position: absolute;
                    white-space: pre;
                    cursor: text;
                    transform-origin: 0% 0%;
                }
                .textLayer ::selection {
                    background-color: rgba(0, 0, 255, 0.3);
                }
            `}</style>
            {isLoading && (
                <div className="absolute inset-0 z-30 flex flex-col items-center justify-center gap-4 bg-[var(--card)]">
                    <div className="w-10 h-10 border-4 border-[var(--primary)] border-t-transparent rounded-full animate-spin" />
                    <p className="text-sm text-[var(--muted-foreground)]">Loading PDF…</p>
                </div>
            )}

            {/* Desktop Toolbar */}
            <div className="hidden md:flex relative z-20 flex-shrink-0 items-center justify-between px-4 py-3 bg-[var(--background)]/90 backdrop-blur-md border-b border-[var(--border)]">
                <span className="text-[10px] font-bold text-[var(--primary)] uppercase tracking-[0.2em]">
                    Page {currentPage} of {totalPages}
                </span>
                <div className="flex items-center gap-2">
                    <button
                        onClick={() => setUserScale(s => Math.max(0.5, s - 0.25))}
                        className="px-3 py-1 text-lg font-bold rounded bg-[var(--muted)] hover:bg-[var(--accent)] transition-colors select-none text-[var(--foreground)]"
                    >
                        −
                    </button>
                    <button
                        onClick={() => setUserScale(s => Math.min(3, s + 0.25))}
                        className="px-3 py-1 text-lg font-bold rounded bg-[var(--muted)] hover:bg-[var(--accent)] transition-colors select-none text-[var(--foreground)]"
                    >
                        +
                    </button>
                </div>
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
                        {currentPage} / {totalPages}
                    </span>
                </div>
                <div className="flex items-center gap-1">
                    <button
                        onClick={(e) => { e.stopPropagation(); setUserScale(s => Math.max(0.5, s - 0.25)); }}
                        className="w-10 h-10 flex items-center justify-center text-xl font-bold rounded-full bg-white/10 hover:bg-white/20 transition-colors z-50 relative cursor-pointer"
                    >
                        −
                    </button>
                    <button
                        onClick={(e) => { e.stopPropagation(); setUserScale(s => Math.min(3, s + 0.25)); }}
                        className="w-10 h-10 flex items-center justify-center text-xl font-bold rounded-full bg-white/10 hover:bg-white/20 transition-colors z-50 relative cursor-pointer"
                    >
                        +
                    </button>
                </div>
            </div>

            {/* Scrollable Document Container */}
            <div 
                ref={wrapperRef} 
                className="flex-1 w-full overflow-auto overscroll-none scroll-smooth relative z-10"
                style={{ opacity: isLoading ? 0 : 1, transition: 'opacity 0.3s ease' }}
                onClick={handleTap}
            >
                <div 
                    ref={pagesContainerRef} 
                    className="flex flex-col items-center py-0 md:py-8 w-max min-w-full" 
                />
            </div>
        </div>
    );
}
