"use client";

import Link from 'next/link'
import Image from 'next/image'
import { usePathname } from 'next/navigation'
import { cn } from '@/lib/utils'
import { Show, SignInButton, SignUpButton, UserButton } from "@clerk/nextjs"

const navItems = [
    { label: "Library", href: "/" },
    { label: "Add New", href: "/books/new" },
]

const Navbar = () => {
    const pathName = usePathname();
    return (
        <header className="w-full fixed top-0 left-0 z-50 bg-[var(--background)] border-b border-[var(--border)]">
            <div className="max-w-7xl mx-auto px-4 h-16 flex justify-between items-center">
                <Link href="/" className="flex items-center">
                    <Image
                        src="/assets/logo.png"
                        alt="eLib" //Google and other search engines are much "happier" when they see a brand name in actual text format inside the header. It helps your site rank higher for the keyword "eLib" than just having an image would
                        width={42}
                        height={26}
                        priority
                    />
                    <span className="logo-text">eLib</span>
                </Link>

                <nav className="flex gap-8 items-center">
                    {navItems.map(({ label, href }) => {
                        const isActive = pathName === href || (href !== '/' && pathName.startsWith(href));

                        return (
                            <Link 
                                key={label} 
                                href={href}
                                className={cn(
                                    'nav-link-base', 
                                    isActive ? 'nav-link-active' : 'text-[var(--muted-foreground)] hover:bg-[var(--accent)]'
                                )}
                            >
                                {label}
                            </Link>
                        );
                    })}
                    
                    <div className="flex gap-4 items-center ml-4 border-l border-[var(--border)] pl-4">
                        <Show when="signed-out">
                            <SignInButton mode="modal">
                                <button className="text-sm font-medium hover:text-[var(--primary)] transition-colors">Sign In</button>
                            </SignInButton>
                            <SignUpButton mode="modal">
                                <button className="text-sm font-medium bg-[var(--primary)] text-[var(--primary-foreground)] px-4 py-2 rounded-full hover:opacity-90 transition-opacity">Get Started</button>
                            </SignUpButton>
                        </Show>
                        <Show when="signed-in">
                            <UserButton />
                        </Show>
                    </div>
                </nav>
            </div>
        </header>
    )
}

export default Navbar;