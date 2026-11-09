'use client';

import {
    useCallback,
    useEffect,
    useRef,
    useState,
    type KeyboardEvent,
    type PointerEvent,
    type ReactNode,
} from 'react';
import { cn } from '@/lib/utils';

/**
 * Full-viewport reader + companion layout inside ONE bordered surface,
 * with a draggable divider between the panes.
 *
 * The companion has a pixel width (not a percentage), so on a laptop the book
 * isn't squeezed and on a big monitor the extra space goes to the book.
 */

const STORAGE_KEY = 'elib:companion-width';
const DEFAULT_RIGHT = 420; // comfortable chat width
const MIN_LEFT = 420;      // reader never narrower than this
const MIN_RIGHT = 300;     // companion never narrower than this

interface ResizableSplitProps {
    left: ReactNode;
    right: ReactNode;
    /** Height of the app header above this component, in px */
    headerHeight?: number;
}

export default function ResizableSplit({ left, right, headerHeight = 65 }: ResizableSplitProps) {
    const wrapRef = useRef<HTMLDivElement>(null);
    const [rightPx, setRightPx] = useState(DEFAULT_RIGHT);
    const [dragging, setDragging] = useState(false);

    const getWidth = useCallback(
        () => wrapRef.current?.getBoundingClientRect().width || 1200,
        []
    );

    const clamp = useCallback(
        (px: number) => {
            const max = Math.max(MIN_RIGHT, getWidth() - MIN_LEFT);
            return Math.min(Math.max(px, MIN_RIGHT), max);
        },
        [getWidth]
    );

    const persist = (px: number) => {
        try { localStorage.setItem(STORAGE_KEY, String(Math.round(px))); } catch {}
    };

    // Set + remember (used for clicks and keyboard, not for every drag frame)
    const commit = (px: number) => {
        const v = clamp(px);
        setRightPx(v);
        persist(v);
    };

    // Restore the saved width after mount (avoids a hydration mismatch in Next.js)
    useEffect(() => {
        try {
            const saved = parseFloat(localStorage.getItem(STORAGE_KEY) ?? '');
            // eslint-disable-next-line react-hooks/set-state-in-effect
            if (Number.isFinite(saved)) setRightPx(clamp(saved));
        } catch {}
    }, [clamp]);

    // Keep the split valid when the window is resized
    useEffect(() => {
        const onResize = () => setRightPx((p) => clamp(p));
        window.addEventListener('resize', onResize);
        return () => window.removeEventListener('resize', onResize);
    }, [clamp]);

    const onPointerDown = (e: PointerEvent<HTMLDivElement>) => {
        e.currentTarget.setPointerCapture(e.pointerId);
        setDragging(true);
    };
    const onPointerMove = (e: PointerEvent<HTMLDivElement>) => {
        if (!dragging || !wrapRef.current) return;
        const rect = wrapRef.current.getBoundingClientRect();
        setRightPx(clamp(rect.right - e.clientX));
    };
    const onPointerUp = (e: PointerEvent<HTMLDivElement>) => {
        e.currentTarget.releasePointerCapture(e.pointerId);
        setDragging(false);
        persist(rightPx);
    };
    const onKeyDown = (e: KeyboardEvent<HTMLDivElement>) => {
        const step = e.shiftKey ? 80 : 20;
        if (e.key === 'ArrowLeft') commit(rightPx + step);        // companion grows
        else if (e.key === 'ArrowRight') commit(rightPx - step);  // reader grows
        else if (e.key === 'Home') commit(getWidth());
        else if (e.key === 'End') commit(0);
        else if (e.key === 'Enter') commit(DEFAULT_RIGHT);
    };

    const presets = [
        { label: 'Reader', title: 'Focus on the book', px: () => MIN_RIGHT },
        { label: 'Balanced', title: 'Book and companion side by side', px: () => DEFAULT_RIGHT },
        { label: 'Companion', title: 'Focus on the companion', px: () => getWidth() - MIN_LEFT },
    ];

    return (
        <div
            className="flex min-h-0 flex-col px-4 pb-4 pt-2"
            style={{ height: `calc(100dvh - ${headerHeight}px)` }}
        >
            {/* Layout presets */}
            <div
                role="toolbar"
                aria-label="Layout"
                className="mb-2 inline-flex self-end gap-0.5 rounded-xl bg-[var(--muted)] p-[3px]"
            >
                {/* eslint-disable-next-line react-hooks/refs */}
                {presets.map((p) => {
                    const active = Math.abs(clamp(p.px()) - rightPx) < 8;
                    return (
                        <button
                            key={p.label}
                            type="button"
                            title={p.title}
                            aria-pressed={active}
                            onClick={() => commit(p.px())}
                            className={cn(
                                'rounded-lg px-3 py-1 text-[13px] transition-colors',
                                'focus-visible:outline focus-visible:outline-2 focus-visible:outline-[var(--primary)]',
                                active
                                    ? 'bg-[var(--background)] text-[var(--foreground)] shadow-sm'
                                    : 'text-[var(--muted-foreground)] hover:text-[var(--foreground)]'
                            )}
                        >
                            {p.label}
                        </button>
                    );
                })}
            </div>

            {/* One surface, two panes */}
            <div
                ref={wrapRef}
                className={cn(
                    'flex min-h-0 flex-1 items-stretch overflow-hidden rounded-2xl',
                    'border border-[var(--border)] bg-[var(--card)]',
                    dragging && 'cursor-col-resize'
                )}
            >
                <section
                    aria-label="Reader"
                    className={cn(
                        'flex min-h-0 min-w-0 flex-1 flex-col overflow-hidden [&>*]:min-h-0 [&>*]:flex-1',
                        dragging && 'pointer-events-none select-none' // iframes/canvas must not steal the drag
                    )}
                >
                    {left}
                </section>

                <div
                    role="separator"
                    aria-orientation="vertical"
                    aria-label="Resize reader and companion"
                    aria-valuenow={Math.round(rightPx)}
                    aria-valuemin={MIN_RIGHT}
                    tabIndex={0}
                    title="Drag to resize · double-click to reset"
                    onPointerDown={onPointerDown}
                    onPointerMove={onPointerMove}
                    onPointerUp={onPointerUp}
                    onKeyDown={onKeyDown}
                    onDoubleClick={() => commit(DEFAULT_RIGHT)}
                    className={cn(
                        'group relative flex w-2.5 flex-none cursor-col-resize touch-none select-none items-center justify-center',
                        'border-x border-[var(--border)] bg-[var(--muted)] transition-colors hover:bg-[var(--accent)]',
                        'focus-visible:outline focus-visible:outline-2 focus-visible:-outline-offset-2 focus-visible:outline-[var(--primary)]',
                        dragging && 'bg-[var(--accent)]'
                    )}
                >
                    <span
                        className={cn(
                            'h-10 w-[3px] rounded-full bg-[var(--muted-foreground)]/50 transition-all',
                            'group-hover:h-16 group-hover:bg-[var(--primary)] group-focus-visible:h-16 group-focus-visible:bg-[var(--primary)]',
                            dragging && 'h-16 bg-[var(--primary)]'
                        )}
                    />
                </div>

                <section
                    aria-label="Companion"
                    style={{ flexBasis: rightPx }}
                    className={cn(
                        'flex min-h-0 min-w-0 flex-none flex-col overflow-hidden [&>*]:min-h-0 [&>*]:flex-1',
                        dragging && 'pointer-events-none select-none'
                    )}
                >
                    {right}
                </section>
            </div>
        </div>
    );
}
