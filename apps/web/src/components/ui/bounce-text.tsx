"use client";

import { useEffect, useRef, useState, type ComponentProps, type CSSProperties } from "react";

const css = `
@layer theme, base, components, utilities;
@layer components {
@keyframes kk-bounce-text-rise {
  from { transform: translateY(100%) scale(1); }
  60% { transform: translateY(-8%) scale(0.95, 1.15); }
  80% { transform: translateY(0) scale(1.04, 0.96); }
  to { transform: translateY(0) scale(1); }
}
@keyframes kk-bounce-text-pop {
  from { opacity: 0; }
  20%, to { opacity: 1; }
  from { transform: scale(0); }
  50% { transform: scale(1.2, 1.25); }
  75% { transform: scale(0.9, 0.95); }
  to { transform: none; }
}
@keyframes kk-bounce-text-drop {
  from { opacity: 0; }
  15%, to { opacity: 1; }
  from { transform: translateY(-120%) scale(0.9, 1.15); animation-timing-function: cubic-bezier(0.6, 0, 1, 0.7); }
  45% { transform: translateY(0) scale(1.18, 0.82); animation-timing-function: cubic-bezier(0, 0.4, 0.4, 1); }
  62% { transform: translateY(-22%) scale(0.96, 1.05); animation-timing-function: cubic-bezier(0.6, 0, 1, 0.7); }
  77% { transform: translateY(0) scale(1.06, 0.94); animation-timing-function: cubic-bezier(0, 0.4, 0.4, 1); }
  88% { transform: translateY(-8%) scale(1); animation-timing-function: cubic-bezier(0.6, 0, 1, 0.7); }
  to { transform: translateY(0) scale(1); }
}
:where([data-slot="bounce-text"] .kk-bounce-text-word) {
  display: inline-block;
  white-space: nowrap;
}
/* Rise comes up out of the baseline: clip below the row only, a little low so descenders survive. */
:where([data-slot="bounce-text"][data-variant="rise"] .kk-bounce-text-word) {
  clip-path: inset(-1em -0.5em -0.1em);
}
:where([data-slot="bounce-text"] .kk-bounce-text-char) {
  display: inline-block;
  transform-origin: 50% 100%;
  animation: kk-bounce-text-rise var(--kk-duration) ease-in-out
    calc(var(--kk-delay) + var(--kk-i) * var(--kk-stagger)) both;
}
:where([data-slot="bounce-text"][data-variant="pop"] .kk-bounce-text-char) {
  animation-name: kk-bounce-text-pop;
  transform-origin: 50% 60%;
}
:where([data-slot="bounce-text"][data-variant="drop"] .kk-bounce-text-char) {
  animation-name: kk-bounce-text-drop;
}
:where([data-slot="bounce-text"][data-state="idle"] .kk-bounce-text-char) {
  animation-play-state: paused;
}
@media (prefers-reduced-motion: reduce) {
  :where([data-slot="bounce-text"] .kk-bounce-text-char) {
    animation: none;
  }
}
}
`;

type Variant = "rise" | "pop" | "drop";
type Order = "forward" | "reverse" | "center" | "shuffle";

const durations: Record<Variant, number> = { rise: 550, pop: 600, drop: 800 };

const space = /^\s+$/u;
const opening = /^[\p{Ps}\p{Pi}]/u;

const cjk = /[\p{Script=Han}\p{Script=Hiragana}\p{Script=Katakana}]/u;
// Closing brackets, 、。！？ and ー never start a line.
const closing = /^[\p{Pe}\p{Pf}\p{Po}ー]/u;

/**
 * Splits text into words and spaces. Words never break inside. Japanese and Chinese may wrap
 * between any two characters, except before closing punctuation or after an opening bracket.
 * Plain string rules rather than Intl.Segmenter's word mode, whose dictionary differs between
 * browsers and would make server and client markup disagree.
 */
function words(text: string) {
  const out: string[] = [];
  for (const chunk of text.split(/(\s+)/u).filter(Boolean)) {
    if (space.test(chunk) || !cjk.test(chunk)) {
      out.push(chunk);
      continue;
    }
    let word = "";
    for (const letter of graphemes(chunk)) {
      const previous = word ? graphemes(word).at(-1)! : "";
      const canBreak =
        word &&
        (cjk.test(letter) || cjk.test(previous) || opening.test(letter)) &&
        !closing.test(letter) &&
        !opening.test(previous);
      if (canBreak) {
        out.push(word);
        word = letter;
      } else word += letter;
    }
    if (word) out.push(word);
  }
  return out;
}

function graphemes(text: string) {
  if (typeof Intl.Segmenter !== "function") return Array.from(text);
  const segmenter = new Intl.Segmenter(undefined, { granularity: "grapheme" });
  return Array.from(segmenter.segment(text), (part) => part.segment);
}

/** The stagger step of each letter. */
function steps(count: number, order: Order) {
  const index = Array.from({ length: count }, (_, i) => i);
  if (order === "reverse") return index.map((i) => count - 1 - i);
  if (order === "center") return index.map((i) => Math.floor(Math.abs(i - (count - 1) / 2)));
  if (order === "shuffle") {
    // Rank by the golden-ratio sequence: the same order every render, and neighbours land apart.
    const step: number[] = [];
    index
      .slice()
      .sort((a, b) => ((a * 0.618034) % 1) - ((b * 0.618034) % 1))
      .forEach((i, rank) => (step[i] = rank));
    return step;
  }
  return index;
}

type BounceTextProps = Omit<ComponentProps<"span">, "children"> & {
  text: string;
  as?: "span" | "h1" | "h2" | "h3" | "p";
  variant?: Variant;
  order?: Order;
  /** Milliseconds between letters. */
  stagger?: number;
  /** Milliseconds per letter. Defaults to 550 for rise, 600 for pop and 800 for drop. */
  duration?: number;
  /** Milliseconds before the first letter. */
  delay?: number;
  trigger?: "mount" | "inView";
};

export function BounceText({
  text,
  as = "span",
  variant = "rise",
  order = "forward",
  stagger = 80,
  duration,
  delay = 200,
  trigger = "inView",
  className,
  style,
  ...props
}: BounceTextProps) {
  const Tag = as as "span";
  const ref = useRef<HTMLSpanElement>(null);
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

  const parts = words(text).map((word) => (space.test(word) ? word : graphemes(word)));
  const step = steps(
    parts.reduce((sum, part) => sum + (typeof part === "string" ? 0 : part.length), 0),
    order,
  );
  let letter = 0;

  return (
    <Tag
      data-slot="bounce-text"
      data-variant={variant}
      data-state={trigger === "mount" || seen ? "play" : "idle"}
      className={className}
      style={
        {
          "--kk-delay": `${delay}ms`,
          "--kk-duration": `${duration ?? durations[variant]}ms`,
          "--kk-stagger": `${stagger}ms`,
          ...style,
        } as CSSProperties
      }
      {...props}
    >
      <style href="kk-bounce-text" precedence="kirakira">
        {css}
      </style>
      <span className="sr-only">{text}</span>
      <span ref={ref} aria-hidden="true">
        {parts.map((part, p) =>
          typeof part === "string" ? (
            part
          ) : (
            <span key={p} className="kk-bounce-text-word">
              {part.map((char, c) => (
                <span
                  key={c}
                  className="kk-bounce-text-char"
                  style={{ "--kk-i": step[letter++] } as CSSProperties}
                >
                  {char}
                </span>
              ))}
            </span>
          ),
        )}
      </span>
    </Tag>
  );
}
