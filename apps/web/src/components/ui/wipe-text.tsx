"use client";

import {
  Children,
  useEffect,
  useImperativeHandle,
  useRef,
  useState,
  type ComponentProps,
  type CSSProperties,
  type ReactNode,
} from "react";
import { cn } from "@/lib/utils";

const css = `
@layer theme, base, components, utilities;
@layer components {
@keyframes kk-wipe-text-in {
  from { translate: calc(var(--kk-wipe-dir) * -101%) 0; inset: 0; z-index: 2; }
  to { translate: 0 0; inset: 0; z-index: 2; }
}
@keyframes kk-wipe-text-out {
  from { translate: 0 0; z-index: 2; }
  to { translate: calc(var(--kk-wipe-dir) * 101%) 0; z-index: 2; }
}
@keyframes kk-wipe-text-mark {
  from { top: 0; bottom: 0; z-index: 0; }
  to { top: var(--kk-wipe-mark-top); bottom: var(--kk-wipe-mark-bottom); z-index: 0; }
}
@keyframes kk-wipe-text-show {
  from { opacity: 0; }
}
:where([data-slot="wipe-text"] .kk-wipe-text-line) {
  --kk-t: calc(var(--kk-delay) + var(--kk-i) * var(--kk-stagger));
  position: relative;
  display: block;
  overflow: hidden;
  isolation: isolate;
  padding: 0.04em 0.16em;
}
:where([data-slot="wipe-text"] .kk-wipe-text-text) {
  position: relative;
  z-index: 1;
  display: block;
  /* A 50 ms cut, timed for the moment the bar fully covers the line. */
  animation: kk-wipe-text-show 50ms linear calc(var(--kk-t) + var(--kk-duration)) both;
}
:where([data-slot="wipe-text"] .kk-wipe-text-bar) {
  position: absolute;
  inset: 0;
  background: var(--kk-wipe-color);
  translate: calc(var(--kk-wipe-dir) * 101%) 0;
  pointer-events: none;
  animation:
    kk-wipe-text-in var(--kk-duration) var(--kk-ease-snap, cubic-bezier(0.85, 0, 0.15, 1)) var(--kk-t) both,
    kk-wipe-text-out var(--kk-duration) var(--kk-ease-snap, cubic-bezier(0.85, 0, 0.15, 1))
      calc(var(--kk-t) + var(--kk-duration) * 1.25) forwards;
}
/*
 * The marker band belongs to the text, so it's sized in em from the middle of the last row, where
 * the glyphs are centred whatever the line height: 0.14em to 0.58em below it, the lower half of
 * the letters and a little under the baseline. The middle is the padding plus half a line up
 * from the bottom. The sweep in keeps the full line (kk-wipe-text-in sets inset: 0), and the
 * squash closes the edges in on the band.
 */
:where([data-slot="wipe-text"][data-variant="marker"] .kk-wipe-text-bar) {
  --kk-wipe-mark-top: calc(100% - 0.04em - 0.5lh + 0.14em);
  --kk-wipe-mark-bottom: calc(0.04em + 0.5lh - 0.58em);
  top: var(--kk-wipe-mark-top);
  bottom: var(--kk-wipe-mark-bottom);
  translate: 0 0;
  animation-name: kk-wipe-text-in, kk-wipe-text-mark;
}
:where([data-slot="wipe-text"][data-state="idle"] :is(.kk-wipe-text-text, .kk-wipe-text-bar)) {
  animation-play-state: paused;
}
@media (prefers-reduced-motion: reduce) {
  :where([data-slot="wipe-text"] :is(.kk-wipe-text-text, .kk-wipe-text-bar)) {
    animation: none;
  }
}
}
`;

type WipeTextProps = Omit<ComponentProps<"div">, "color"> & {
  /** One entry per line. Without it, each child is a line. */
  lines?: ReactNode[];
  as?: "div" | "span" | "h1" | "h2" | "h3" | "p";
  /** Bar colour. An array gives each line its own colour, in turn. */
  color?: string | string[];
  /** Which way the bar travels. */
  direction?: "right" | "left";
  /** `marker` leaves a highlighter band behind the text instead of sweeping off. */
  variant?: "wipe" | "marker";
  /** Milliseconds before the first bar starts. */
  delay?: number;
  /** Milliseconds for each sweep, in and out. */
  duration?: number;
  /** Milliseconds between lines. */
  stagger?: number;
  trigger?: "mount" | "inView";
};

export function WipeText({
  lines,
  children,
  as = "div",
  color = "var(--primary)",
  direction = "right",
  variant = "wipe",
  delay = 200,
  duration = 600,
  stagger = 200,
  trigger = "inView",
  className,
  style,
  ref: forwardedRef,
  ...props
}: WipeTextProps) {
  const Tag = as as "div";
  // Observe the root, which always exists: lines may arrive after mount.
  const ref = useRef<HTMLDivElement>(null);
  useImperativeHandle(forwardedRef, () => ref.current!, []);
  const [seen, setSeen] = useState(false);

  useEffect(() => {
    const node = ref.current;
    if (trigger !== "inView" || !node) return;
    // Inside a Timeline the clock decides when it plays, so it starts as if trigger were "mount".
    if (typeof IntersectionObserver === "undefined" || node.closest('[data-slot="timeline"]')) {
      setSeen(true);
      return;
    }
    const observer = new IntersectionObserver(
      ([entry]) => {
        if (!entry?.isIntersecting) return;
        setSeen(true);
        observer.disconnect();
      },
      { rootMargin: "0px 0px -10% 0px" },
    );
    observer.observe(node);
    return () => observer.disconnect();
  }, [trigger]);

  const items = lines ?? Children.toArray(children);
  const colors = Array.isArray(color) ? color : [color];

  return (
    <Tag
      ref={ref}
      data-slot="wipe-text"
      data-variant={variant}
      data-state={trigger === "mount" || seen ? "play" : "idle"}
      className={cn("flex flex-col items-start", className)}
      style={
        {
          "--kk-delay": `${delay}ms`,
          "--kk-duration": `${duration}ms`,
          "--kk-stagger": `${stagger}ms`,
          "--kk-wipe-dir": direction === "left" ? -1 : 1,
          ...style,
        } as CSSProperties
      }
      {...props}
    >
      <style href="kk-wipe-text" precedence="kirakira">
        {css}
      </style>
      {items.map((line, i) => (
        <span
          key={i}
          className="kk-wipe-text-line"
          style={{ "--kk-i": i, "--kk-wipe-color": colors[i % colors.length] } as CSSProperties}
        >
          <span className="kk-wipe-text-text">{line}</span>
          <span aria-hidden="true" className="kk-wipe-text-bar" />
        </span>
      ))}
    </Tag>
  );
}
