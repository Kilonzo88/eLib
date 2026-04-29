"use client";

import React, { useMemo, useRef, useState } from "react";
import { Upload } from "lucide-react";

import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";

type VoiceOption = {
  id: string;
  name: string;
  description: string;
};

const MAX_PDF_BYTES = 50 * 1024 * 1024; // 50MB
const ACCEPTED_PDF_TYPES = ["application/pdf"];
const ACCEPTED_IMAGE_TYPES = ["image/jpeg", "image/jpg", "image/png", "image/webp"];

function formatBytes(bytes: number) {
  const mb = bytes / (1024 * 1024);
  if (mb >= 1) return `${mb.toFixed(0)}MB`;
  return `${bytes}B`;
}

type DropzoneProps = {
  kind: "pdf" | "image";
  title: string;
  subtitle: string;
  file: File | null;
  inputRef: React.RefObject<HTMLInputElement | null>;
  onClear: () => void;
  onPick: (file: File | null) => void;
};

function Dropzone({
  kind,
  title,
  subtitle,
  file,
  inputRef,
  onClear,
  onPick,
}: DropzoneProps) {
  const accept =
    kind === "pdf" ? ACCEPTED_PDF_TYPES.join(",") : ACCEPTED_IMAGE_TYPES.join(",");

  return (
    <div>
      <label className="block w-full bg-white rounded-xl px-4 py-6 text-center cursor-pointer hover:bg-white/90 transition-colors">
        <div className="flex flex-col items-center justify-center gap-2">
          <Upload className="h-5 w-5 text-[var(--primary)]" />
          <div className="text-[var(--primary)] font-medium text-sm">{title}</div>
          <div className="text-[10px] text-[var(--muted-foreground)] leading-tight">
            {subtitle}
          </div>
          {file && (
            <div className="mt-2 text-[10px] text-[var(--muted-foreground)]">
              Selected: <span className="text-[var(--foreground)]">{file.name}</span>
            </div>
          )}
        </div>

        <input
          className="hidden"
          ref={inputRef}
          type="file"
          accept={accept}
          onChange={(e) => {
            const picked = e.target.files?.[0] ?? null;
            onPick(picked);
          }}
        />
      </label>

      {file && (
        <button
          type="button"
          onClick={() => {
            onClear();
            // Ensure the underlying <input type="file"> can re-pick the same file.
            if (inputRef.current) inputRef.current.value = "";
          }}
          className="mt-2 mx-auto block text-[11px] text-[var(--primary)] underline underline-offset-2"
        >
          Remove
        </button>
      )}
    </div>
  );
}

function VoiceCard({
  option,
  selected,
  onSelect,
}: {
  option: VoiceOption;
  selected: boolean;
  onSelect: () => void;
}) {
  return (
    <button
      type="button"
      onClick={onSelect}
      className={cn(
        "w-full rounded-xl bg-white border px-3 py-3 text-left transition-colors",
        selected
          ? "border-[var(--primary)]"
          : "border-[rgba(33,42,59,0.12)] hover:border-[rgba(33,42,59,0.22)]"
      )}
    >
      <div className="flex items-start gap-2">
        <span
          className={cn(
            "mt-0.5 h-3.5 w-3.5 rounded-full border inline-flex items-center justify-center",
            selected
              ? "border-[var(--primary)] bg-[var(--primary)]"
              : "border-[rgba(104,81,65,0.25)] bg-transparent"
          )}
          aria-hidden="true"
        >
          {selected && <span className="h-1.5 w-1.5 rounded-full bg-white" />}
        </span>
        <div className="min-w-0">
          <div className="text-[13px] font-medium text-[var(--primary)] leading-tight">
            {option.name}
          </div>
          <div className="text-[10px] text-[var(--muted-foreground)] leading-tight mt-1">
            {option.description}
          </div>
        </div>
      </div>
    </button>
  );
}

const UploadForm = () => {
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
  const [coverImageFile, setCoverImageFile] = useState<File | null>(null);

  const [title, setTitle] = useState("");
  const [authorName, setAuthorName] = useState("");

  // Defaults are chosen to match the screenshot.
  const [maleVoice, setMaleVoice] = useState<string>("chris");
  const [femaleVoice, setFemaleVoice] = useState<string>("sarah");

  const [isSubmitting, setIsSubmitting] = useState(false);
  const [formError, setFormError] = useState<string | null>(null);

  const pdfInputRef = useRef<HTMLInputElement | null>(null);
  const coverInputRef = useRef<HTMLInputElement | null>(null);

  const onPickPdf = (file: File | null) => {
    setFormError(null);
    if (!file) {
      setPdfFile(null);
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
  };

  const onPickCoverImage = (file: File | null) => {
    setFormError(null);
    if (!file) {
      setCoverImageFile(null);
      return;
    }

    if (!ACCEPTED_IMAGE_TYPES.includes(file.type)) {
      setFormError("Please upload a supported cover image (jpg/png/webp).");
      return;
    }

    setCoverImageFile(file);
  };

  const onSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setFormError(null);

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
        coverImageFile,
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
    <div className="w-full max-w-md mx-auto">
      <p className="text-xs text-[var(--muted-foreground)] mb-3">
        5 of 10 books used (Update)
      </p>

      <form onSubmit={onSubmit} className="flex flex-col gap-5">
        <div>
          <div className="text-sm font-medium text-[var(--primary)] mb-2">Book PDF File</div>
          <Dropzone
            kind="pdf"
            title="Click to upload PDF"
            subtitle="PDF file must be 50MB or less"
            file={pdfFile}
            onClear={() => setPdfFile(null)}
            onPick={onPickPdf}
            inputRef={pdfInputRef}
          />
        </div>

        <div>
          <div className="text-sm font-medium text-[var(--primary)] mb-2">
            Cover Image (Optional)
          </div>
          <Dropzone
            kind="image"
            title="Click to upload cover image"
            subtitle="Leave empty to auto-generate from PDF"
            file={coverImageFile}
            onClear={() => setCoverImageFile(null)}
            onPick={onPickCoverImage}
            inputRef={coverInputRef}
          />
        </div>

        <div className="space-y-2">
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

        {formError && (
          <div className="text-[12px] text-[var(--destructive)]">{formError}</div>
        )}

        <Button
          type="submit"
          disabled={isSubmitting}
          className="w-full h-12 rounded-xl bg-[var(--primary)] hover:bg-[var(--primary)]/90 text-[var(--primary-foreground)] text-sm font-semibold shadow-sm"
        >
          {isSubmitting ? "Begin Synthesis..." : "Begin Synthesis"}
        </Button>
      </form>
    </div>
  );
};

export default UploadForm;