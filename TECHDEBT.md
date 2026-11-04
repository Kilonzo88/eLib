# Technical Debt

This document records concrete risks in the current implementation and a practical order for addressing them. It describes the code as it exists today, not planned features.

## Recommended Order

### 1. Standardize backend URL configuration

Backend URLs are configured through three different variables:

- `BACKEND_URL` is used by the Next.js rewrite in `frontend/next.config.ts`.
- `API_URL` is used by the server-rendered home page in `frontend/src/app/page.tsx`.
- `NEXT_PUBLIC_API_URL` is used by the `my-books` Next.js route handlers and the frontend URL helper.

In Docker Compose, `BACKEND_URL` points to `http://backend:8081`, while `NEXT_PUBLIC_API_URL` points to `http://127.0.0.1:8081`. A server-side request from the frontend container to `127.0.0.1` targets the frontend container itself, not the backend. The public-prefixed variable is also exposed to browser code, so it should not be the source of truth for server-to-server traffic.

**Suggested direction:** use one server-only `BACKEND_URL` for all Next.js server-side requests and rewrites. Keep browser requests same-origin using `/api/...` and let the rewrite forward them. Validate required configuration at startup rather than silently falling back to a potentially incorrect address.

### 2. Make book processing recoverable

Book upload inserts a MongoDB record with `processing_status: "processing"`, then starts processing with `tokio::spawn`. If the task fails, it logs the error but does not persist a failure state. The book can remain stuck in `processing`. Embedding errors are handled as non-fatal: processing continues with missing embeddings and can still mark the book `ready`, even though chat retrieval depends on embeddings.

`book_service::ingest_book_atomic` exists but is not used by the upload flow. It only covers MongoDB inserts; it does not make the external R2 upload and Gemini calls transactional.

**Suggested direction:** persist a terminal `failed` status and a safe error summary, make processing idempotent, and add a retry mechanism. Only report `ready` when all data required by the reader and chat is available. If processing volume warrants it, move the task to a durable worker or job queue; a queue is not required as the first step.

### 3. Provision and document the actual search index

Chat uses MongoDB Atlas `$vectorSearch` with an index named `vector_index` over `book_segments.embedding`. Backend startup creates a regular text index on `content`, but does not create or verify the Atlas vector index. The vector index must therefore be configured separately in Atlas for chat retrieval to work.

**Suggested direction:** document the required Atlas vector index definition and add a startup/readiness check that clearly reports when the index is missing. Remove or clarify the regular text index if it is not used by an endpoint.

### 4. Add verifiable citations to chat answers

Chat retrieves up to four `BookSegment` records, but currently passes only their text to Gemini and returns only a response string. It discards the retrieved segment metadata, and the UI renders the answer as Markdown without source references. The stored `page_number` is currently set to `0` during ingestion, so reliable page citations are not available either.

**Suggested direction:** preserve source location metadata during extraction (such as PDF page or EPUB chapter/section), include stable source identifiers with the retrieved context, and return citations as structured response data for the UI to render. Treat model-produced citation labels as untrusted: validate that every citation refers to a segment actually retrieved for that answer. Until page/section locations are captured, citations should identify a chunk rather than imply a page number.

### 5. Clarify the Node.js and Rust boundary

The primary book and chat API is Rust/Axum. The frontend is still a Node.js server when run with `next start`: Next.js performs server rendering and has route handlers at `frontend/src/app/api/my-books/route.ts` and `frontend/src/app/api/my-books/[slug]/route.ts`. Those handlers authenticate with Clerk and call the Rust API. Next.js rewrites also proxy browser `/api/...` requests to Axum.

This is a reasonable small BFF (backend-for-frontend) layer, but the boundary should be intentional. If the goal is to keep business logic in Rust, keep Next handlers limited to web-specific auth/proxy behavior. Removing Node.js entirely would also require changing how the Next.js app is served; moving API handlers alone will not remove the Next server used for SSR.

### 6. Align documentation and quota behavior

The README describes Vapi voice workflows, subscription tiers, and text-search behavior that are not represented by the registered Axum routes. The current chat path uses Gemini embeddings and Atlas Vector Search; the Gemini chat request uses `gemini-2.5-flash`. `VoiceSession` and its index exist, but the router does not expose a voice-session API.

Quota behavior is also inconsistent: the backend upload quota check is commented out, its helper uses a fixed development limit of 100 books, and the upload UI displays a hard-coded “5 of 10 books used” message.

**Suggested direction:** either implement and test the planned voice/billing features or label them clearly as planned. Update the README to match shipped behavior, and make quota values come from one authoritative backend policy before presenting them as enforced limits.

### 7. Tighten CORS for each deployment

The Axum API currently allows any origin. The Next.js rewrite makes ordinary browser requests same-origin with the frontend, so those requests do not need cross-origin browser access to the Rust server. Direct browser-to-backend requests, including a future Tauri setup, do need a policy appropriate to the origins those builds actually send.

**Suggested direction:** keep development configuration convenient, but explicitly configure production origins and only the methods and request headers the client needs. Verify the actual Tauri `Origin` values per platform instead of assuming one universal origin.

## Lower-Priority Cleanup

R2 client setup and credential loading are repeated across several storage functions. Extracting a shared client/config constructor would reduce duplication and make endpoint, region, and credential changes safer. This is lower priority than fixing URL routing and processing reliability.

## Verification notes (2026-11-04)

Checked against the working tree at `6b601dc` while setting up local tooling. These sharpen the items above; none of them change the recommended order.

