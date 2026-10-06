"use client";

import type { ComponentProps } from "react";
import { Dialog as SheetPrimitive } from "@base-ui/react/dialog";
import { cn } from "@/lib/utils";

// A plate wipe in miniature. The plate is the panel's own box-shadow in the primary colour, offset
// towards the middle of the screen: a copy of the panel lying just under it. It sweeps out ahead of
// the panel (a 2.5rem band at its peak) and the panel catches up and covers it. The panel slides on
// `translate` from fully off-screen, runs 0.375rem past its rest, comes back 0.125rem short and
// settles. An ::after on the outer edge, in the panel's own background, fills the gap the overshoot
// opens against the screen edge. Close: the panel leaves fast on an ease-in; Base UI waits for it
// before it unmounts.
// Each side sets the direction: --kk-pop-sheet-x/y point from the screen edge into the screen.
const css = `
@layer theme, base, components, utilities;
@layer components {
:where(.kk-pop-sheet-overlay[data-open]) {
  animation: kk-pop-sheet-fade-in 0.3s ease-out;
}
:where(.kk-pop-sheet-overlay[data-closed]) {
  animation: kk-pop-sheet-fade-out 0.22s ease-in both;
}
:where(.kk-pop-sheet-content) {
  --kk-pop-sheet-x: -1;
  --kk-pop-sheet-y: 0;
}
:where(.kk-pop-sheet-content[data-side="left"]) { --kk-pop-sheet-x: 1; }
:where(.kk-pop-sheet-content[data-side="top"]) { --kk-pop-sheet-x: 0; --kk-pop-sheet-y: 1; }
:where(.kk-pop-sheet-content[data-side="bottom"]) { --kk-pop-sheet-x: 0; --kk-pop-sheet-y: -1; }
:where(.kk-pop-sheet-content[data-open]) {
  animation:
    kk-pop-sheet-in 0.5s ease-in-out backwards,
    kk-pop-sheet-plate 0.32s ease-in-out backwards;
}
:where(.kk-pop-sheet-content[data-closed]) {
  animation: kk-pop-sheet-out 0.22s ease-in both;
}
/* Fills the screen edge while the panel runs past its rest. Off-screen the rest of the time. */
:where(.kk-pop-sheet-content)::after {
  content: "";
  position: absolute;
  background: inherit;
  pointer-events: none;
}
:where(.kk-pop-sheet-content[data-side="right"])::after { inset: 0 -1rem 0 100%; }
:where(.kk-pop-sheet-content[data-side="left"])::after { inset: 0 100% 0 -1rem; }
:where(.kk-pop-sheet-content[data-side="top"])::after { inset: -1rem 0 100% 0; }
:where(.kk-pop-sheet-content[data-side="bottom"])::after { inset: 100% 0 -1rem 0; }
@keyframes kk-pop-sheet-in {
  0% {
    translate: calc(var(--kk-pop-sheet-x) * -100%) calc(var(--kk-pop-sheet-y) * -100%);
    animation-timing-function: cubic-bezier(0.6, 0, 0.2, 1);
  }
  62% {
    translate: calc(var(--kk-pop-sheet-x) * 0.375rem) calc(var(--kk-pop-sheet-y) * 0.375rem);
    animation-timing-function: ease-in-out;
  }
  82% {
    translate: calc(var(--kk-pop-sheet-x) * -0.125rem) calc(var(--kk-pop-sheet-y) * -0.125rem);
  }
  100% { translate: 0 0; }
}
@keyframes kk-pop-sheet-plate {
  0% {
    box-shadow: 0 0 0 0 var(--kk-pop-sheet-plate, var(--primary, #d6336f));
    animation-timing-function: ease-out;
  }
  35% {
    box-shadow:
      calc(var(--kk-pop-sheet-x) * 2.5rem) calc(var(--kk-pop-sheet-y) * 2.5rem) 0 0
      var(--kk-pop-sheet-plate, var(--primary, #d6336f));
  }
  100% {
    box-shadow: 0 0 0 0 var(--kk-pop-sheet-plate, var(--primary, #d6336f));
  }
}
@keyframes kk-pop-sheet-out {
  to { translate: calc(var(--kk-pop-sheet-x) * -100%) calc(var(--kk-pop-sheet-y) * -100%); }
}
@keyframes kk-pop-sheet-fade-in {
  from { opacity: 0; }
}
@keyframes kk-pop-sheet-fade-out {
  to { opacity: 0; }
}
/* Reduced motion: no slide and no plate. The panel fades in and out where it rests. */
@media (prefers-reduced-motion: reduce) {
  :where(.kk-pop-sheet-content[data-open]) {
    animation: kk-pop-sheet-fade-in 0.2s ease-out;
  }
  :where(.kk-pop-sheet-content[data-closed]) {
    animation: kk-pop-sheet-fade-out 0.15s ease-in both;
  }
}
}
`;

