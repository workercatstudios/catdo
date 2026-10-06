"use client";

import type { ComponentProps, CSSProperties } from "react";
import { cn } from "@/lib/utils";

// Everything is sized in em, so `text-*` scales the loader, and painted in currentColor.
// Each moving part keys `translate`, `rotate` and `scale` separately, so one part can hop and
// squash at once without the two fighting over `transform`.
const css = `
@layer theme, base, components, utilities;
@layer components {
:where(.kk-pop-loader-dot) {
  transform-origin: 50% 100%;
  animation: kk-pop-loader-hop 1.2s ease-in-out infinite;
  animation-delay: calc(var(--kk-i) * 0.15s);
}
@keyframes kk-pop-loader-hop {
  0%, 62%, 100% { translate: 0 0; scale: 1 1; }
  12% { translate: 0 0; scale: 1.3 0.7; animation-timing-function: cubic-bezier(0.2, 0.6, 0.4, 1); }
  22% { scale: 0.8 1.3; }
  32% { translate: 0 -0.75em; scale: 1 1; animation-timing-function: cubic-bezier(0.6, 0, 0.8, 0.4); }
  43% { scale: 0.85 1.2; }
  46% { translate: 0 0; scale: 1.3 0.7; }
  54% { scale: 0.92 1.08; }
}

:where(.kk-pop-loader-glyph) {
  transform-origin: 50% 100%;
  animation: kk-pop-loader-wave 1.5s ease-in-out infinite;
  animation-delay: calc(var(--kk-i) * 0.06s);
}
@keyframes kk-pop-loader-wave {
  0%, 64%, 100% { translate: 0 0; scale: 1 1; }
  14% { translate: 0 0; scale: 1.3 0.72; }
  30% { translate: 0 -0.3em; scale: 0.82 1.3; }
  46% { translate: 0 0; scale: 1.18 0.8; }
  56% { scale: 0.96 1.04; }
}

/* Three nested boxes, one job each. The outer one jumps a cell to the right each time the tile
   lands, on step-end so the jump is a clean cut at 31.1 % and 64.1 %. The middle one pops in and
   out at the ends of the track. The tile only tips over its bottom-right corner, and snaps back to
   0deg at the same instant the outer box jumps, so the cut can't be seen. */
:where(.kk-pop-loader-roll) {
  translate: 1em 0;
  animation: kk-pop-loader-step 1.8s step-end infinite;
}
@keyframes kk-pop-loader-step {
  0% { translate: 0 0; }
  31.1% { translate: 1em 0; }
  64.1%, 100% { translate: 2em 0; }
}
:where(.kk-pop-loader-pop) {
  transform-origin: 50% 100%;
  animation: kk-pop-loader-pop 1.8s ease-in-out infinite;
}
@keyframes kk-pop-loader-pop {
  0% { scale: 0; }
  7% { scale: 1.15; }
  12%, 82% { scale: 1; }
  87% { scale: 1.12; }
  94%, 100% { scale: 0; }
}
:where(.kk-pop-loader-tile) {
  transform-origin: 100% 100%;
  animation: kk-pop-loader-tip 1.8s ease-in-out infinite;
}
/* Each roll accelerates into the landing; the landing rocks 8deg forward and back. */
@keyframes kk-pop-loader-tip {
  0%, 14% { rotate: 0deg; animation-timing-function: cubic-bezier(0.55, 0, 0.9, 0.6); }
  31% { rotate: 90deg; animation-timing-function: step-end; }
  31.1% { rotate: 0deg; }
  37% { rotate: 8deg; }
  43%, 47% { rotate: 0deg; animation-timing-function: cubic-bezier(0.55, 0, 0.9, 0.6); }
  64% { rotate: 90deg; animation-timing-function: step-end; }
  64.1% { rotate: 0deg; }
  70% { rotate: 8deg; }
  76%, 100% { rotate: 0deg; }
}

:where(.kk-pop-loader-ring) {
  scale: calc(0.7 + var(--kk-i) * 0.3);
  opacity: calc(0.6 - var(--kk-i) * 0.3);
  animation: kk-pop-loader-ring 1.5s var(--kk-ease-out, cubic-bezier(0.05, 0.3, 0.1, 1)) infinite;
  animation-delay: calc(var(--kk-i) * 0.5s);
}
/* Opacity holds for the first quarter, so the ring is still visible as it opens out. */
@keyframes kk-pop-loader-ring {
  0% { scale: 0.3; }
  80%, 100% { scale: 1; }
  0%, 25% { opacity: 1; }
  80%, 100% { opacity: 0; }
}
:where(.kk-pop-loader-core) {
  animation: kk-pop-loader-beat 1.5s ease-in-out infinite;
}
@keyframes kk-pop-loader-beat {
  0%, 26%, 58%, 100% { scale: 1; }
  6% { scale: 1.25; }
  16% { scale: 0.9; }
  39% { scale: 1.18; }
  49% { scale: 0.94; }
}

:where(.kk-pop-loader-bar) {
  scale: 1 var(--kk-rest);
  animation: kk-pop-loader-spike var(--kk-duration) ease-in-out var(--kk-offset) infinite;
}
@keyframes kk-pop-loader-spike {
  0%, 55%, 100% { scale: 1 0.3; }
  14% { scale: 1 1; }
  28% { scale: 1 0.45; }
  38% { scale: 1 0.7; }
}

@media (prefers-reduced-motion: reduce) {
  :where(.kk-pop-loader) :where(*) { animation: none; }
}
}
`;

