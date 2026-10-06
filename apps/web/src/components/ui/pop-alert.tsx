"use client";

import {
  useEffect,
  useImperativeHandle,
  useRef,
  useState,
  type ComponentProps,
  type CSSProperties,
} from "react";
import { cva, type VariantProps } from "class-variance-authority";
import { cn } from "@/lib/utils";

// Two beats. The box pops up from a little below, squashed, stretches past its height and
// settles in 0.4 s. Its icon lands 150 ms later: from nothing, past full size with a turn, back
// under, rest. A destructive icon jolts side to side instead of turning once. Both fills are
// `backwards`, so nothing lingers on the box or the icon once they land; an alert waiting to scroll
// into view holds the first, hidden keyframe.
const css = `
@layer theme, base, components, utilities;
@layer components {
:where(.kk-pop-alert) {
  animation: kk-pop-alert-in 0.4s ease-in-out var(--kk-delay, 0ms) backwards;
}
:where(.kk-pop-alert) > :where(svg) {
  animation: kk-pop-alert-icon 0.45s ease-in-out calc(var(--kk-delay, 0ms) + 150ms) backwards;
}
:where(.kk-pop-alert[data-variant="destructive"]) > :where(svg) {
  animation-name: kk-pop-alert-jolt;
}
:where(.kk-pop-alert[data-state="waiting"]),
:where(.kk-pop-alert[data-state="waiting"]) > :where(svg) {
  animation-play-state: paused;
}
@keyframes kk-pop-alert-in {
  from { opacity: 0; transform: translateY(0.75em) scale(0.97, 0.8); animation-timing-function: ease-out; }
  30% { opacity: 1; }
  55% { transform: translateY(-0.1em) scale(1.005, 1.05); }
  80% { transform: scale(1, 0.99); }
  to { transform: none; }
}
@keyframes kk-pop-alert-icon {
  from { opacity: 0; scale: 0; rotate: -30deg; }
  20% { opacity: 1; }
  50% { scale: 1.3; rotate: 10deg; }
  75% { scale: 0.9; rotate: -4deg; }
  to { scale: 1; rotate: 0deg; }
}
@keyframes kk-pop-alert-jolt {
  from { opacity: 0; scale: 0; }
  20% { opacity: 1; }
  40% { scale: 1.3; rotate: 0deg; }
  55% { scale: 1; rotate: -14deg; }
  70% { rotate: 10deg; }
  85% { rotate: -5deg; }
  to { scale: 1; rotate: 0deg; }
}
@media (prefers-reduced-motion: reduce) {
  :where(.kk-pop-alert),
  :where(.kk-pop-alert) > :where(svg) {
    animation: none;
  }
}
}
`;

const alertVariants = cva(
  "kk-pop-alert relative grid w-full grid-cols-[0_1fr] items-start gap-y-0.5 rounded-2xl border px-4 py-3 text-sm has-[>svg]:grid-cols-[calc(var(--spacing)*4)_1fr] has-[>svg]:gap-x-3 [&>svg]:size-4 [&>svg]:translate-y-0.5 [&>svg]:text-current",
  {
    variants: {
      variant: {
        default: "bg-card text-card-foreground",
        destructive:
          "bg-card text-destructive *:data-[slot=alert-description]:text-destructive/90 [&>svg]:text-current",
      },
    },
    defaultVariants: {
      variant: "default",
    },
  },
);

function Alert({
  className,
  variant,
  trigger = "inView",
  delay,
  style,
  ref,
  ...props
}: ComponentProps<"div"> &
  VariantProps<typeof alertVariants> & {
    /** Pop in when mounted, or when it scrolls into view. Inside a Timeline it acts as mount. */
    trigger?: "mount" | "inView";
    /** Milliseconds before the pop. The icon lands 150 ms after the box. */
    delay?: number;
  }) {
  const node = useRef<HTMLDivElement>(null);
  const [seen, setSeen] = useState(trigger === "mount");
  useImperativeHandle(ref, () => node.current as HTMLDivElement, []);

  useEffect(() => {
    const element = node.current;
    if (seen || !element) return;
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
  }, [seen]);

  return (
    <>
      <style href="kk-pop-alert" precedence="kirakira">
        {css}
      </style>
      <div
        ref={node}
        data-slot="alert"
        data-variant={variant ?? "default"}
        data-state={seen ? undefined : "waiting"}
        role="alert"
        className={cn(alertVariants({ variant }), className)}
        style={
          delay === undefined ? style : ({ "--kk-delay": `${delay}ms`, ...style } as CSSProperties)
        }
        {...props}
      />
    </>
  );
}

function AlertTitle({ className, ...props }: ComponentProps<"div">) {
  return (
    <div
      data-slot="alert-title"
      className={cn("col-start-2 line-clamp-1 min-h-4 font-medium tracking-tight", className)}
      {...props}
    />
  );
}

function AlertDescription({ className, ...props }: ComponentProps<"div">) {
  return (
    <div
      data-slot="alert-description"
      className={cn(
        "col-start-2 grid justify-items-start gap-1 text-sm text-muted-foreground [&_p]:leading-relaxed",
        className,
      )}
      {...props}
    />
  );
}

export { Alert, AlertTitle, AlertDescription };
