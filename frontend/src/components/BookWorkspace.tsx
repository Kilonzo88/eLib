'use client';

import { useEffect, useRef, useState, type ReactNode } from 'react';
import ResizableSplit from './ResizableSplit';
import PdfReader from './reader/PdfReader';
import EpubReader from './reader/EpubReader';
import BookChat from './chat/BookChat';

/**
 * SSR-safe media query hook.
 * Returns null until the browser has been measured, so we never mount the
 * wrong layout first (which would load the PDF/EPUB twice).
 */
function useMediaQuery(query: string): boolean | null {
    const [matches, setMatches] = useState<boolean | null>(null);
    useEffect(() => {
        const mq = window.matchMedia(query);
        const update = () => setMatches(mq.matches);
        update();
        mq.addEventListener('change', update);
        return () => mq.removeEventListener('change', update);
    }, [query]);
    return matches;
}

interface BookWorkspaceProps {
    slug: string;
    format: 'pdf' | 'epub';
    data: ArrayBuffer;
    /**
     * Optional. Whatever your app already shows for chat on phones
     * (bottom sheet, floating button, etc). Rendered next to the reader
     * below 768px. Not used on desktop.
     */
    renderMobileExtras?: (ctx: {
        selectedText: string | null;
        clearSelection: () => void;
        navigateToPage: (pageNumber: number) => void;
        navigateToChapter: (chapterIndex: number, passage: string) => void;
    }) => ReactNode;
    headerHeight?: number;
}

export default function BookWorkspace({ slug, format, data, renderMobileExtras, headerHeight = 65 }: BookWorkspaceProps) {
    const [selectedText, setSelectedText] = useState<string | null>(null);
    const pdfPageNavigator = useRef<(pageNumber: number) => void>(() => {});
    const epubChapterNavigator = useRef<(chapterIndex: number, passage: string) => void>(() => {});
    const navigateToPage = (pageNumber: number) => pdfPageNavigator.current(pageNumber);
    const navigateToChapter = (chapterIndex: number, passage: string) => epubChapterNavigator.current(chapterIndex, passage);
    // Same breakpoint as the readers' Tailwind `md:` classes
    const isDesktop = useMediaQuery('(min-width: 768px)');

    const handleSelect = (text: string) => setSelectedText(text.trim() || null);
    const clearSelection = () => setSelectedText(null);

    if (isDesktop === null) return null;

    const reader =
        format === 'pdf'
            ? <PdfReader data={data} onTextSelect={handleSelect} onNavigateToPage={(navigate) => { pdfPageNavigator.current = navigate; }} />
            : <EpubReader data={data} onTextSelect={handleSelect} onNavigateToChapter={(navigate) => { epubChapterNavigator.current = navigate; }} />;

    if (!isDesktop) {
        return (
            <>
                {reader}
                {renderMobileExtras?.({ selectedText, clearSelection, navigateToPage, navigateToChapter })}
            </>
        );
    }

    return (
        <ResizableSplit
            headerHeight={headerHeight} // must match the real header height in px
            left={reader}
            right={
                <BookChat
                    slug={slug}
                    selectedText={selectedText}
                    onClearSelection={clearSelection}
                    onCitationPage={navigateToPage}
                    onCitationChapter={format === 'epub' ? navigateToChapter : undefined}
                    className="h-full"
                />
            }
        />
    );
}
