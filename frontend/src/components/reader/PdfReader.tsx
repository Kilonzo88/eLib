'use client';

import { useEffect, useRef, useState, useCallback } from 'react';
import * as pdfjsLib from 'pdfjs-dist';

// Configure worker
pdfjsLib.GlobalWorkerOptions.workerSrc = `https://cdn.jsdelivr.net/npm/pdfjs-dist@${pdfjsLib.version}/build/pdf.worker.min.mjs`;

interface PdfReaderProps {
    data: ArrayBuffer;
    onTextSelect?: (text: string, pageNumber: number) => void;
}

export default function PdfReader({ data, onTextSelect }: PdfReaderProps) {
    const containerRef = useRef<HTMLDivElement>(null);
    const pdfDocRef = useRef<pdfjsLib.PDFDocumentProxy | null>(null);
    const renderedPagesRef = useRef<Set<number>>(new Set());
    const observerRef = useRef<IntersectionObserver | null>(null);

    const [totalPages, setTotalPages] = useState(0);
    const [isLoading, setIsLoading] = useState(true);
    const [error, setError] = useState<string | null>(null);
    const [scale, setScale] = useState(1.2);

    // Render a single page into its placeholder div
    const renderPage = useCallback(async (pageNum: number, placeholder: HTMLDivElement) => {
        const pdfDoc = pdfDocRef.current;
        if (!pdfDoc || renderedPagesRef.current.has(pageNum)) return;

        renderedPagesRef.current.add(pageNum);

        try {
            const page = await pdfDoc.getPage(pageNum);
            const viewport = page.getViewport({ scale });

            // Canvas layer
            const canvas = document.createElement('canvas');
            canvas.className = 'pdf-page-canvas';
            canvas.width = viewport.width;
            canvas.height = viewport.height;
            canvas.style.width = '100%';
            canvas.style.height = 'auto';

            const ctx = canvas.getContext('2d')!;
            await page.render({ canvasContext: ctx, viewport, canvas }).promise;

            // Text layer (for selection/highlighting)
            const textContent = await page.getTextContent();
            const textLayerDiv = document.createElement('div');
            textLayerDiv.className = 'pdf-text-layer';
            textLayerDiv.style.cssText = `
                position: absolute; top: 0; left: 0; right: 0; bottom: 0;
                overflow: hidden; opacity: 0.25; line-height: 1.0;
            `;

            const textItems = textContent.items as Array<{
                str: string;
                transform: number[];
                width: number;
                height: number;
            }>;

            for (const item of textItems) {
                if (!item.str.trim()) continue;
                const span = document.createElement('span');
                span.textContent = item.str;

                const tx = item.transform;
                const fontSize = Math.sqrt(tx[0] * tx[0] + tx[1] * tx[1]);
                const x = tx[4];
                const y = viewport.height - tx[5];

                span.style.cssText = `
                    position: absolute;
                    left: ${(x / viewport.width) * 100}%;
                    top: ${((y - fontSize) / viewport.height) * 100}%;
                    font-size: ${(fontSize / viewport.height) * 100}vh;
                    font-family: sans-serif;
                    white-space: pre;
                    pointer-events: all;
                    cursor: text;
                    color: transparent;
                `;
                textLayerDiv.appendChild(span);
            }

            // Clear placeholder and insert rendered content
            placeholder.innerHTML = '';
            placeholder.style.position = 'relative';
            placeholder.appendChild(canvas);
            placeholder.appendChild(textLayerDiv);
        } catch (err) {
            console.error(`[PdfReader] Error rendering page ${pageNum}:`, err);
        }
    }, [scale]);

    // Initialize PDF document from ArrayBuffer
    useEffect(() => {
        if (!data || data.byteLength === 0) return;

        let cancelled = false;

        const initPdf = async () => {
            try {
                // Pass ArrayBuffer directly — no network fetch at all
                const loadingTask = pdfjsLib.getDocument({
                    data: new Uint8Array(data.slice(0)),
                    cMapUrl: `https://cdn.jsdelivr.net/npm/pdfjs-dist@${pdfjsLib.version}/cmaps/`,
                    cMapPacked: true,
                });

                const pdfDoc = await loadingTask.promise;
                if (cancelled) return;

                pdfDocRef.current = pdfDoc;
                setTotalPages(pdfDoc.numPages);
                setIsLoading(false);

                // Create placeholder divs for each page
                const container = containerRef.current;
                if (!container) return;
                container.innerHTML = '';

                const placeholders: HTMLDivElement[] = [];
                for (let i = 1; i <= pdfDoc.numPages; i++) {
                    const pageDiv = document.createElement('div');
                    pageDiv.className = 'pdf-page-placeholder';
                    pageDiv.dataset.page = String(i);
                    pageDiv.style.cssText = `
                        min-height: 400px; 
                        margin-bottom: 12px; 
                        border-radius: 8px;
                        background: var(--muted, #f4f4f5);
                        display: flex;
                        align-items: center;
                        justify-content: center;
                        font-size: 14px;
                        color: var(--muted-foreground, #71717a);
                    `;
                    pageDiv.textContent = `Page ${i}`;
                    container.appendChild(pageDiv);
                    placeholders.push(pageDiv);
                }

                // IntersectionObserver: only render pages when they enter the viewport
                const observer = new IntersectionObserver(
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
                    { rootMargin: '200px', threshold: 0.01 }
                );
                observerRef.current = observer;

                for (const p of placeholders) {
                    observer.observe(p);
                }
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
            if (observerRef.current) {
                observerRef.current.disconnect();
            }
            if (pdfDocRef.current) {
                pdfDocRef.current.destroy();
            }
            renderedPagesRef.current.clear();
        };
    }, [data, renderPage]);

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
        return () => document.removeEventListener('mouseup', handleSelection);
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
        <div className="relative w-full rounded-2xl bg-[var(--card)] border border-[var(--border)] shadow-sm overflow-hidden">
            {isLoading && (
                <div className="absolute inset-0 z-10 flex flex-col items-center justify-center gap-4 bg-[var(--card)]">
                    <div className="w-10 h-10 border-4 border-[var(--primary)] border-t-transparent rounded-full animate-spin" />
                    <p className="text-sm text-[var(--muted-foreground)]">Loading PDF…</p>
                </div>
            )}

            {/* Toolbar */}
            {!isLoading && (
                <div className="sticky top-0 z-20 flex items-center justify-between px-4 py-2 bg-[var(--card)] border-b border-[var(--border)]">
                    <span className="text-xs text-[var(--muted-foreground)]">{totalPages} pages</span>
                    <div className="flex items-center gap-2">
                        <button
                            onClick={() => setScale(s => Math.max(0.5, s - 0.2))}
                            className="px-2 py-1 text-xs rounded bg-[var(--muted)] hover:bg-[var(--accent)] transition-colors"
                        >
                            −
                        </button>
                        <span className="text-xs font-mono w-12 text-center">{Math.round(scale * 100)}%</span>
                        <button
                            onClick={() => setScale(s => Math.min(3, s + 0.2))}
                            className="px-2 py-1 text-xs rounded bg-[var(--muted)] hover:bg-[var(--accent)] transition-colors"
                        >
                            +
                        </button>
                    </div>
                </div>
            )}

            <div
                ref={containerRef}
                className="w-full max-h-[80vh] overflow-y-auto p-4"
                style={{ opacity: isLoading ? 0 : 1, transition: 'opacity 0.3s ease' }}
            />
        </div>
    );
}
