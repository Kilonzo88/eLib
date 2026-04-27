import React from 'react'
import Link from 'next/link'
import Image from 'next/image'

const Navbar = () => {
    return (
        <header className="w-full fixed top-0 left-0 z-50 bg-[var(--background)] border-b border-[var(--border)]">
            <div className="max-w-7xl mx-auto px-4 h-16 flex justify-between items-center">
                <Link href="/" className="flex items-center">
                    <Image 
                        src="/assets/logo.png" 
                        alt="eLib" 
                        width={42} 
                        height={26} 
                        priority
                    />
                </Link>
            </div>
        </header>
    )
}

export default Navbar