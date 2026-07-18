"use client";

import React from "react";
import { useAuth } from "@clerk/nextjs";
import { getBackendUrl } from "@/lib/utils";

export default function ClaimTracker() {
  const { userId, getToken } = useAuth();

  React.useEffect(() => {
    if (!userId) return;

    const params = new URLSearchParams(window.location.search);
    const claimId = params.get("claim");
    const storedId = localStorage.getItem("pending_claim_book_id");

    const targetId = claimId || storedId;

    if (targetId) {
      const claimBook = async () => {
        try {
          const token = await getToken();
          const res = await fetch(`/api/books/claim`, {
            method: "POST",
            headers: {
              "Content-Type": "application/json",
              Authorization: `Bearer ${token}`,
            },
            body: JSON.stringify({ book_id: targetId }),
          });

          if (res.ok) {
            localStorage.removeItem("pending_claim_book_id");
            
            // Clean up the URL if needed
            if (claimId) {
              const newParams = new URLSearchParams(window.location.search);
              newParams.delete("claim");
              const newUrl = window.location.pathname + (newParams.toString() ? "?" + newParams.toString() : "");
              window.history.replaceState({}, "", newUrl);
            }

            // Force a refresh or redirect to home to show the new book
            window.location.href = "/";
          } else {
            // If it's 404, maybe it's already claimed or invalid, so clear it
            if (res.status === 404) {
              localStorage.removeItem("pending_claim_book_id");
            }
          }
        } catch (e) {
          // Silent catch to avoid spamming the console in background
        }
      };
      claimBook();
    }
  }, [userId, getToken]);

  return null; // This component has no UI
}
