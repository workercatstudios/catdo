"use client";

import type { ComponentProps } from "react";
import { cn } from "@/lib/utils";

// Pressing a label clicks its control, so the label nods: it dips 0.08em in 0.08 s and springs
// back on the spring curve, rising a hair past its line before it settles. A transition, not a
// keyframe, so a quick tap still shows the whole nod.
const css = `
@layer theme, base, components, utilities;
@layer components {
:where(.kk-pop-label) {
  transition: translate 0.3s var(--kk-ease-spring, cubic-bezier(0.34, 1.56, 0.64, 1));
}
:where(.kk-pop-label:active) {
  translate: 0 0.08em;
  transition-duration: 0.08s;
  transition-timing-function: ease-out;
}
/* Next to a disabled control there is nothing to press, so no nod. Base UI's controls mark it
   with data-disabled, native ones with :disabled. */
:where(.peer:disabled ~ .kk-pop-label:active, .peer[data-disabled] ~ .kk-pop-label:active) {
  translate: none;
}
@media (prefers-reduced-motion: reduce) {
  :where(.kk-pop-label),
  :where(.kk-pop-label:active) { translate: none; transition: none; }
}
}
`;

function Label({ className, ...props }: ComponentProps<"label">) {
  return (
    <>
      <style href="kk-pop-label" precedence="kirakira">
        {css}
      </style>
      <label
        data-slot="label"
        className={cn(
          "kk-pop-label flex items-center gap-2 text-sm leading-none font-medium select-none group-data-[disabled=true]:pointer-events-none group-data-[disabled=true]:opacity-50 peer-disabled:cursor-not-allowed peer-disabled:opacity-50 peer-data-disabled:cursor-not-allowed peer-data-disabled:opacity-50",
          className,
        )}
        {...props}
      />
    </>
  );
}

export { Label };
