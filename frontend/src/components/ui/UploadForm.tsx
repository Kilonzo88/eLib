"use client";

import React, { useRef, useState } from "react";
import { useAuth, useClerk } from "@clerk/nextjs";

import { Button } from "@/components/ui/button";
import Dropzone, { ACCEPTED_IMAGE_TYPES, ACCEPTED_BOOK_TYPES } from "@/components/ui/Dropzone";
import { getBackendUrl } from "@/lib/utils";

const MAX_BOOK_BYTES = 50 * 1024 * 1024; // 50MB

function formatBytes(bytes: number) {
  const mb = bytes / (1024 * 1024);
  if (mb >= 1) return `${mb.toFixed(0)}MB`;
  return `${bytes}B`;
}

const UploadForm = () => {
  const { userId, getToken } = useAuth();
  const clerk = useClerk();

  const [bookFile, setBookFile] = useState<File | null>(null);
  const [coverPreviewB64, setCoverPreviewB64] = useState<string | null>(null);

  const [title, setTitle] = useState("");
  const [authorName, setAuthorName] = useState("");

  const [isSubmitting, setIsSubmitting] = useState(false);
  const [formError, setFormError] = useState<string | null>(null);

  const bookInputRef = useRef<HTMLInputElement | null>(null);

  const onPickBook = async (file: File | null) => {
    setFormError(null);
    if (!file) {
      setBookFile(null);
      setCoverPreviewB64(null);
      return;
    }

    console.log("[onPickBook] File selected:", file.name, "type:", JSON.stringify(file.type), "size:", file.size);

    // Mobile browsers often report empty MIME types — fall back to extension check
    const hasValidMime = ACCEPTED_BOOK_TYPES.includes(file.type);
    const hasValidExt = /\.(pdf|epub)$/i.test(file.name);
    console.log("[onPickBook] hasValidMime:", hasValidMime, "hasValidExt:", hasValidExt);
    if (!hasValidMime && !hasValidExt) {
      setFormError("Please upload a PDF or EPUB file.");
      return;
    }

    if (file.size > MAX_BOOK_BYTES) {
      setFormError(`File must be 50MB or less (selected: ${formatBytes(file.size)}).`);
      return;
    }

    setBookFile(file);
    setCoverPreviewB64(null);
    setAuthorName("");

    // Call our metadata extraction endpoint
    try {
      const formData = new FormData();
      formData.append("file", file);
      
      const token = userId ? await getToken() : null;
      const headers: Record<string, string> = {};
      if (token) {
        headers["Authorization"] = `Bearer ${token}`;
      }

      const res = await fetch(`/api/books/extract-metadata`, {
        method: "POST",
        headers,
        body: formData,
      });

      if (res.ok) {
        const data = await res.json();
        
        // Auto-fill Title (fallback to filename)
        if (data.title && data.title.trim() !== "") {
          setTitle(data.title.trim());
        } else {
          const cleanName = file.name.replace(/\.(pdf|epub)$/i, "").replace(/[-_]/g, " ");
          setTitle(cleanName);
        }

        if (data.author && data.author.trim() !== "") {
          setAuthorName(data.author.trim());
        }

        // Display base64 preview if available
        if (data.cover_b64) {
          setCoverPreviewB64(data.cover_b64);
        }
      } else {
        const cleanName = file.name.replace(/\.(pdf|epub)$/i, "").replace(/[-_]/g, " ");
        setTitle(cleanName);
      }
    } catch (e) {
      console.error("Failed to extract metadata", e);
      const cleanName = file.name.replace(/\.(pdf|epub)$/i, "").replace(/[-_]/g, " ");
      setTitle(cleanName);
    }
  };

  const onSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setFormError(null);

    if (!bookFile) {
      setFormError("Please upload a book first.");
      return;
    }

    const finalTitle = title || bookFile.name;

    setIsSubmitting(true);
    try {
      const token = userId ? await getToken() : null;
      
      // If signed in, check for duplicate. If guest, we skip this check for simplicity
      if (token) {
        const existingRes = await fetch("/api/my-books", {
          headers: { "Authorization": `Bearer ${token}` },
          cache: "no-store",
        });
        if (existingRes.ok) {
          const existingBooks = await existingRes.json();
          if (Array.isArray(existingBooks) && existingBooks.some(b => b.title.trim().toLowerCase() === finalTitle.trim().toLowerCase())) {
            throw new Error("You have already uploaded a book with this title.");
          }
        }
      }

      const formData = new FormData();
      formData.append("file", bookFile);
      formData.append("title", finalTitle);
      if (authorName) formData.append("author", authorName);

      const headers: Record<string, string> = {};
      if (token) {
        headers["Authorization"] = `Bearer ${token}`;
      }
      const res = await fetch(`/api/books`, {
        method: "POST",
        headers,
        body: formData,
      });

      if (!res.ok) {
        let errText = "Failed to upload book";
        try {
          errText = await res.text();
        } catch (_) {}
        throw new Error(errText);
      }

      const data = await res.json();
      const bookId = data._id || data.id;

      if (!userId) {
        // Guest mode: initiated sync. Now prompt for sign-in and claim later
        localStorage.setItem("pending_claim_book_id", bookId);
        clerk.openSignIn({ 
          forceRedirectUrl: window.location.href + (window.location.search ? "&" : "?") + "claim=" + bookId 
        });
        return;
      }

      // alert("Upload Successful! Backend is processing the book.");
      window.location.href = "/";
    } catch (e: any) {
      console.error(e);
      setFormError(e.message || "An error occurred during submission.");
    } finally {
      setIsSubmitting(false);
    }
  };



  return (
    <div className="w-full">
      <p className="text-xs text-[var(--muted-foreground)] mb-3">
        5 of 10 books used (Update)
      </p>

      <form onSubmit={onSubmit} className="flex flex-col gap-5 sm:gap-6 lg:gap-7">
        <div className="grid grid-cols-1 gap-4">
          <div>
            <div className="text-sm font-medium text-[var(--primary)] mb-2">Book File (PDF or EPUB)</div>
            <Dropzone
              kind="pdf"
              title="Click to upload Ebook"
              subtitle="PDF or EPUB file must be 50MB or less"
              file={bookFile}
              onClear={() => {
                setBookFile(null);
                setCoverPreviewB64(null);
              }}
              onPick={onPickBook}
              inputRef={bookInputRef}
              previewB64={coverPreviewB64}
            />
          </div>
        </div>

        <div className="space-y-2 sm:space-y-3">
          <label className="block text-sm font-medium text-[var(--primary)]">
            Title
            <input
              value={title}
              onChange={(e) => setTitle(e.target.value)}
              placeholder="Ex: Rich Dad Poor Dad"
              className="mt-2 w-full h-10 rounded-xl bg-white border border-[rgba(33,42,59,0.12)] px-3 text-sm placeholder:text-[var(--muted-foreground)] outline-none focus:border-[var(--primary)]"
            />
          </label>

          <label className="block text-sm font-medium text-[var(--primary)]">
            Author Name
            <input
              value={authorName}
              onChange={(e) => setAuthorName(e.target.value)}
              placeholder="ex: Robert Kiyosaki"
              className="mt-2 w-full h-10 rounded-xl bg-white border border-[rgba(33,42,59,0.12)] px-3 text-sm placeholder:text-[var(--muted-foreground)] outline-none focus:border-[var(--primary)]"
            />
          </label>
        </div>

        {formError && (
          <div className="text-[12px] text-[var(--destructive)]">{formError}</div>
        )}

        <Button
          type="submit"
          disabled={isSubmitting}
          className="w-full h-12 rounded-xl bg-[var(--primary)] hover:bg-[var(--primary)]/90 text-[var(--primary-foreground)] text-sm font-semibold shadow-sm sm:max-w-[360px]"
        >
          {isSubmitting ? "Uploading Book..." : "Upload Book"}
        </Button>
      </form>
    </div>
  );
};

export default UploadForm;