// Durations with no common beat, so the bars never line up. Rest heights show without motion.
const bars = [
  { duration: 1.1, offset: 0, rest: 0.5 },
  { duration: 1.7, offset: -0.9, rest: 0.8 },
  { duration: 1.3, offset: -0.4, rest: 0.6 },
  { duration: 1.9, offset: -1.3, rest: 1 },
  { duration: 1.5, offset: -0.7, rest: 0.7 },
];

function Art({ variant, label }: { variant: PopLoaderVariant; label: string }) {
  switch (variant) {
    case "wave":
      return (
        <span className="inline-flex items-end pt-[0.3em] leading-none font-black tracking-wide whitespace-pre">
          {Array.from(label, (char, index) => (
            <span
              key={index}
              className="kk-pop-loader-glyph inline-block"
              style={{ "--kk-i": index } as CSSProperties}
            >
              {char}
            </span>
          ))}
        </span>
      );
    case "roll":
      return (
        <span className="relative inline-block h-[1.6em] w-[3em]">
          <span className="absolute inset-x-0 bottom-0 h-[0.12em] rounded-full bg-current opacity-25" />
          <span className="kk-pop-loader-roll absolute bottom-[0.12em] left-0 block size-[1em]">
            <span className="kk-pop-loader-pop block size-full">
              <span className="kk-pop-loader-tile block size-full rounded-[28%] bg-current" />
            </span>
          </span>
        </span>
      );
    case "ripple":
      return (
        <span className="relative inline-block size-[1.6em]">
          {[0, 1].map((index) => (
            <span
              key={index}
              className="kk-pop-loader-ring absolute inset-0 rounded-full border-[0.12em] border-current"
              style={{ "--kk-i": index } as CSSProperties}
            />
          ))}
          <span className="kk-pop-loader-core absolute inset-[34%] rounded-full bg-current" />
        </span>
      );
    case "bars":
      return (
        <span className="inline-flex h-[1.2em] items-center gap-[0.14em]">
          {bars.map((bar, index) => (
            <span
              key={index}
              className="kk-pop-loader-bar block h-full w-[0.2em] rounded-full bg-current"
              style={
                {
                  "--kk-duration": `${bar.duration}s`,
                  "--kk-offset": `${bar.offset}s`,
                  "--kk-rest": bar.rest,
                } as CSSProperties
              }
            />
          ))}
        </span>
      );
    default:
      return (
        <span className="inline-flex h-[1.3em] items-end gap-[0.22em] pb-[0.1em]">
          {[0, 1, 2].map((index) => (
            <span
              key={index}
              className="kk-pop-loader-dot block size-[0.42em] rounded-full bg-current"
              style={{ "--kk-i": index } as CSSProperties}
            />
          ))}
        </span>
      );
  }
}

export type PopLoaderVariant = "dots" | "wave" | "roll" | "ripple" | "bars";

export type PopLoaderProps = ComponentProps<"span"> & {
  variant?: PopLoaderVariant;
  /** Read by screen readers, and the text the `wave` variant animates. */
  label?: string;
};

export function PopLoader({
  className,
  variant = "dots",
  label = "Loading",
  ...props
}: PopLoaderProps) {
  return (
    <span
      role="status"
      data-slot="pop-loader"
      data-variant={variant}
      className={cn(
        "kk-pop-loader inline-flex items-center justify-center align-middle text-primary",
        className,
      )}
      {...props}
    >
      <style href="kk-pop-loader" precedence="kirakira">
        {css}
      </style>
      <span className="sr-only">{label}</span>
      <span aria-hidden className="inline-flex">
        <Art variant={variant} label={label} />
      </span>
    </span>
  );
}
