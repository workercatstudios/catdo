"use client";

import type { ComponentProps } from "react";
import { cn } from "@/lib/utils";

// An <input> has no pseudo-elements, so the underline is a background gradient: a primary line
// 0.125em thick along the bottom of the text area, whose width runs 0 -> 100 % from the left on
// focus (0.25 s snap) and back towards the right on blur (0.15 s, faster than it came in).
// aria-invalid="true" shakes the field once: -A, +A/2, -A/4, +A/8, 0 in 0.3 s.
const css = `
@layer theme, base, components, utilities;
@layer components {
:where(.kk-pop-input) {
  --kk-pop-input-line: var(--primary, #d6336f);
  background-image: linear-gradient(var(--kk-pop-input-line), var(--kk-pop-input-line));
  background-repeat: no-repeat;
  background-position: right 0.75rem bottom 0.25rem;
  background-size: 0 0.125em;
  transition:
    color 0.15s ease-out,
    border-color 0.15s ease-out,
    box-shadow 0.15s ease-out,
    background-size 0.15s var(--kk-ease-in, cubic-bezier(0.8, 0, 1, 1));
}
:where(.kk-pop-input:focus-visible) {
  background-position: left 0.75rem bottom 0.25rem;
  background-size: calc(100% - 1.5rem) 0.125em;
  transition:
    color 0.15s ease-out,
    border-color 0.15s ease-out,
    box-shadow 0.15s ease-out,
    background-size 0.25s var(--kk-ease-snap, cubic-bezier(0.85, 0, 0.15, 1));
}
:where(.kk-pop-input[aria-invalid="true"]) {
  --kk-pop-input-line: var(--destructive, #e5484d);
  animation: kk-pop-input-shake 0.3s ease-in-out;
}
@keyframes kk-pop-input-shake {
  0%, 100% { translate: 0 0; }
  20% { translate: -0.375em 0; }
  45% { translate: 0.1875em 0; }
  70% { translate: -0.09em 0; }
  88% { translate: 0.045em 0; }
}
/* Reduced motion: the underline appears and goes at once, and nothing shakes. */
@media (prefers-reduced-motion: reduce) {
  :where(.kk-pop-input),
  :where(.kk-pop-input:focus-visible) {
    transition: color 0.15s ease-out, border-color 0.15s ease-out, box-shadow 0.15s ease-out;
  }
  :where(.kk-pop-input[aria-invalid="true"]) { animation: none; }
}
}
`;

function Input({ className, type, ...props }: ComponentProps<"input">) {
  return (
    <>
      <style href="kk-pop-input" precedence="kirakira">
        {css}
      </style>
      <input
        type={type}
        data-slot="input"
        className={cn(
          "kk-pop-input h-9 w-full min-w-0 rounded-xl border border-input bg-transparent px-3 py-1 text-base shadow-xs outline-none selection:bg-primary selection:text-primary-foreground file:inline-flex file:h-7 file:border-0 file:bg-transparent file:text-sm file:font-medium file:text-foreground placeholder:text-muted-foreground disabled:pointer-events-none disabled:cursor-not-allowed disabled:opacity-50 md:text-sm dark:bg-input/30",
          "focus-visible:border-ring focus-visible:ring-[3px] focus-visible:ring-ring/50",
          "aria-invalid:border-destructive aria-invalid:ring-destructive/20 dark:aria-invalid:ring-destructive/40",
          className,
        )}
        {...props}
      />
    </>
  );
}

export { Input };