function XIcon({ className }: { className?: string }) {
  return (
    <svg
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="2"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden
      className={className}
    >
      <path d="M18 6 6 18" />
      <path d="m6 6 12 12" />
    </svg>
  );
}

function Sheet({ ...props }: SheetPrimitive.Root.Props) {
  return <SheetPrimitive.Root data-slot="sheet" {...props} />;
}

function SheetTrigger({ ...props }: SheetPrimitive.Trigger.Props) {
  return <SheetPrimitive.Trigger data-slot="sheet-trigger" {...props} />;
}

function SheetClose({ ...props }: SheetPrimitive.Close.Props) {
  return <SheetPrimitive.Close data-slot="sheet-close" {...props} />;
}

function SheetPortal({ ...props }: SheetPrimitive.Portal.Props) {
  return <SheetPrimitive.Portal data-slot="sheet-portal" {...props} />;
}

function SheetOverlay({ className, ...props }: SheetPrimitive.Backdrop.Props) {
  return (
    <SheetPrimitive.Backdrop
      data-slot="sheet-overlay"
      className={cn("kk-pop-sheet-overlay fixed inset-0 z-50 bg-black/50", className)}
      {...props}
    />
  );
}

function SheetContent({
  className,
  children,
  side = "right",
  showCloseButton = true,
  ...props
}: SheetPrimitive.Popup.Props & {
  side?: "top" | "right" | "bottom" | "left";
  showCloseButton?: boolean;
}) {
  return (
    <>
      <style href="kk-pop-sheet" precedence="kirakira">
        {css}
      </style>
      <SheetPortal>
        <SheetOverlay />
        <SheetPrimitive.Popup
          data-slot="sheet-content"
          data-side={side}
          className={cn(
            "kk-pop-sheet-content fixed z-50 flex flex-col gap-4 bg-background shadow-lg outline-none",
            side === "right" && "inset-y-0 right-0 h-full w-3/4 rounded-l-2xl border-l sm:max-w-sm",
            side === "left" && "inset-y-0 left-0 h-full w-3/4 rounded-r-2xl border-r sm:max-w-sm",
            side === "top" && "inset-x-0 top-0 h-auto rounded-b-2xl border-b",
            side === "bottom" && "inset-x-0 bottom-0 h-auto rounded-t-2xl border-t",
            className,
          )}
          {...props}
        >
          {children}
          {showCloseButton && (
            <SheetPrimitive.Close
              data-slot="sheet-close"
              className="absolute top-2.5 right-2.5 inline-flex size-7 items-center justify-center rounded-full opacity-70 outline-none transition-[background-color,opacity] hover:bg-secondary hover:opacity-100 focus-visible:opacity-100 focus-visible:ring-[3px] focus-visible:ring-ring/50 disabled:pointer-events-none"
            >
              <XIcon className="size-4" />
              <span className="sr-only">Close</span>
            </SheetPrimitive.Close>
          )}
        </SheetPrimitive.Popup>
      </SheetPortal>
    </>
  );
}

function SheetHeader({ className, ...props }: ComponentProps<"div">) {
  return (
    <div
      data-slot="sheet-header"
      className={cn("flex flex-col gap-1.5 p-4", className)}
      {...props}
    />
  );
}

function SheetFooter({ className, ...props }: ComponentProps<"div">) {
  return (
    <div
      data-slot="sheet-footer"
      className={cn("mt-auto flex flex-col gap-2 p-4", className)}
      {...props}
    />
  );
}

function SheetTitle({ className, ...props }: SheetPrimitive.Title.Props) {
  return (
    <SheetPrimitive.Title
      data-slot="sheet-title"
      className={cn("font-semibold text-foreground", className)}
      {...props}
    />
  );
}

function SheetDescription({ className, ...props }: SheetPrimitive.Description.Props) {
  return (
    <SheetPrimitive.Description
      data-slot="sheet-description"
      className={cn("text-sm text-muted-foreground", className)}
      {...props}
    />
  );
}

export {
  Sheet,
  SheetTrigger,
  SheetClose,
  SheetContent,
  SheetHeader,
  SheetFooter,
  SheetTitle,
  SheetDescription,
};
