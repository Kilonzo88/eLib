'use client';

import { useState, useRef, useEffect } from 'react';
import { useAuth } from '@clerk/nextjs';
import { cn } from '@/lib/utils';

interface Message {
    role: 'user' | 'assistant';
    text: string;
    selected_text?: string;
}

interface BookChatProps {
    slug: string;
    selectedText: string | null;
    onClearSelection: () => void;
    className?: string;
}

export default function BookChat({ slug, selectedText, onClearSelection, className }: BookChatProps) {
    const { getToken } = useAuth();
    const [messages, setMessages] = useState<Message[]>([]);
    const [input, setInput] = useState('');
    const [loading, setLoading] = useState(false);
    const [error, setError] = useState<string | null>(null);
    const messagesEndRef = useRef<HTMLDivElement>(null);

    // Auto-scroll to the bottom of the conversation
    const scrollToBottom = () => {
        messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' });
    };

    useEffect(() => {
        scrollToBottom();
    }, [messages, loading]);

    // Handle sending message
    const handleSubmit = async (e: React.FormEvent) => {
        e.preventDefault();
        const trimmedInput = input.trim();
        if (!trimmedInput || loading) return;

        setError(null);
        setInput('');
        setLoading(true);

        const newMsg: Message = {
            role: 'user',
            text: trimmedInput,
            selected_text: selectedText || undefined
        };

        setMessages((prev) => [...prev, newMsg]);

        // Capture current context values to submit
        const activeSelectedText = selectedText;
        onClearSelection(); // Clear selection badge after sending, like Google Docs/Claude

        try {
            const token = await getToken();
            const res = await fetch(`/api/books/${slug}/chat`, {
                method: 'POST',
                headers: {
                    'Content-Type': 'application/json',
                    ...(token ? { Authorization: `Bearer ${token}` } : {})
                },
                body: JSON.stringify({
                    query: trimmedInput,
                    selected_text: activeSelectedText,
                    // Map local message history to expected API history structure
                    history: messages.map(m => ({
                        role: m.role,
                        text: m.text
                    }))
                })
            });

            if (!res.ok) {
                const errText = await res.text().catch(() => `HTTP error ${res.status}`);
                throw new Error(errText);
            }

            const data = await res.json();
            setMessages((prev) => [
                ...prev,
                { role: 'assistant', text: data.response }
            ]);
        } catch (err: unknown) {
            console.error('Chat error:', err);
            setError(err instanceof Error ? err.message : 'Something went wrong. Please check your API configuration.');
            // Remove user message if it failed so they can re-try easily
            setMessages((prev) => prev.slice(0, -1));
            setInput(trimmedInput);
        } finally {
            setLoading(false);
        }
    };

    return (
        <div className={cn("flex flex-col bg-[var(--card)] shadow-md overflow-hidden", className || "h-[550px] border border-[var(--border)] rounded-2xl")}>
            {/* Header */}
            <div className="flex items-center justify-between px-5 py-3 border-b border-[var(--border)] bg-[var(--muted)]/20">
                <div className="flex items-center gap-2">
                    <span className="text-lg">✨</span>
                    <h3 className="font-serif font-bold text-sm text-[var(--foreground)]">Reader Companion</h3>
                </div>
                <button
                    onClick={() => setMessages([])}
                    title="Clear Chat History"
                    className="text-xs text-[var(--muted-foreground)] hover:text-[var(--foreground)] transition-colors"
                >
                    Reset
                </button>
            </div>

            {/* Chat Body */}
            <div className="flex-1 overflow-y-auto p-4 space-y-4">
                {messages.length === 0 && (
                    <div className="flex flex-col items-center justify-center h-full text-center p-6 opacity-60">
                        <span className="text-4xl mb-2">📚</span>
                        <p className="font-serif text-sm font-semibold text-[var(--foreground)]">Ask about this book</p>
                        <p className="text-xs text-[var(--muted-foreground)] mt-1 max-w-[200px]">
                            Highlight any words or paragraphs in the viewer to ask specific questions about them.
                        </p>
                    </div>
                )}

                {messages.map((msg, idx) => (
                    <div
                        key={idx}
                        className={`flex flex-col max-w-[85%] ${
                            msg.role === 'user' ? 'ml-auto items-end' : 'mr-auto items-start'
                        }`}
                    >
                        {msg.selected_text && (
                            <div className="text-[10px] bg-sky-50 text-sky-800 border border-sky-200 dark:bg-sky-950/20 dark:text-sky-400 dark:border-sky-800 rounded px-1.5 py-0.5 mb-1 italic truncate max-w-full">
                                Context: "{msg.selected_text}"
                            </div>
                        )}
                        <div
                            className={`rounded-2xl px-4 py-2.5 text-sm leading-relaxed ${
                                msg.role === 'user'
                                    ? 'bg-[var(--primary)] text-[var(--primary-foreground)]'
                                    : 'bg-[var(--muted)]/50 text-[var(--foreground)] border border-[var(--border)]/50'
                            }`}
                        >
                            {msg.text}
                        </div>
                    </div>
                ))}

                {loading && (
                    <div className="flex mr-auto items-start max-w-[85%]">
                        <div className="bg-[var(--muted)]/50 border border-[var(--border)]/50 rounded-2xl px-4 py-2.5 text-sm flex items-center gap-2 text-[var(--muted-foreground)]">
                            <span className="w-1.5 h-1.5 bg-[var(--muted-foreground)] rounded-full animate-bounce" style={{ animationDelay: '0ms' }} />
                            <span className="w-1.5 h-1.5 bg-[var(--muted-foreground)] rounded-full animate-bounce" style={{ animationDelay: '150ms' }} />
                            <span className="w-1.5 h-1.5 bg-[var(--muted-foreground)] rounded-full animate-bounce" style={{ animationDelay: '300ms' }} />
                        </div>
                    </div>
                )}

                {error && (
                    <div className="p-3 bg-[var(--destructive)]/10 border border-[var(--destructive)]/20 text-[var(--destructive)] text-xs rounded-xl text-center">
                        {error}
                    </div>
                )}
                <div ref={messagesEndRef} />
            </div>

            {/* Selection Context Indicator */}
            {selectedText && (
                <div className="flex items-center justify-between px-3 py-1.5 bg-sky-50 dark:bg-sky-950/20 border-t border-[var(--border)] text-xs text-sky-700 dark:text-sky-400">
                    <span className="truncate italic pr-2">
                        Referencing: "{selectedText}"
                    </span>
                    <button
                        onClick={onClearSelection}
                        className="text-sky-500 hover:text-sky-800 dark:hover:text-sky-200 font-bold px-1"
                    >
                        ✕
                    </button>
                </div>
            )}

            {/* Input Footer */}
            <form onSubmit={handleSubmit} className="p-3 border-t border-[var(--border)] bg-[var(--muted)]/10 flex gap-2">
                <input
                    type="text"
                    value={input}
                    onChange={(e) => setInput(e.target.value)}
                    placeholder="Ask a question about this page..."
                    className="flex-1 bg-[var(--muted)] text-sm px-4 py-2 border border-[var(--border)] rounded-full focus:outline-none focus:ring-1 focus:ring-[var(--primary)] text-[var(--foreground)]"
                />
                <button
                    type="submit"
                    disabled={!input.trim() || loading}
                    className="bg-[var(--primary)] text-[var(--primary-foreground)] hover:opacity-90 disabled:opacity-40 w-9 h-9 rounded-full flex items-center justify-center flex-shrink-0 transition-opacity"
                >
                    ➔
                </button>
            </form>
        </div>
    );
}
