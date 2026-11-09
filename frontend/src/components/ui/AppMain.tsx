'use client';

import { usePathname } from 'next/navigation';

export default function AppMain({ children }: { children: React.ReactNode }) {
    const pathname = usePathname();
    const isBookReader = pathname?.startsWith('/books/') && pathname !== '/books/new';

    if (isBookReader) {
        return (
            <main className="w-full pt-0 md:pt-[65px] pb-0 overflow-hidden">
                {children}
            </main>
        );
    }

    return (
        <main className="max-w-6xl mx-auto px-4 md:px-8 pt-16 md:pt-24 pb-0 md:pb-12">
            {children}
        </main>
    );
}
