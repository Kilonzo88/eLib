'use client';

import { useState, useEffect, useRef } from 'react';
import { cn } from '@/lib/utils';

interface DraggableAiButtonProps {
    onClick: () => void;
}

export default function DraggableAiButton({ onClick }: DraggableAiButtonProps) {
    const [position, setPosition] = useState({ x: 0, y: 0 }); // Bottom right by default
    const [isDragging, setIsDragging] = useState(false);
    const [isMounted, setIsMounted] = useState(false);
    
    // Set initial position on mount (bottom right corner)
    useEffect(() => {
        setIsMounted(true);
        setPosition({ 
            x: window.innerWidth - 80, // 24px padding from right (button width ~ 56px)
            y: window.innerHeight - 100 // padding from bottom
        });
        
        // Update limits on resize
        const handleResize = () => {
            setPosition(prev => ({
                x: Math.min(prev.x, window.innerWidth - 64),
                y: Math.min(prev.y, window.innerHeight - 64)
            }));
        };
        window.addEventListener('resize', handleResize);
        return () => window.removeEventListener('resize', handleResize);
    }, []);

    // Pointer events
    const buttonRef = useRef<HTMLButtonElement>(null);
    const startPos = useRef({ x: 0, y: 0 });
    const dragTimeout = useRef<NodeJS.Timeout | null>(null);
    const hasDragged = useRef(false);

    const handlePointerDown = (e: React.PointerEvent) => {
        // Only trigger on left click or touch
        if (e.button !== 0 && e.nativeEvent.type !== 'touchstart') return;
        
        e.currentTarget.setPointerCapture(e.pointerId);
        startPos.current = { x: e.clientX, y: e.clientY };
        hasDragged.current = false;
        
        // If held for 200ms, start dragging mode
        dragTimeout.current = setTimeout(() => {
            setIsDragging(true);
        }, 200);
    };

    const handlePointerMove = (e: React.PointerEvent) => {
        if (!isDragging) {
            // Check if user moved to cancel long-press threshold
            const dx = Math.abs(e.clientX - startPos.current.x);
            const dy = Math.abs(e.clientY - startPos.current.y);
            if (dx > 5 || dy > 5) {
                if (dragTimeout.current) clearTimeout(dragTimeout.current);
            }
            return;
        }

        hasDragged.current = true;
        
        // Center button on pointer
        const padding = 24;
        const buttonRadius = 28; // width 14 (56px) / 2
        
        const newX = Math.min(Math.max(padding, e.clientX - buttonRadius), window.innerWidth - padding - buttonRadius * 2);
        const newY = Math.min(Math.max(padding, e.clientY - buttonRadius), window.innerHeight - padding - buttonRadius * 2);
        
        setPosition({ x: newX, y: newY });
    };

    const handlePointerUp = (e: React.PointerEvent) => {
        e.currentTarget.releasePointerCapture(e.pointerId);
        if (dragTimeout.current) clearTimeout(dragTimeout.current);
        
        if (isDragging) {
            setIsDragging(false);
        } else if (!hasDragged.current) {
            onClick();
        }
    };

    if (!isMounted) return null; // Wait for hydration and layout

    return (
        <button 
            ref={buttonRef}
            onPointerDown={handlePointerDown}
            onPointerMove={handlePointerMove}
            onPointerUp={handlePointerUp}
            onPointerCancel={handlePointerUp}
            style={{ 
                transform: `translate(${position.x}px, ${position.y}px)`,
                touchAction: 'none' // Prevent scrolling while dragging
            }}
            className={cn(
                "lg:hidden fixed top-0 left-0 z-[60] text-[var(--primary-foreground)] rounded-full w-14 h-14 flex items-center justify-center flex-shrink-0 touch-none",
                isDragging 
                    ? "scale-110 shadow-2xl ring-4 ring-[var(--primary)]/30 cursor-grabbing bg-[oklch(from_var(--primary)_l_c_h)] opacity-95 transition-none"
                    : "shadow-xl hover:scale-105 active:scale-95 transition-transform duration-200 bg-[var(--primary)]"
            )}
            aria-label="Open AI Companion"
        >
            <span className="font-bold text-lg leading-none select-none">AI</span>
        </button>
    );
}
