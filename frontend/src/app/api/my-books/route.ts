import { auth } from '@clerk/nextjs/server';
import { NextResponse } from 'next/server';

/**
 * GET /api/my-books
 * Server-side proxy — attaches the Clerk session JWT and forwards the request
 * to the Rust backend's GET /api/books endpoint, which returns all books for
 * the authenticated user sorted newest first.
 */
export async function GET() {
    const { getToken, userId } = await auth();

    if (!userId) {
        return NextResponse.json({ error: 'Unauthorized' }, { status: 401 });
    }

    const token = await getToken();
    const apiUrl = process.env.NEXT_PUBLIC_API_URL ?? 'http://localhost:8081';

    const res = await fetch(`${apiUrl}/api/books`, {
        headers: {
            Authorization: `Bearer ${token}`,
        },
        // No caching — we always want fresh data
        cache: 'no-store',
    });

    if (!res.ok) {
        const text = await res.text();
        console.error('[my-books] Backend error:', res.status, text);
        return NextResponse.json({ error: 'Failed to fetch books' }, { status: res.status });
    }

    const books = await res.json();
    return NextResponse.json(books);
}
