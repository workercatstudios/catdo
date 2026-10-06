"use client";

import { useState } from "react";
import { Checkbox as CheckboxPrimitive } from "@base-ui/react/checkbox";
import { cn } from "@/lib/utils";

// Pressing squashes the box to 0.85. Checking rebounds it from there, 0.85 -> 1.08 -> 0.97 -> 1 in
// 0.34 s, and a beat later (0.08 s) the tick draws itself in over 0.25 s: an SVG path with
// pathLength="1" whose dash offset runs 1 -> 0. Unchecking fades the mark out in 0.12 s while the
// box fades back to empty; Base UI keeps the indicator mounted until that fade ends. Indeterminate
// does the same with a dash. Nothing plays on first render.
const css = `
@layer theme, base, components, utilities;
@layer components {
:where(.kk-pop-checkbox) {
  transition:
    background-color 0.15s ease-out,
    border-color 0.15s ease-out,
    color 0.15s ease-out,
    box-shadow 0.15s ease-out,
    scale 0.3s var(--kk-ease-spring, cubic-bezier(0.34, 1.56, 0.64, 1));
}
:where(.kk-pop-checkbox[data-animate][data-checked]) {
  animation: kk-pop-checkbox-check 0.34s ease-in-out;
}
:where(.kk-pop-checkbox[data-animate][data-indeterminate]) {
  animation: kk-pop-checkbox-mixed 0.34s ease-in-out;
}
/* Held down, or its label is: squash, even mid-rebound. The rebound starts from this same 0.85. */
:where(.kk-pop-checkbox:active:not([data-disabled]), label:active .kk-pop-checkbox:not([data-disabled])) {
  scale: 0.85;
  animation: none;
  transition-duration: 0.15s, 0.15s, 0.15s, 0.15s, 0.1s;
  transition-timing-function: ease-out;
}
:where(.kk-pop-checkbox-mark) {
  stroke-dasharray: 1;
  stroke-dashoffset: 0;
}
:where(.kk-pop-checkbox[data-animate] .kk-pop-checkbox-indicator[data-checked] .kk-pop-checkbox-mark) {
  animation: kk-pop-checkbox-tick 0.25s ease-in-out 0.08s both;
}
:where(.kk-pop-checkbox[data-animate] .kk-pop-checkbox-indicator[data-indeterminate] .kk-pop-checkbox-mark) {
  animation: kk-pop-checkbox-dash 0.25s ease-in-out 0.08s both;
}
:where(.kk-pop-checkbox[data-animate] .kk-pop-checkbox-indicator[data-unchecked]) {
  animation: kk-pop-checkbox-out 0.12s ease-in both;
}
@keyframes kk-pop-checkbox-check {
  0% { scale: 0.85; }
  40% { scale: 1.08; }
  72% { scale: 0.97; }
  100% { scale: 1; }
}
@keyframes kk-pop-checkbox-mixed {
  0% { scale: 0.85; }
  40% { scale: 1.08; }
  72% { scale: 0.97; }
  100% { scale: 1; }
}
/* Two names, so switching between tick and dash draws again. */
@keyframes kk-pop-checkbox-tick {
  0% { stroke-dashoffset: 1; opacity: 0; }
  1%, 100% { opacity: 1; }
  100% { stroke-dashoffset: 0; }
}
@keyframes kk-pop-checkbox-dash {
  0% { stroke-dashoffset: 1; opacity: 0; }
  1%, 100% { opacity: 1; }
  100% { stroke-dashoffset: 0; }
}
@keyframes kk-pop-checkbox-out {
  to { opacity: 0; }
}
/* Reduced motion: no squash or drawing. The mark and the fill change at once. */
@media (prefers-reduced-motion: reduce) {
  :where(.kk-pop-checkbox),
  :where(.kk-pop-checkbox:active:not([data-disabled]), label:active .kk-pop-checkbox:not([data-disabled])) {
    scale: none;
    animation: none;
    transition: none;
  }
  :where(.kk-pop-checkbox .kk-pop-checkbox-mark) { animation: none; }
  :where(.kk-pop-checkbox .kk-pop-checkbox-indicator) { animation: none; }
}
}
`;

type Mark = "check" | "dash";

function Checkbox({
  className,
  checked,
  indeterminate = false,
  onCheckedChange,
  ...props
}: CheckboxPrimitive.Root.Props) {
  // Motion is armed by the first change (a click, Space, or a click on its label) or a new
  // controlled value, so boxes that render checked don't all pop on load. Base UI reports a change
  // in the same render as the new state, so the fade-out is armed in time.
  const [animate, setAnimate] = useState(false);
  // The last mark shown, kept through unchecking so the right one fades out.
  const [mark, setMark] = useState<Mark>(indeterminate ? "dash" : "check");
  const [seen, setSeen] = useState({ checked, indeterminate });
  if (seen.checked !== checked || seen.indeterminate !== indeterminate) {
    setSeen({ checked, indeterminate });
    setAnimate(true);
    if (indeterminate) setMark("dash");
    else if (checked) setMark("check");
  }

  return (
    <>
      <style href="kk-pop-checkbox" precedence="kirakira">
        {css}
      </style>
      <CheckboxPrimitive.Root
        data-slot="checkbox"
        data-animate={animate ? "" : undefined}
        checked={checked}
        indeterminate={indeterminate}
        onCheckedChange={(value, eventDetails) => {
          setAnimate(true);
          if (value && !indeterminate) setMark("check");
          onCheckedChange?.(value, eventDetails);
        }}
        className={cn(
          "kk-pop-checkbox peer inline-block size-4 shrink-0 rounded-[30%] border border-input shadow-xs outline-none focus-visible:border-ring focus-visible:ring-[3px] focus-visible:ring-ring/50 aria-invalid:border-destructive aria-invalid:ring-destructive/20 data-checked:border-primary data-checked:bg-primary data-checked:text-primary-foreground data-disabled:cursor-not-allowed data-disabled:opacity-50 data-indeterminate:border-primary data-indeterminate:bg-primary data-indeterminate:text-primary-foreground dark:bg-input/30 dark:aria-invalid:ring-destructive/40 dark:data-checked:bg-primary dark:data-indeterminate:bg-primary",
          className,
        )}
        {...props}
      >
        <CheckboxPrimitive.Indicator
          data-slot="checkbox-indicator"
          className="kk-pop-checkbox-indicator grid size-full place-items-center text-primary-foreground"
        >
          <svg
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            strokeWidth={3}
            strokeLinecap="round"
            strokeLinejoin="round"
            aria-hidden="true"
            className="size-full"
          >
            <path
              className="kk-pop-checkbox-mark"
              pathLength={1}
              d={mark === "dash" ? "M6.5 12h11" : "M5 12.5l4.5 4.5L19 7"}
            />
          </svg>
        </CheckboxPrimitive.Indicator>
      </CheckboxPrimitive.Root>
    </>
  );
}

export { Checkbox };
