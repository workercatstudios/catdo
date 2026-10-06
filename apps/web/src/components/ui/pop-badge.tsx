"use client";

import { useEffect, useState, type CSSProperties } from "react";
import { mergeProps } from "@base-ui/react/merge-props";
import { useRender } from "@base-ui/react/use-render";
import { cva, type VariantProps } from "class-variance-authority";
import { cn } from "@/lib/utils";

// Hover pops on `scale` with a little overshoot. The tape variant sits on `rotate` (its tilt) and
// slaps on with a keyframe that overshoots both: big and turned past its tilt, squashed under the
// hand, then settled. The fill is `backwards`, so nothing lingers once it lands. A tape waiting to
// scroll into view holds that first, hidden keyframe.
const css = `
@layer theme, base, components, utilities;
@layer components {
:where(.kk-pop-badge) {
  transition:
    color 0.15s ease-out,
    background-color 0.15s ease-out,
    box-shadow 0.15s ease-out,
    scale 0.2s var(--kk-ease-spring, cubic-bezier(0.34, 1.56, 0.64, 1));
}
@media (hover: hover) {
  :where(.kk-pop-badge:not([data-variant="tape"]):hover) { scale: 1.06; }
}
/* A strip of tape: torn ends, a tilt, and a slap when it mounts. */
:where(.kk-pop-badge[data-variant="tape"]) {
  --kk-pop-badge-tilt: -3deg;
  rotate: var(--kk-pop-badge-tilt);
  clip-path: polygon(
    0.15em 0, calc(100% - 0.2em) 0, 100% 18%, calc(100% - 0.25em) 36%, calc(100% - 0.05em) 55%,
    calc(100% - 0.3em) 74%, 100% 90%, calc(100% - 0.15em) 100%, 0.1em 100%, 0.25em 84%, 0 66%,
    0.2em 47%, 0.05em 30%, 0.3em 14%
  );
  animation: kk-pop-badge-slap 0.4s ease-in-out var(--kk-delay, 0ms) backwards;
}
:where(.kk-pop-badge[data-variant="tape"][data-state="waiting"]) { animation-play-state: paused; }
/* The clip would cut off the focus ring, so a focused tape shows it with straight ends. */
:where(.kk-pop-badge[data-variant="tape"]:focus-visible) { clip-path: none; }
@keyframes kk-pop-badge-slap {
  from {
    opacity: 0;
    scale: 1.2;
    rotate: calc(var(--kk-pop-badge-tilt) - 8deg);
    animation-timing-function: cubic-bezier(0.8, 0, 1, 1);
  }
  20% { opacity: 1; }
  40% { scale: 0.95; rotate: calc(var(--kk-pop-badge-tilt) + 2deg); }
  70% { scale: 1.02; rotate: calc(var(--kk-pop-badge-tilt) - 1deg); }
  to { scale: 1; rotate: var(--kk-pop-badge-tilt); }
}
@media (prefers-reduced-motion: reduce) {
  :where(.kk-pop-badge) { animation: none; }
  :where(.kk-pop-badge:not([data-variant="tape"]):hover) { scale: none; }
}
}
`;

export const badgeVariants = cva(
  "kk-pop-badge inline-flex w-fit shrink-0 items-center justify-center gap-1 overflow-hidden rounded-full border border-transparent px-2 py-0.5 text-xs font-medium whitespace-nowrap focus-visible:border-ring focus-visible:ring-[3px] focus-visible:ring-ring/50 aria-invalid:border-destructive aria-invalid:ring-destructive/20 dark:aria-invalid:ring-destructive/40 [&>svg]:pointer-events-none [&>svg]:size-3",
  {
    variants: {
      variant: {
        default: "bg-primary text-primary-foreground [a&]:hover:bg-primary/90",
        secondary: "bg-secondary text-secondary-foreground [a&]:hover:bg-secondary/90",
        destructive:
          "bg-destructive text-white focus-visible:ring-destructive/20 dark:bg-destructive/60 dark:focus-visible:ring-destructive/40 [a&]:hover:bg-destructive/90",
        outline:
          "border-border text-foreground [a&]:hover:bg-accent [a&]:hover:text-accent-foreground",
        ghost: "[a&]:hover:bg-accent [a&]:hover:text-accent-foreground",
        link: "text-primary underline-offset-4 [a&]:hover:underline",
        // Ink tape with paper text: the page's own foreground and background, swapped.
        tape: "rounded-none bg-foreground px-2.5 pt-0.5 pb-1 text-background [a&]:hover:bg-foreground/90",
      },
    },
    defaultVariants: {
      variant: "default",
    },
  },
);

/** `render` swaps the `<span>` for another element (a link, say) with the badge's styles and motion. */
export type BadgeProps = useRender.ComponentProps<"span"> &
  VariantProps<typeof badgeVariants> & {
    /** Tape only: the resting tilt in degrees. */
    tilt?: number;
    /** Tape only: milliseconds before it slaps on, to stagger several. */
    delay?: number;
    /** Tape only: slap on when mounted, or when it scrolls into view. */
    trigger?: "mount" | "inView";
  };

export function Badge({
  className,
  variant = "default",
  render,
  tilt,
  delay,
  trigger = "inView",
  style,
  ...props
}: BadgeProps) {
  // The node lives in state: with `render`, swapping the element swaps the node, and the
  // observer must follow it.
  const [node, setNode] = useState<HTMLSpanElement | null>(null);
  const [seen, setSeen] = useState(false);

  const waits = variant === "tape" && trigger === "inView" && !seen;
  useEffect(() => {
    const element = node;
    if (!waits || !element) return;
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
  }, [waits, node]);

  const vars: CSSProperties & Record<`--${string}`, string> = {};
  if (tilt !== undefined) vars["--kk-pop-badge-tilt"] = `${tilt}deg`;
  if (delay !== undefined) vars["--kk-delay"] = `${delay}ms`;
  const element = useRender<Record<string, unknown>, HTMLSpanElement>({
    defaultTagName: "span",
    render,
    // Merged with the `ref` in props and the render element's own.
    ref: setNode,
    props: mergeProps<"span">(
      {
        className: cn(badgeVariants({ variant }), className),
        style: { ...vars, ...style },
      },
      props,
    ),
    // Turned into data-slot, data-variant and data-state="waiting" (only while it waits).
    state: { slot: "badge", variant, state: waits ? "waiting" : undefined },
  });
  return (
    <>
      <style href="kk-pop-badge" precedence="kirakira">
        {css}
      </style>
      {element}
    </>
  );
}
