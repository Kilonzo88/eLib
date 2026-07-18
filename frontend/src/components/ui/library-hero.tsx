// components/library-hero.tsx
'use client';

import Image from "next/image";
import Link from "next/link";

interface LibraryHeroProps {
    isSignedIn?: boolean;
}

export function LibraryHero({ isSignedIn = false }: LibraryHeroProps) {
    return (
        <div className="w-full rounded-2xl bg-primary px-5 py-4 md:px-8 md:py-7 flex flex-col md:flex-row items-center gap-6 overflow-hidden shadow-xl border border-primary/20">

            {/* Left — heading + CTA */}
            <div className="w-full md:flex-shrink-0 md:w-[240px] text-center md:text-left">
                <h1 className="library-hero-title text-3xl md:text-4xl font-serif font-bold mb-2 md:mb-4 text-primary-foreground">
                    {isSignedIn ? 'My Library' : 'Your Library'}
                </h1>
                <p className="text-sm text-primary-foreground/80 leading-relaxed mb-4 md:mb-6 mx-auto md:mx-0">
                    Convert your books into interactive conversations.
                    Discuss your favorite reads with AI.
                </p>
                <Link
                    href="/books/new"
                    className="inline-flex items-center gap-2 rounded-lg border border-[oklch(0.880_0.030_76)] bg-background px-4 py-2 text-sm font-medium text-[oklch(0.192_0.025_55)] hover:bg-secondary transition-colors"
                >
                    <span>+</span> Add new book
                </Link>
            </div>

            {/* Center — illustration */}
            <div className="hidden md:block w-full max-w-[320px] h-[180px] md:h-[200px] relative flex-shrink-0 mx-auto">
                <Image
                    src="/assets/hero-illustration.png"
                    alt="Books, globe and lamp illustration"
                    fill
                    className="object-contain drop-shadow-2xl"
                    priority
                />
            </div>

            {/* Right — steps */}
            <div className="hidden md:flex w-full md:flex-shrink-0 md:w-auto flex flex-col gap-3 md:min-w-[180px]">
                {[
                    { n: 1, title: "Upload PDF", sub: "Add your book file" },
                    { n: 2, title: "AI Processing", sub: "We analyze the content" },
                    { n: 3, title: "Voice Chat", sub: "Discuss with AI" },
                ].map(({ n, title, sub }) => (
                    <div
                        key={n}
                        className="flex items-start gap-3 rounded-xl bg-background px-4 py-3 shadow-md transition-transform hover:scale-[1.02]"
                    >
                        <span className="mt-0.5 flex h-5 w-5 flex-shrink-0 items-center justify-center rounded-full border border-[oklch(0.880_0.030_76)] text-xs font-medium text-[oklch(0.510_0.030_62)]">
                            {n}
                        </span>
                        <div>
                            <p className="text-sm font-medium text-[oklch(0.192_0.025_55)]">{title}</p>
                            <p className="text-xs text-[oklch(0.510_0.030_62)]">{sub}</p>
                        </div>
                    </div>
                ))}
            </div>

        </div>
    );
}