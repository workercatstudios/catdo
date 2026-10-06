"use client";

import type { ComponentProps, CSSProperties } from "react";
import { cn } from "@/lib/utils";

const css = `
@layer theme, base, components, utilities;
@layer components {
:where(.kk-sparkles-star) {
  position: absolute;
  left: var(--kk-sparkles-x);
  top: var(--kk-sparkles-y);
  width: var(--kk-sparkles-s);
  height: var(--kk-sparkles-s);
  margin: calc(var(--kk-sparkles-s) / -2) 0 0 calc(var(--kk-sparkles-s) / -2);
  color: var(--kk-sparkles-c);
  scale: 0;
  animation: kk-sparkles-twinkle var(--kk-sparkles-cycle) ease-in-out var(--kk-sparkles-delay) infinite both;
}
:where(.kk-sparkles[data-trigger="hover"]:not(:hover, :focus-within) .kk-sparkles-star) {
  animation: none;
}
/* The twinkle takes the first 40% of the cycle; the rest is a pause before the next one. */
@keyframes kk-sparkles-twinkle {
  0% { scale: 0; rotate: -30deg; }
  20% { scale: 1; rotate: 0deg; }
  40%, 100% { scale: 0; rotate: 30deg; }
}
@media (prefers-reduced-motion: reduce) {
  :where(.kk-sparkles-star) { display: none; }
}
}
`;

// A four-point star whose sides curve in towards the centre.
const star = "M12 0Q13.4 10.6 24 12 13.4 13.4 12 24 10.6 13.4 0 12 10.6 10.6 12 0Z";

const defaultColors = ["var(--kk-yellow, #f7d35c)", "var(--kk-pink, #ec5f8f)"];

// Cheap, stable pseudo-random numbers in [0, 1) from the index (no Math.random during render).
const hash = (i: number, seed: number) => ((i + 1) * seed) % 1;

type Vars = CSSProperties & Record<`--${string}`, string | number>;

export interface SparklesProps extends ComponentProps<"span"> {
  /** How many stars. */
  count?: number;
  /** Star colours, cycled in order. */
  colors?: string[];
  /** Size of the largest star in px. Defaults to 0.75em, so stars follow the text size. */
  size?: number;
  /** `loop` twinkles all the time; `hover` only while hovered or focused within. */
  trigger?: "loop" | "hover";
  /** Length of one twinkle in ms. */
  duration?: number;
  /** Time between one star's twinkle and the next in ms. */
  stagger?: number;
}

export function Sparkles({
  count = 6,
  colors = defaultColors,
  size,
  trigger = "loop",
  duration = 600,
  stagger = 200,
  className,
  style,
  children,
  ...props
}: SparklesProps) {
  const stars = Array.from({ length: count }, (_, i) => {
    // Golden-angle steps scatter the stars evenly around an ellipse just outside the content.
    const angle = ((i * 137.508 - 30) * Math.PI) / 180;
    const rx = 48 + 14 * hash(i, 0.618034);
    const ry = 58 + 30 * hash(i, 0.754878);
    return {
      x: `${(50 + Math.cos(angle) * rx).toFixed(2)}%`,
      y: `${(50 + Math.sin(angle) * ry).toFixed(2)}%`,
      scale: 0.55 + 0.45 * hash(i, 0.414214),
      delay: `${i * stagger}ms`,
      color: colors[i % colors.length] ?? defaultColors[0],
    };
  });

  return (
    <span
      data-slot="sparkles"
      data-trigger={trigger}
      className={cn("kk-sparkles relative inline-block", className)}
      style={
        {
          "--kk-sparkles-size": size === undefined ? "0.75em" : `${size}px`,
          // The twinkle fills 40% of the cycle, so the cycle is 2.5 twinkles long.
          "--kk-sparkles-cycle": `${duration * 2.5}ms`,
          ...style,
        } as Vars
      }
      {...props}
    >
      <style href="kk-sparkles" precedence="kirakira">
        {css}
      </style>
      {children}
      <span aria-hidden className="pointer-events-none absolute inset-0">
        {stars.map((s, i) => (
          <svg
            key={i}
            viewBox="0 0 24 24"
            className="kk-sparkles-star"
            style={
              {
                "--kk-sparkles-x": s.x,
                "--kk-sparkles-y": s.y,
                "--kk-sparkles-s": `calc(var(--kk-sparkles-size) * ${s.scale.toFixed(3)})`,
                "--kk-sparkles-delay": s.delay,
                "--kk-sparkles-c": s.color,
              } as Vars
            }
          >
            <path d={star} fill="currentColor" />
          </svg>
        ))}
      </span>
    </span>
  );
}
