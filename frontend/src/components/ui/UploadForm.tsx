"use client";

import React, { useMemo, useRef, useState } from "react";
import { useAuth, useClerk } from "@clerk/nextjs";

import { Button } from "@/components/ui/button";
import Dropzone, { ACCEPTED_IMAGE_TYPES, ACCEPTED_PDF_TYPES } from "@/components/ui/Dropzone";
import VoiceCard, { VoiceOption } from "@/components/ui/VoiceCard";

const MAX_PDF_BYTES = 50 * 1024 * 1024; // 50MB

function formatBytes(bytes: number) {
  const mb = bytes / (1024 * 1024);
  if (mb >= 1) return `${mb.toFixed(0)}MB`;
  return `${bytes}B`;
}

const UploadForm = () => {
  const { userId } = useAuth();
  const clerk = useClerk();

  const maleVoiceOptions: VoiceOption[] = useMemo(
    () => [
      {
        id: "daniel",
        name: "Daniel",
        description: "Middle-aged male, British, authoritative but warm.",
      },
      {
        id: "chris",
        name: "Chris",
        description: "Young male, British-Essex, casual & conversational.",
      },
    ],
    []
  );

  const femaleVoiceOptions: VoiceOption[] = useMemo(
    () => [
      {
        id: "rachel",
        name: "Rachel",
        description: "Young female, American, calm & clear.",
      },
      {
        id: "sarah",
        name: "Sarah",
        description: "Young female, American, soft & approachable.",
      },
    ],
    []
  );

  const [pdfFile, setPdfFile] = useState<File | null>(null);
  const [coverPreviewB64, setCoverPreviewB64] = useState<string | null>(null);

  const [title, setTitle] = useState("");
  const [authorName, setAuthorName] = useState("");

  // Defaults are chosen to match the screenshot.
  const [maleVoice, setMaleVoice] = useState<string>("chris");
  const [femaleVoice, setFemaleVoice] = useState<string>("sarah");

  const [isSubmitting, setIsSubmitting] = useState(false);
  const [formError, setFormError] = useState<string | null>(null);

  const pdfInputRef = useRef<HTMLInputElement | null>(null);

  const onPickPdf = async (file: File | null) => {
    setFormError(null);
    if (!file) {
      setPdfFile(null);
      setCoverPreviewB64(null);
      return;
    }

    if (!ACCEPTED_PDF_TYPES.includes(file.type)) {
      setFormError("Please upload a PDF file.");
      return;
    }

    if (file.size > MAX_PDF_BYTES) {
      setFormError(`PDF file must be 50MB or less (selected: ${formatBytes(file.size)}).`);
      return;
    }

    setPdfFile(file);
    setCoverPreviewB64(null);
    setAuthorName("");

    // Call our metadata extraction endpoint
    try {
      const formData = new FormData();
      formData.append("file", file);
      
      const res = await fetch("/api/books/extract-metadata", {
        method: "POST",
        body: formData,
      });

      if (res.ok) {
        const data = await res.json();
        
        // Auto-fill Title (fallback to filename)
        if (data.title && data.title.trim() !== "") {
          setTitle(data.title.trim());
        } else {
          const cleanName = file.name.replace(/\.pdf$/i, "").replace(/[-_]/g, " ");
          setTitle(cleanName);
        }

        // Display base64 preview if available
        if (data.cover_b64) {
          setCoverPreviewB64(data.cover_b64);
        }
      } else {
        const cleanName = file.name.replace(/\.pdf$/i, "").replace(/[-_]/g, " ");
        setTitle(cleanName);
      }
    } catch (e) {
      console.error("Failed to extract metadata", e);
      const cleanName = file.name.replace(/\.pdf$/i, "").replace(/[-_]/g, " ");
      setTitle(cleanName);
    }
  };

  const onSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setFormError(null);

    if (!userId) {
      clerk.openSignIn();
      return;
    }

    if (!pdfFile) {
      setFormError("Please upload a PDF first.");
      return;
    }

    setIsSubmitting(true);
    try {
      // TODO: wire to your backend/API.
      // Keeping it UI-only for now so the page renders immediately.
      console.log("Begin Synthesis", {
        pdfFile,
        title,
        authorName,
        maleVoice,
        femaleVoice,
      });
      await new Promise((r) => setTimeout(r, 600));
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
            <div className="text-sm font-medium text-[var(--primary)] mb-2">Book PDF File</div>
            <Dropzone
              kind="pdf"
              title="Click to upload PDF"
              subtitle="PDF file must be 50MB or less"
              file={pdfFile}
              onClear={() => {
                setPdfFile(null);
                setCoverPreviewB64(null);
              }}
              onPick={onPickPdf}
              inputRef={pdfInputRef}
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

        <div className="space-y-3">
          <div className="text-sm font-medium text-[var(--primary)]">Choose Assistant Voice</div>

          <div className="space-y-3">
            <div>
              <div className="text-[11px] text-[var(--muted-foreground)] font-semibold mb-2">
                Male Voices
              </div>
              <div className="grid grid-cols-2 gap-3">
                {maleVoiceOptions.map((opt) => (
                  <VoiceCard
                    key={opt.id}
                    option={opt}
                    selected={maleVoice === opt.id}
                    onSelect={() => setMaleVoice(opt.id)}
                  />
                ))}
              </div>
            </div>

            <div>
              <div className="text-[11px] text-[var(--muted-foreground)] font-semibold mb-2">
                Female Voices
              </div>
              <div className="grid grid-cols-2 gap-3">
                {femaleVoiceOptions.map((opt) => (
                  <VoiceCard
                    key={opt.id}
                    option={opt}
                    selected={femaleVoice === opt.id}
                    onSelect={() => setFemaleVoice(opt.id)}
                  />
                ))}
              </div>
            </div>
          </div>
        </div>

        {formError && (
          <div className="text-[12px] text-[var(--destructive)]">{formError}</div>
        )}

        <Button
          type="submit"
          disabled={isSubmitting}
          className="w-full h-12 rounded-xl bg-[var(--primary)] hover:bg-[var(--primary)]/90 text-[var(--primary-foreground)] text-sm font-semibold shadow-sm sm:max-w-[360px]"
        >
          {isSubmitting ? "Begin Synthesis..." : "Begin Synthesis"}
        </Button>
      </form>
    </div>
  );
};

export default UploadForm;