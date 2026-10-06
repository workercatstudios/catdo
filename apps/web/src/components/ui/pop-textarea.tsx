"use client";

import type { ComponentProps } from "react";
import { cn } from "@/lib/utils";

// The same focus underline and invalid shake as Pop Input. The underline is a background gradient,
// so it stays on the bottom edge while the text scrolls and the field grows.
const css = `
@layer theme, base, components, utilities;
@layer components {
:where(.kk-pop-textarea) {
  --kk-pop-textarea-line: var(--primary, #d6336f);
  background-image: linear-gradient(var(--kk-pop-textarea-line), var(--kk-pop-textarea-line));
  background-repeat: no-repeat;
  background-position: right 0.75rem bottom 0.25rem;
  background-size: 0 0.125em;
  transition:
    color 0.15s ease-out,
    border-color 0.15s ease-out,
    box-shadow 0.15s ease-out,
    background-size 0.15s var(--kk-ease-in, cubic-bezier(0.8, 0, 1, 1));
}
:where(.kk-pop-textarea:focus-visible) {
  background-position: left 0.75rem bottom 0.25rem;
  background-size: calc(100% - 1.5rem) 0.125em;
  transition:
    color 0.15s ease-out,
    border-color 0.15s ease-out,
    box-shadow 0.15s ease-out,
    background-size 0.25s var(--kk-ease-snap, cubic-bezier(0.85, 0, 0.15, 1));
}
:where(.kk-pop-textarea[aria-invalid="true"]) {
  --kk-pop-textarea-line: var(--destructive, #e5484d);
  animation: kk-pop-textarea-shake 0.3s ease-in-out;
}
@keyframes kk-pop-textarea-shake {
  0%, 100% { translate: 0 0; }
  20% { translate: -0.375em 0; }
  45% { translate: 0.1875em 0; }
  70% { translate: -0.09em 0; }
  88% { translate: 0.045em 0; }
}
/* Reduced motion: the underline appears and goes at once, and nothing shakes. */
@media (prefers-reduced-motion: reduce) {
  :where(.kk-pop-textarea),
  :where(.kk-pop-textarea:focus-visible) {
    transition: color 0.15s ease-out, border-color 0.15s ease-out, box-shadow 0.15s ease-out;
  }
  :where(.kk-pop-textarea[aria-invalid="true"]) { animation: none; }
}
}
`;

function Textarea({ className, ...props }: ComponentProps<"textarea">) {
  return (
    <>
      <style href="kk-pop-textarea" precedence="kirakira">
        {css}
      </style>
      <textarea
        data-slot="textarea"
        className={cn(
          "kk-pop-textarea flex field-sizing-content min-h-16 w-full rounded-xl border border-input bg-transparent px-3 py-2 text-base shadow-xs outline-none placeholder:text-muted-foreground focus-visible:border-ring focus-visible:ring-[3px] focus-visible:ring-ring/50 disabled:cursor-not-allowed disabled:opacity-50 aria-invalid:border-destructive aria-invalid:ring-destructive/20 md:text-sm dark:bg-input/30 dark:aria-invalid:ring-destructive/40",
          className,
        )}
        {...props}
      />
    </>
  );
}

export { Textarea };
