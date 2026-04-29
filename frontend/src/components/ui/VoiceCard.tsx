"use client";

import React from "react";
import { cn } from "@/lib/utils";

export type VoiceOption = {
  id: string;
  name: string;
  description: string;
};

type VoiceCardProps = {
  option: VoiceOption;
  selected: boolean;
  onSelect: () => void;
};

export default function VoiceCard({
  option,
  selected,
  onSelect,
}: VoiceCardProps) {
  return (
    <button
      type="button"
      onClick={onSelect}
      className={cn(
        "w-full rounded-xl bg-white border px-3 py-3 lg:px-4 lg:py-4 text-left transition-colors",
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
