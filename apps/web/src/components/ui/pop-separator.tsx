"use client";

import { useEffect, useImperativeHandle, useRef, useState, type CSSProperties } from "react";
import { Separator as SeparatorPrimitive } from "@base-ui/react/separator";
import { cn } from "@/lib/utils";

// The line draws out from its centre on `scale`, along its own axis, on the in-out snap curve:
// it hangs at the centre for a moment, shoots out, and brakes hard at full length. The fill is
// `backwards`, so a user's own scale-* class applies once it's drawn. A line waiting to scroll
// into view holds the first keyframe, a zero-length line.
const css = `
@layer theme, base, components, utilities;
@layer components {
:where(.kk-pop-separator) {
  animation: kk-pop-separator-draw var(--kk-duration, 500ms)
    var(--kk-ease-snap, cubic-bezier(0.85, 0, 0.15, 1)) var(--kk-delay, 0ms) backwards;
}
:where(.kk-pop-separator[data-orientation="vertical"]) {
  animation-name: kk-pop-separator-draw-vertical;
}
:where(.kk-pop-separator[data-state="waiting"]) {
  animation-play-state: paused;
}
@keyframes kk-pop-separator-draw {
  from { scale: 0 1; }
}
@keyframes kk-pop-separator-draw-vertical {
  from { scale: 1 0; }
}
@media (prefers-reduced-motion: reduce) {
  :where(.kk-pop-separator) { animation: none; }
}
}
`;

function Separator({
  className,
  orientation = "horizontal",
  trigger = "inView",
  delay,
  duration,
  style,
  ref,
  ...props
}: SeparatorPrimitive.Props & {
  /** Draw when mounted, or when it scrolls into view. Inside a Timeline it acts as mount. */
  trigger?: "mount" | "inView";
  /** Milliseconds before the line draws. */
  delay?: number;
  /** Milliseconds for the draw. */
  duration?: number;
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

  const vars: CSSProperties & Record<`--${string}`, string> = {};
  if (delay !== undefined) vars["--kk-delay"] = `${delay}ms`;
  if (duration !== undefined) vars["--kk-duration"] = `${duration}ms`;
  return (
    <>
      <style href="kk-pop-separator" precedence="kirakira">
        {css}
      </style>
      <SeparatorPrimitive
        ref={node}
        data-slot="separator"
        data-state={seen ? undefined : "waiting"}
        orientation={orientation}
        className={cn(
          "kk-pop-separator shrink-0 bg-border data-[orientation=horizontal]:h-px data-[orientation=horizontal]:w-full data-[orientation=vertical]:h-full data-[orientation=vertical]:w-px",
          className,
        )}
        style={{ ...vars, ...style }}
        {...props}
      />
    </>
  );
}

export { Separator };
