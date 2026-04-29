# eLib: Interactive AI Mentors for Education

**eLib** is a high-assurance, AI-driven learning platform built specifically to support education in emerging economies, with an initial focus on East Africa. 

The platform allows users to upload static PDFs (such as textbooks) and instantly transforms them into interactive, voice-enabled AI mentors. By leveraging real-time voice SDKs and a highly performant backend, students can have intelligent conversational sessions with their study material.

## 🏗 System Architecture

The project is structured as a decoupled monorepo full-stack application, ensuring both an immersive user experience and a highly concurrent, memory-safe backend infrastructure.

### Frontend
- **Framework:** Next.js 16.2.4 (App Router)
- **UI/UX:** Shadcn UI + Tailwind CSS, employing a "warm literary aesthetic."
- **Real-Time Voice:** Vapi Web SDK integration to handle audio streaming, dynamic turn-taking, and active "thinking/speaking" UI states.
- **State Management:** Custom React Hooks to manage call duration matrices, session IDs, and subscription tier restrictions.

### Backend 
- **Framework:** Rust with Axum
- **Why Rust?** Selected for maximum concurrency, performance under high load, and type-safe CPU-bound tasks like processing large PDFs.
- **RAG Orchestration Workflow:** 
  - The Rust backend acts as the authoritative source of truth for the AI.
  - Exposes an ultra-fast webhook (`POST /api/vapi/search-book`) for the Vapi agent.
  - Fetches the most conditionally relevant context in real-time.
- **Processing Engine:** Ingests PDFs, extracts text, and robustly chunks it into ~500-word logical segments.

### Infrastructure & Data
- **Database:** MongoDB Atlas (accessed via the Rust `mongodb` crate). Heavy reliance on Text Indexes for instant lookups on `Books`, `BookSegments`, and `VoiceSessions`.
- **File Storage:** Vercel Blob handles raw PDF binaries and book cover images.
- **Authentication & Monetization:** Clerk handles identity routing and tier-based billing (Free, Standard, Pro). The backend uses custom Rust middleware to securely verify these Clerk JWTs on every sensitive request strictly at the server level.
- **Observability:** Sentry is integrated across both layers for error tracking and deep session replays.

## 🚀 Setup & Initialization

### Prerequisites
- Node.js (v18+) and `pnpm`
- Rust toolchain (`cargo` & `rustc`)
- MongoDB Atlas cluster URL
- Vercel Blob access tokens
- Clerk Dashboard environment variables
- Vapi secret keys

### 1. Start the Backend (Rust)
```bash
cd backend
cargo run
```

### 2. Start the Frontend (Next.js)
```bash
cd frontend
pnpm install
pnpm dev
```

---

## 📅 Roadmap & Workflows
1. **Ingestion Flow:** User uploads PDF $\rightarrow$ Rust parses/chunks $\rightarrow$ stored in Mongo.
2. **Session Ignition:** Next.js requests `startVoiceSession` $\rightarrow$ Rust verifies limits $\rightarrow$ Issue Session ID.
3. **Conversational Phase:** User Speaks $\rightarrow$ Vapi Transcription $\rightarrow$ Hits Rust Search tool $\rightarrow$ Stream Context $\rightarrow$ Vapi AI speaks.
