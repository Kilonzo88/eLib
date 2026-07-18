"use client";

import { useEffect, useState } from "react";

interface ScrollIndicatorProps {
  /** Hide once the user has scrolled past this many pixels */
  hideAfter?: number;
  /** Text under the arrow, e.g. "Scroll" — pass null to hide */
  label?: string | null;
}

export default function ScrollIndicator({
  hideAfter = 60,
  label = "Scroll",
}: ScrollIndicatorProps) {
  const [visible, setVisible] = useState(true);

  useEffect(() => {
    const onScroll = () => setVisible(window.scrollY < hideAfter);
    window.addEventListener("scroll", onScroll, { passive: true });
    return () => window.removeEventListener("scroll", onScroll);
  }, [hideAfter]);

  return (
    <div
      aria-hidden="true"
      className={`scroll-indicator ${visible ? "" : "scroll-indicator--hidden"}`}
    >
      <div className="scroll-indicator__fade" />
      <div className="scroll-indicator__content">
        <svg
          className="scroll-indicator__arrow"
          width="20"
          height="20"
          viewBox="0 0 24 24"
          fill="none"
        >
          <path
            d="M6 9l6 6 6-6"
            stroke="currentColor"
            strokeWidth="2"
            strokeLinecap="round"
            strokeLinejoin="round"
          />
        </svg>
        {label && <span className="scroll-indicator__label">{label}</span>}
      </div>

      <style jsx>{`
        .scroll-indicator {
          position: fixed;
          left: 0;
          right: 0;
          bottom: 0;
          height: 96px;
          display: flex;
          align-items: flex-end;
          justify-content: center;
          pointer-events: none;
          z-index: 40;
          opacity: 1;
          transition: opacity 0.3s ease;
        }
        .scroll-indicator--hidden {
          opacity: 0;
        }
        /* soft fade that works on light backgrounds */
        .scroll-indicator__fade {
          position: absolute;
          inset: 0;
          background: radial-gradient(
            120% 100% at 50% 100%,
            rgba(255, 255, 255, 0.92) 0%,
            rgba(255, 255, 255, 0.6) 45%,
            rgba(255, 255, 255, 0) 100%
          );
        }
        .scroll-indicator__content {
          position: relative;
          display: flex;
          flex-direction: column;
          align-items: center;
          gap: 2px;
          padding-bottom: 14px;
          color: var(--primary);
        }
        .scroll-indicator__label {
          font-size: 11px;
          letter-spacing: 0.08em;
          text-transform: uppercase;
        }
        .scroll-indicator__arrow {
          animation: scroll-bounce 1.6s ease-in-out infinite;
        }
        @keyframes scroll-bounce {
          0%,
          100% {
            transform: translateY(0);
            opacity: 0.6;
          }
          50% {
            transform: translateY(6px);
            opacity: 1;
          }
        }
        @media (prefers-reduced-motion: reduce) {
          .scroll-indicator__arrow {
            animation: none;
          }
        }
      `}</style>
    </div>
  );
}