- **Item 3 (search index).** The query stage is at `backend/src/routes/chat.rs:74`: index `vector_index`, path `embedding`, `numCandidates: 100`, `limit: 4`. The embedding dimension is **768** (`backend/src/services/gemini_service.rs:104` and `:141`, `output_dimensionality: 768`; `backend/src/models/book_segment.rs:20`). The provisioning documentation this item asks for must state `numDimensions: 768`, because an Atlas index created with a different dimension is accepted at creation time and fails only at query time — which surfaces as an empty result set rather than an error.
- **Item 4 (citations).** Confirmed: retrieval is capped at 4 segments (`chat.rs:79`), and the retrieved documents are reduced to text before the Gemini call, so no source identifiers survive to the response.
- **Item 6 (docs and quota).** Confirmed the upload quota check is commented out (`backend/src/routes/book.rs:90`). Additional drift found: the doc comment at `backend/src/services/gemini_service.rs:161` says `gemini-3.5-flash`, while the request at `:170` targets `gemini-2.5-flash`. The embedding model is `gemini-embedding-001` (`:95`, `:100`, `:126`, `:137`), which the README does not name.
- **New: this ledger is not versioned.** `TECHDEBT.md` is untracked (`git status` reports `?? TECHDEBT.md`) and is not ignored. Until it is committed, its history and attribution are lost with the working tree.

## Environment and tooling debt

These are not application defects. They are risks in the machine and toolchain the app is developed on, recorded here because they determine how items 3, 4 and 6 can be verified.

### E1. Local AI configuration lives outside the repo

The local model route is a hand-edited YAML patch in a harness profile (`~/.dsh/profiles/web/cordis.patch.yml`), not a file in this repository, and that file has already been reformatted once by a settings surface. Nothing in the repo records the provider route, the model tags, their context budgets, or the sandbox grant that makes a notes vault writable. The setup therefore cannot be reproduced from the repo, and a reformat can change behaviour silently.

**Suggested direction:** add a `docs/local-ai.md` describing the route, the model tags and their context budgets, and the manual step in E2.

### E2. One manual step is required and undocumented

`OLLAMA_ORIGINS` is unset, so Ollama rejects requests whose origin is `app://obsidian.md`. The fix is a systemd drop-in (`sudo systemctl edit ollama`), which cannot be performed from an agent session and is recorded nowhere.

### E3. The machine's ceiling is memory, not disk

Measured with `gemma4:26b-a4b-it-qat` (15 GB resident on a shared-memory iGPU, 72% GPU / 28% CPU) and an editor open: 22 GiB used of 23 GiB, **915 MiB available**, 16 GiB shared. Short prompts reached first token in 3.5 s; a 3,881-token prompt took **196 s** to first token at 19.8 tok/s prefill. Any plan that assumes a local model serves long-context requests while the editor runs will swap.

### E4. Local embeddings match this app's dimensions but are not interchangeable

A local embedding model (`nomic-embed-text`, 768 dimensions) matches the 768 dimensions this app already produces (`gemini_service.rs:104`), so the shape of the Atlas index would not change if embeddings moved local. The vector space still differs: existing vectors cannot be reused, and every segment would need re-embedding. Measured on this machine: 4.6 chunks/s, roughly 3.6 minutes per 1,000-chunk book. If item 3 is ever addressed by moving embeddings local, this figure belongs in that plan.

## Tauri packaging (planned: Linux, Windows, macOS, Android)

The Rust API survives unchanged; the Node layer does not. This is a frontend migration with a handful of backend deltas. The full plan, including phases and platform matrix, lives in the notes vault: `01_Projects/eLib/Tauri-Shipping`.

Blockers, in order:

1. **The frontend is a server today.** Eleven of 24 sources import `next/*` (`navigation` ×5, `link` ×4, `image` ×4, `server` ×2, `font` ×1, `dynamic` ×1). `app/page.tsx` is an async server component calling Clerk `auth()`, `app/api/my-books/*` is a Clerk BFF, and `next.config.ts` rewrites `/api/*` to Axum. None of that exists in a webview. Port to a Vite SPA — `output: 'export'` is not a shortcut (route handlers unsupported, no request-time auth, `[slug]` would need `generateStaticParams`).
2. **CORS becomes load-bearing** (see item 7). `main.rs:109-112` allows any origin. The webview origin differs per platform and by `useHttpsScheme`, so it must be read from `window.location.origin` per target rather than assumed.
3. **The PDF worker is fetched from a CDN.** `frontend/src/components/reader/PdfReader.tsx:9` points `workerSrc` at jsdelivr; offline or under CSP the reader fails. Bundle `pdf.worker.min.mjs` from `pdfjs-dist` locally.
4. **Chat is non-streaming.** `chat.rs:131` returns one JSON blob at the end of the Gemini call — a frozen screen on mobile. SSE fixes it.
5. **Item 2 becomes a release blocker on mobile.** Handsets suspend background apps, so a missing `failed` state stops being invisible debt: an uploaded book sits in `processing` forever with no recovery path.
6. **JWT validation is the only trust boundary on a public client.** `middleware/auth.rs:48-55` checks `exp` only, and `OptionalUser` turns an invalid token into a guest. Public binaries make this High rather than Medium.
7. **No secret may enter the bundle.** A Tauri artifact is extractable: Gemini, R2 and MongoDB credentials stay server-side, the same lesson as the committed Clerk test keys.
8. **Release matrix.** macOS builds require a macOS runner, Windows a Windows runner, Android an SDK/NDK; notarization and store signing are separate work items.

Suggested order: origin allowlist → SPA port → bundled PDF worker → item 2 → Android.

