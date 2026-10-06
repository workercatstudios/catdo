"use client";

import { useRef } from "react";
import { ScrollArea as ScrollAreaPrimitive } from "@base-ui/react/scroll-area";
import { cn } from "@/lib/utils";

// The scrollbar shows while the pointer is over the area or it scrolls (Base UI's data-hovering
// and data-scrolling), and fades out 0.6 s after both end. When it appears the thumb pops out of
// nothing across the track: 0 -> 135 % -> 90 % of its resting width -> rest, in 0.3 s. It rests as
// a thin pill, 70 % of the track's width; hovering or dragging it springs it out to the full track
// and darkens it. The motion is on `scale`; Base UI moves the thumb with `transform`, so the two
// never clash. A scrollbar that hasn't been shown yet stays hidden instead of fading out on mount.
const css = `
@layer theme, base, components, utilities;
@layer components {
:where(.kk-pop-scroll-area-scrollbar) {
  --kk-pop-scroll-area-rest: 0.7;
}
:where(.kk-pop-scroll-area-scrollbar:not([data-hovering], [data-scrolling])) {
  pointer-events: none;
}
:where(.kk-pop-scroll-area-scrollbar:not(.kk-pop-scroll-area-shown)) {
  opacity: 0;
}
:where(.kk-pop-scroll-area-scrollbar:hover),
:where(.kk-pop-scroll-area-scrollbar:has(> :active)) {
  --kk-pop-scroll-area-rest: 1;
}
:where(.kk-pop-scroll-area-thumb) {
  scale: var(--kk-pop-scroll-area-rest) 1;
  transition:
    scale 0.3s var(--kk-ease-spring, cubic-bezier(0.34, 1.56, 0.64, 1)),
    background-color 0.15s ease-out;
}
:where([data-orientation="horizontal"] > .kk-pop-scroll-area-thumb) {
  scale: 1 var(--kk-pop-scroll-area-rest);
}
:where(.kk-pop-scroll-area-scrollbar:is([data-hovering], [data-scrolling])[data-orientation="vertical"] > .kk-pop-scroll-area-thumb) {
  animation: kk-pop-scroll-area-pop-y 0.3s ease-in-out;
}
:where(.kk-pop-scroll-area-scrollbar:is([data-hovering], [data-scrolling])[data-orientation="horizontal"] > .kk-pop-scroll-area-thumb) {
  animation: kk-pop-scroll-area-pop-x 0.3s ease-in-out;
}
:where(.kk-pop-scroll-area-shown:not([data-hovering], [data-scrolling])) {
  animation: kk-pop-scroll-area-fade 0.2s ease-in 0.6s both;
}
@keyframes kk-pop-scroll-area-pop-y {
  0% { scale: 0 1; animation-timing-function: ease-out; }
  55% { scale: calc(var(--kk-pop-scroll-area-rest) * 1.35) 1; }
  80% { scale: calc(var(--kk-pop-scroll-area-rest) * 0.9) 1; }
}
@keyframes kk-pop-scroll-area-pop-x {
  0% { scale: 1 0; animation-timing-function: ease-out; }
  55% { scale: 1 calc(var(--kk-pop-scroll-area-rest) * 1.35); }
  80% { scale: 1 calc(var(--kk-pop-scroll-area-rest) * 0.9); }
}
@keyframes kk-pop-scroll-area-fade {
  to { opacity: 0; }
}
/* Reduced motion: the scrollbar still appears, fades away and widens, without the pop or spring. */
@media (prefers-reduced-motion: reduce) {
  :where(.kk-pop-scroll-area-thumb) { transition: background-color 0.15s ease-out; }
  :where(.kk-pop-scroll-area-scrollbar > .kk-pop-scroll-area-thumb) { animation: none; }
}
}
`;

function ScrollArea({ className, children, ...props }: ScrollAreaPrimitive.Root.Props) {
  return (
    <>
      <style href="kk-pop-scroll-area" precedence="kirakira">
        {css}
      </style>
      <ScrollAreaPrimitive.Root
        data-slot="scroll-area"
        className={cn("relative", className)}
        {...props}
      >
        {/* Base UI makes the viewport focusable while it scrolls, so a keyboard can scroll it. */}
        <ScrollAreaPrimitive.Viewport
          data-slot="scroll-area-viewport"
          className="size-full rounded-[inherit] transition-[color,box-shadow] outline-none focus-visible:ring-[3px] focus-visible:ring-ring/50 focus-visible:outline-1"
        >
          {children}
        </ScrollAreaPrimitive.Viewport>
        <ScrollBar />
        <ScrollAreaPrimitive.Corner />
      </ScrollAreaPrimitive.Root>
    </>
  );
}

function ScrollBar({
  className,
  orientation = "vertical",
  ...props
}: ScrollAreaPrimitive.Scrollbar.Props) {
  // Latches once the scrollbar has been shown, so only a scrollbar that was visible fades out.
  const shown = useRef(false);
  return (
    <ScrollAreaPrimitive.Scrollbar
      data-slot="scroll-area-scrollbar"
      data-orientation={orientation}
      orientation={orientation}
      className={(state) => {
        if (state.hovering || state.scrolling) shown.current = true;
        return cn(
          "kk-pop-scroll-area-scrollbar group/scroll-area-scrollbar flex touch-none p-px select-none",
          shown.current && "kk-pop-scroll-area-shown",
          orientation === "vertical" && "h-full w-2.5 border-l border-l-transparent",
          orientation === "horizontal" && "h-2.5 flex-col border-t border-t-transparent",
          typeof className === "function" ? className(state) : className,
        );
      }}
      {...props}
    >
      <ScrollAreaPrimitive.Thumb
        data-slot="scroll-area-thumb"
        className="kk-pop-scroll-area-thumb relative flex-1 rounded-full bg-border group-hover/scroll-area-scrollbar:bg-muted-foreground/50 active:bg-muted-foreground/50"
      />
    </ScrollAreaPrimitive.Scrollbar>
  );
}

export { ScrollArea, ScrollBar };
