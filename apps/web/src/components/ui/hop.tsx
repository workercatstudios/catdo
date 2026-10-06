"use client";

import {
  useEffect,
  useImperativeHandle,
  useRef,
  useState,
  type ComponentProps,
  type CSSProperties,
} from "react";

// A thrown arc, one transform per layer. Across runs linear, like a thrown ball; up and down runs
// an ease-out to the peak at 35 % and an ease-in back to the ground, so it hangs at the top. The
// squash layer turns on the feet: stretched in the air, squashed flat on landing, then rebounds
// that halve. Its animation is 2.5 times the flight, so the landing falls at 40 % of it.
const css = `
@layer theme, base, components, utilities;
@layer components {
:where([data-slot="hop"]) {
  width: fit-content;
}
:where(.kk-hop-x) {
  animation: kk-hop-x var(--kk-duration) linear var(--kk-delay) both;
}
:where(.kk-hop-y) {
  animation: kk-hop-y var(--kk-duration) linear var(--kk-delay) both;
}
:where(.kk-hop-squash) {
  transform-origin: 50% 100%;
  animation: kk-hop-squash calc(var(--kk-duration) * 2.5) ease-in-out var(--kk-delay) both;
}
:where([data-slot="hop"][data-state="waiting"]) :where(.kk-hop-x, .kk-hop-y, .kk-hop-squash) {
  animation-play-state: paused;
}
@keyframes kk-hop-x {
  from { translate: var(--kk-from-x) 0; opacity: 0; }
  8% { opacity: 1; }
}
@keyframes kk-hop-y {
  0% { translate: 0 var(--kk-from-y); animation-timing-function: cubic-bezier(0.33, 1, 0.68, 1); }
  35% { translate: 0 calc(var(--kk-height) * -1); animation-timing-function: cubic-bezier(0.32, 0, 0.67, 0); }
  100% { translate: 0 0; }
}
@keyframes kk-hop-squash {
  0% { scale: 0.88 1.16; }
  14% { scale: 1; }
  34% { scale: 0.9 1.14; }
  40% { scale: 1.3 0.74; }
  54% { scale: 0.9 1.12; }
  68% { scale: 1.06 0.95; }
  82% { scale: 0.98 1.02; }
  100% { scale: 1; }
}
@media (prefers-reduced-motion: reduce) {
  :where(.kk-hop-x, .kk-hop-y, .kk-hop-squash) { animation: none; }
}
}
`;

type Vars = CSSProperties & Record<`--${string}`, string>;

export type HopProps = ComponentProps<"div"> & {
  /** Where the hop starts, as [x, y] in px from where it lands. Negative x comes from the left. */
  from?: [number, number];
  /** Height of the arc's peak above the landing spot, in px. */
  height?: number;
  /** Wait before take-off, in ms. */
  delay?: number;
  /** Time in the air, in ms. The landing squash takes as long again and half more. */
  duration?: number;
  trigger?: "mount" | "inView";
};

/** Throws its child in along an arc, to land with a squash and settle. For characters and props. */
export function Hop({
  from = [-480, 0],
  height = 260,
  delay = 0,
  duration = 500,
  trigger = "mount",
  style,
  children,
  ref,
  ...props
}: HopProps) {
  const node = useRef<HTMLDivElement>(null);
  const [seen, setSeen] = useState(false);
  const shown = trigger === "mount" || seen;
  useImperativeHandle(ref, () => node.current as HTMLDivElement, []);

  useEffect(() => {
    const element = node.current;
    if (shown || !element) return;
    // Inside a Timeline the clock decides when it plays, so it starts as if trigger were "mount".
    if (typeof IntersectionObserver === "undefined" || element.closest('[data-slot="timeline"]')) {
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
    observer.observe(element);
    return () => observer.disconnect();
  }, [shown]);

  const vars: Vars = {
    "--kk-from-x": `${from[0]}px`,
    "--kk-from-y": `${from[1]}px`,
    "--kk-height": `${height}px`,
    "--kk-delay": `${delay}ms`,
    "--kk-duration": `${duration}ms`,
  };

  return (
    <div
      {...props}
      ref={node}
      data-slot="hop"
      data-state={shown ? "shown" : "waiting"}
      style={{ ...vars, ...style }}
    >
      <style href="kk-hop" precedence="kirakira">
        {css}
      </style>
      <div className="kk-hop-x">
        <div className="kk-hop-y">
          <div className="kk-hop-squash">{children}</div>
        </div>
      </div>
    </div>
  );
}
