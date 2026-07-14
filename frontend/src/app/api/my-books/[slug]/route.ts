import { auth } from '@clerk/nextjs/server';
import { NextRequest, NextResponse } from 'next/server';

/**
 * GET /api/my-books/:slug
 * Fetches a single user book from the Rust backend by slug.
 */
export async function GET(
    req: NextRequest
) {
    const segments = req.nextUrl.pathname.split('/');
    const slug = segments[segments.length - 1];

    const { getToken, userId } = await auth();

    if (!userId) {
        return NextResponse.json({ error: 'Unauthorized' }, { status: 401 });
    }

    const token = await getToken();
    const apiUrl = process.env.NEXT_PUBLIC_API_URL ?? 'http://127.0.0.1:8081';

    try {
        const res = await fetch(`${apiUrl}/api/books/${slug}`, {
            headers: { Authorization: `Bearer ${token}` },
            cache: 'no-store',
        });

        if (res.status === 404) {
            return NextResponse.json({ error: 'Book not found on backend' }, { status: 404 });
        }

        if (!res.ok) {
            const ext = await res.text();
            return NextResponse.json({ error: `Backend error: ${ext}` }, { status: res.status });
        }

        const book = await res.json();
        return NextResponse.json(book);
    } catch (e: any) {
        return NextResponse.json({ error: e.message }, { status: 500 });
    }
}
