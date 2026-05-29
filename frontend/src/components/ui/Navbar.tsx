"use client";

import Link from 'next/link'
import Image from 'next/image'
import { usePathname } from 'next/navigation'
import { cn } from '@/lib/utils'
import { Show, SignInButton, SignUpButton, UserButton, useUser } from "@clerk/nextjs"

const Navbar = () => {
    const pathName = usePathname();
    const { user, isSignedIn } = useUser();

    // Static nav items — Library label changes based on auth state
    const navItems = [
        { label: isSignedIn ? 'My Library' : 'Your Library', href: '/' },
        { label: 'Add New', href: '/books/new' },
    ];

    // Hide navbar on mobile when reading a book to maximize space
    const isBookReader = pathName.startsWith('/books/');

    return (
        <header className={cn(
            "w-full fixed top-0 left-0 z-50 bg-[var(--background)] border-b border-[var(--border)]",
            isBookReader && "hidden md:block" // Hide completely on mobile for reader pages
        )}>
            <div className="max-w-7xl mx-auto px-4 h-16 flex justify-between items-center">
                <Link href="/" className="flex items-center">
                    <Image
                        src="/assets/logo.png"
                        alt="eLib"
                        width={42}
                        height={26}
                        priority
                    />
                    <span className="logo-text">eLib</span>
                </Link>

                <nav className="hidden md:flex gap-8 items-center">
                    {navItems.map(({ label, href }) => {
                        const isActive = pathName === href || (href !== '/' && pathName.startsWith(href));

                        return (
                            <Link
                                key={href}
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

                    <div className="flex gap-4 items-center md:ml-4 md:border-l border-[var(--border)] md:pl-4">
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
                            {user?.firstName && (
                                <span className="nav-user-name cursor-default select-none">
                                    {user.firstName}
                                </span>
                            )}
                        </Show>
                    </div>
                </nav>

                {/* Mobile: auth only */}
                <div className="flex md:hidden gap-3 items-center">
                    <Show when="signed-out">
                        <SignInButton mode="modal">
                            <button className="text-sm font-medium hover:text-[var(--primary)] transition-colors">Sign In</button>
                        </SignInButton>
                    </Show>
                    <Show when="signed-in">
                        <UserButton />
                    </Show>
                </div>
            </div>
        </header>
    )
}

export default Navbar;