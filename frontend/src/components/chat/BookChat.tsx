'use client';

import { useState, useRef, useEffect } from 'react';
import { useAuth } from '@clerk/nextjs';
import ReactMarkdown from 'react-markdown';
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
        <div className={cn("flex flex-col bg-transparent lg:bg-[var(--card)] lg:shadow-md overflow-hidden", className || "h-[550px] border border-[var(--border)] rounded-2xl")}>
            {/* Header */}
            <div className="flex items-center justify-between px-4 py-2 border-b border-[var(--border)]/30 bg-transparent">
                <div className="flex items-center gap-2 opacity-70">
                    <span className="text-sm">✨</span>
                    <span className="text-[10px] uppercase tracking-wider font-bold text-[var(--foreground)]">Companion active</span>
                </div>
                <button
                    onClick={() => setMessages([])}
                    title="Clear Chat History"
                    className="text-[10px] uppercase font-semibold text-[var(--muted-foreground)] hover:text-[var(--foreground)] transition-colors"
                >
                    Reset
                </button>
            </div>

            {/* Chat Body */}
            <div className="flex-1 overflow-y-auto p-3 space-y-3">
                {messages.length === 0 && (
                    <div className="flex flex-col items-center justify-center h-full text-center p-4 opacity-60">
                        <span className="text-3xl mb-1">📚</span>
                        <p className="text-[11px] font-semibold uppercase tracking-wider text-[var(--foreground)]">Ask about this book</p>
                        <p className="text-[10px] text-[var(--muted-foreground)] mt-1 max-w-[200px]">
                            Highlight words in the viewer to ask specific questions.
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
                            <div className="text-[9px] bg-sky-50 text-sky-800 border border-sky-200 dark:bg-sky-950/20 dark:text-sky-400 dark:border-sky-800 rounded px-1.5 py-0.5 mb-1 italic truncate max-w-full">
                                Context: &quot;{msg.selected_text}&quot;
                            </div>
                        )}
                        <div
                            className={`rounded-2xl px-3 py-2 text-xs leading-snug ${
                                msg.role === 'user'
                                    ? 'bg-[var(--primary)] text-[var(--primary-foreground)]'
                                    : 'bg-[var(--muted)]/50 text-[var(--foreground)] border border-[var(--border)]/50'
                            }`}
                        >
                            {msg.role === 'user' ? (
                                msg.text
                            ) : (
                                <div className="space-y-1.5 [&_strong]:font-semibold [&_p]:my-0 [&_ul]:list-disc [&_ul]:pl-4 [&_ol]:list-decimal [&_ol]:pl-4">
                                    <ReactMarkdown>
                                        {msg.text}
                                    </ReactMarkdown>
                                </div>
                            )}
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
                        Referencing: &quot;{selectedText}&quot;
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
            <div className="p-2 border-t border-[var(--border)]/30 bg-transparent">
                <form onSubmit={handleSubmit} className="relative flex items-center w-full">
                    <input
                        type="text"
                        value={input}
                        onChange={(e) => setInput(e.target.value)}
                        placeholder="Ask a question..."
                        className="w-full bg-[var(--muted)]/30 text-xs px-4 py-2.5 pr-10 border border-[var(--border)]/50 rounded-full focus:outline-none focus:ring-1 focus:ring-[var(--primary)] text-[var(--foreground)] shadow-inner"
                    />
                    <button
                        type="submit"
                        disabled={!input.trim() || loading}
                        className="absolute right-1 w-7 h-7 bg-[var(--primary)] text-[var(--primary-foreground)] hover:opacity-90 disabled:opacity-40 rounded-full flex items-center justify-center transition-opacity"
                    >
                        <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" className="w-3.5 h-3.5"><path d="m22 2-7 20-4-9-9-4Z"/><path d="M22 2 11 13"/></svg>
                    </button>
                </form>
            </div>
        </div>
    );
}
