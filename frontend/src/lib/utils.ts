import { clsx, type ClassValue } from "clsx"
import { twMerge } from "tailwind-merge"

export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs))
}

export function getBackendUrl() {
  const url = process.env.NEXT_PUBLIC_API_URL;
  if (!url) {
    console.warn("NEXT_PUBLIC_API_URL is not defined in .env.local, falling back to localhost:8081");
    return "http://localhost:8081";
  }
  return url;
}


