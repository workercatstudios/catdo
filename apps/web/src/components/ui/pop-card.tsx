"use client";

import {
  useEffect,
  useImperativeHandle,
  useRef,
  useState,
  type ComponentProps,
  type CSSProperties,
} from "react";
import { cn } from "@/lib/utils";

// The lift moves `translate` and the entrance moves `transform`, so a hover during the pop, or a
// user's own translate-* class, never fights the entrance. The entrance fill is `backwards`: once it
// lands nothing is left behind, no transform to trap `position: fixed` children.
const css = `
@layer theme, base, components, utilities;
@layer components {
:where(.kk-pop-card) {
  box-shadow: 0 1px 3px 0 rgb(0 0 0 / 0.1), 0 1px 2px -1px rgb(0 0 0 / 0.1);
  transition: translate 0.16s ease-out, box-shadow 0.16s ease-out;
}
@media (hover: hover) {
  :where(.kk-pop-card[data-lift]:hover) {
    translate: 0 -4px;
    box-shadow: 0 14px 28px -12px rgb(0 0 0 / 0.2), 0 3px 8px -3px rgb(0 0 0 / 0.06);
  }
}
:where(.kk-pop-card[data-lift]:focus-within) {
  translate: 0 -4px;
  box-shadow: 0 14px 28px -12px rgb(0 0 0 / 0.2), 0 3px 8px -3px rgb(0 0 0 / 0.06);
}
:where(.kk-pop-card[data-pop]) {
  animation: kk-pop-card-pop var(--kk-duration, 500ms) ease-in-out var(--kk-delay, 0ms) backwards;
}
:where(.kk-pop-card[data-pop="idle"]) {
  animation: none;
  opacity: 0;
}
/* Up from a little below, past full size, a small dip, rest. Opacity is done by 30 %. */
@keyframes kk-pop-card-pop {
  from { opacity: 0; transform: translateY(16px) scale(0.75); }
  30% { opacity: 1; }
  55% { transform: translateY(-2px) scale(1.03); }
  80% { transform: translateY(0) scale(0.99); }
  to { transform: none; }
}
@media (prefers-reduced-motion: reduce) {
  :where(.kk-pop-card[data-lift]:is(:hover, :focus-within)) { translate: none; }
  :where(.kk-pop-card[data-pop]) { animation: none; opacity: 1; }
}
}
`;

export type CardProps = ComponentProps<"div"> & {
  /** Lift a few px with a softer shadow on hover and keyboard focus inside. */
  lift?: boolean;
  /** Pop in with an overshoot when it mounts or scrolls into view. */
  pop?: boolean;
  /** When the pop plays. Inside a Timeline it acts as mount. */
  trigger?: "mount" | "inView";
  /** Milliseconds before the pop, to stagger a grid. */
  delay?: number;
  /** Milliseconds for the pop. */
  duration?: number;
};

export function Card({
  className,
  lift = true,
  pop = false,
  trigger = "inView",
  delay = 0,
  duration = 500,
  style,
  ref,
  ...props
}: CardProps) {
  const node = useRef<HTMLDivElement>(null);
  const [seen, setSeen] = useState(false);
  useImperativeHandle(ref, () => node.current as HTMLDivElement, []);

  useEffect(() => {
    const element = node.current;
    if (!pop || trigger !== "inView" || seen || !element) return;
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
  }, [pop, trigger, seen]);

  return (
    <>
      <style href="kk-pop-card" precedence="kirakira">
        {css}
      </style>
      <div
        ref={node}
        data-slot="card"
        data-lift={lift ? "" : undefined}
        data-pop={pop ? (trigger === "mount" || seen ? "play" : "idle") : undefined}
        className={cn(
          "kk-pop-card flex flex-col gap-6 rounded-2xl border bg-card py-6 text-card-foreground",
          className,
        )}
        style={
          pop
            ? ({
                "--kk-delay": `${delay}ms`,
                "--kk-duration": `${duration}ms`,
                ...style,
              } as CSSProperties)
            : style
        }
        {...props}
      />
    </>
  );
}

export function CardHeader({ className, ...props }: ComponentProps<"div">) {
  return (
    <div
      data-slot="card-header"
      className={cn(
        "@container/card-header grid auto-rows-min grid-rows-[auto_auto] items-start gap-2 px-6 has-data-[slot=card-action]:grid-cols-[1fr_auto] [.border-b]:pb-6",
        className,
      )}
      {...props}
    />
  );
}

export function CardTitle({ className, ...props }: ComponentProps<"div">) {
  return (
    <div
      data-slot="card-title"
      className={cn("leading-none font-semibold", className)}
      {...props}
    />
  );
}

export function CardDescription({ className, ...props }: ComponentProps<"div">) {
  return (
    <div
      data-slot="card-description"
      className={cn("text-sm text-muted-foreground", className)}
      {...props}
    />
  );
}

export function CardAction({ className, ...props }: ComponentProps<"div">) {
  return (
    <div
      data-slot="card-action"
      className={cn("col-start-2 row-span-2 row-start-1 self-start justify-self-end", className)}
      {...props}
    />
  );
}

export function CardContent({ className, ...props }: ComponentProps<"div">) {
  return <div data-slot="card-content" className={cn("px-6", className)} {...props} />;
}

export function CardFooter({ className, ...props }: ComponentProps<"div">) {
  return (
    <div
      data-slot="card-footer"
      className={cn("flex items-center px-6 [.border-t]:pt-6", className)}
      {...props}
    />
  );
}
