"use client";

import React from "react";
import { Upload } from "lucide-react";

export const ACCEPTED_BOOK_TYPES = ["application/pdf", "application/epub+zip"];
export const ACCEPTED_IMAGE_TYPES = ["image/jpeg", "image/jpg", "image/png", "image/webp"];

type DropzoneProps = {
  kind: "pdf" | "image";
  title: string;
  subtitle: string;
  file: File | null;
  inputRef: React.RefObject<HTMLInputElement | null>;
  previewB64?: string | null;
  onClear: () => void;
  onPick: (file: File | null) => void;
};

export default function Dropzone({
  kind,
  title,
  subtitle,
  file,
  inputRef,
  previewB64,
  onClear,
  onPick,
}: DropzoneProps) {
  const accept =
    kind === "pdf" ? ACCEPTED_BOOK_TYPES.join(",") : ACCEPTED_IMAGE_TYPES.join(",");

  return (
    <div>
      <label className="block w-full bg-white rounded-xl px-4 py-6 sm:px-5 sm:py-7 text-center cursor-pointer hover:bg-white/90 transition-colors">
        <div className="flex flex-col items-center justify-center gap-2">
          <Upload className="h-5 w-5 text-[var(--primary)]" />
          <div className="text-[var(--primary)] font-medium text-sm">{title}</div>
          <div className="text-[10px] text-[var(--muted-foreground)] leading-tight">
            {subtitle}
          </div>
          {file && (
            <div className="mt-2 text-[10px] text-[var(--muted-foreground)] flex items-center justify-center gap-2 text-left">
              {previewB64 && (
                <img 
                  src={`data:image/png;base64,${previewB64}`} 
                  alt="Cover preview" 
                  className="h-12 w-auto object-cover rounded shadow-sm border border-[rgba(33,42,59,0.12)]" 
                />
              )}
              <span>Selected: <span className="text-[var(--foreground)] mt-1 block">{file.name}</span></span>
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
