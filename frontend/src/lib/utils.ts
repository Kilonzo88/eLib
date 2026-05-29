import { clsx, type ClassValue } from "clsx"
import { twMerge } from "tailwind-merge"

export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs))
}

export function getBackendUrl() {
  const url = process.env.NEXT_PUBLIC_API_URL;
  if (!url) {
    // On client-side, we must use the Next.js proxy (/api) to reach the backend
    // on a different port without needing multiple ngrok tunnels or CORS.
    if (typeof window !== 'undefined') {
        return "/api";
    }
    // Fallback for SSR
    return "http://localhost:8081";
  }
  return url;
}


