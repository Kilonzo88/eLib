// components/library-hero.tsx
import Image from "next/image";

export function LibraryHero() {
    return (
        <div className="w-full rounded-2xl bg-[oklch(0.924_0.028_78)] px-5 py-6 md:px-8 md:py-7 flex flex-col md:flex-row items-center gap-6 overflow-hidden">

            {/* Left — heading + CTA */}
            <div className="w-full md:flex-shrink-0 md:w-[240px] text-center md:text-left">
                <h1 className="library-hero-title text-3xl md:text-4xl font-serif font-bold mb-4">
                    Your Library
                </h1>
                <p className="text-sm text-[oklch(0.510_0.030_62)] leading-relaxed mb-6 mx-auto md:mx-0">
                    Convert your books into interactive AI conversations.
                    Listen, learn, and discuss your favorite reads.
                </p>
                <button className="inline-flex items-center gap-2 rounded-lg border border-[oklch(0.880_0.030_76)] bg-background px-4 py-2 text-sm font-medium text-[oklch(0.192_0.025_55)] hover:bg-secondary transition-colors shadow-[0_0_12px_rgba(255,253,208,0.8)]">
                    <span>+</span> Add new book
                </button>
            </div>

            {/* Center — illustration */}
            <div className="w-full max-w-[320px] h-[180px] md:h-[200px] relative flex-shrink-0 mx-auto">
                <Image
                    src="/assets/hero-illustration.png"
                    alt="Books, globe and lamp illustration"
                    fill
                    className="object-contain"
                    priority
                />
            </div>

            {/* Right — steps */}
            <div className="w-full md:flex-shrink-0 md:w-auto flex flex-col gap-3 md:min-w-[180px]">
                {[
                    { n: 1, title: "Upload PDF", sub: "Add your book file" },
                    { n: 2, title: "AI Processing", sub: "We analyze the content" },
                    { n: 3, title: "Voice Chat", sub: "Discuss with AI" },
                ].map(({ n, title, sub }) => (
                    <div
                        key={n}
                        className="flex items-start gap-3 rounded-xl bg-background/70 px-4 py-3 shadow-[0_0_12px_rgba(255,253,208,0.8)]"
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