"use client";

import type { ComponentProps } from "react";
import { Dialog as DialogPrimitive } from "@base-ui/react/dialog";
import { cn } from "@/lib/utils";

// The panel is centred with `translate`, so the pop runs on `scale` and the rise on `transform`.
// Open: scale 0.6 -> (1.04, 1.06) -> (0.98, 0.99) -> 1 while it rises 4 % of its height; opacity is
// done by 30 %, so the whole bounce is seen. Close: a quick squash and fade on an ease-in.
// Base UI waits for the closed animation before it unmounts, so the exit plays.
const css = `
@layer theme, base, components, utilities;
@layer components {
:where(.kk-pop-dialog-overlay[data-open]) {
  animation: kk-pop-dialog-fade-in 0.2s ease-out;
}
:where(.kk-pop-dialog-overlay[data-closed]) {
  animation: kk-pop-dialog-fade-out 0.15s ease-in both;
}
:where(.kk-pop-dialog-content[data-open]) {
  animation: kk-pop-dialog-in 0.4s ease-in-out backwards;
}
:where(.kk-pop-dialog-content[data-closed]) {
  animation: kk-pop-dialog-out 0.15s ease-in both;
}
@keyframes kk-pop-dialog-in {
  0% { opacity: 0; scale: 0.6; transform: translateY(4%); animation-timing-function: ease-out; }
  30% { opacity: 1; }
  50% { scale: 1.04 1.06; transform: none; animation-timing-function: ease-in-out; }
  75% { scale: 0.98 0.99; }
  100% { opacity: 1; scale: 1; transform: none; }
}
@keyframes kk-pop-dialog-out {
  to { opacity: 0; scale: 0.94 0.9; }
}
@keyframes kk-pop-dialog-fade-in {
  from { opacity: 0; }
}
@keyframes kk-pop-dialog-fade-out {
  to { opacity: 0; }
}
/* The close button squashes while it is held and springs back. */
:where(.kk-pop-dialog-close) {
  transition:
    scale 0.35s var(--kk-ease-spring, cubic-bezier(0.34, 1.56, 0.64, 1)),
    background-color 0.16s ease-out,
    opacity 0.16s ease-out;
}
:where(.kk-pop-dialog-close:active) {
  scale: 1.2 0.8;
  transition-duration: 0.08s;
  transition-timing-function: ease-out;
}
/* Reduced motion: the panel and backdrop fade, nothing moves. */
@media (prefers-reduced-motion: reduce) {
  :where(.kk-pop-dialog-content[data-open]) {
    animation: kk-pop-dialog-fade-in 0.15s ease-out;
  }
  :where(.kk-pop-dialog-content[data-closed]) {
    animation: kk-pop-dialog-fade-out 0.1s ease-in both;
  }
  :where(.kk-pop-dialog-close),
  :where(.kk-pop-dialog-close:active) {
    scale: none;
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

function Dialog({ ...props }: DialogPrimitive.Root.Props) {
  return <DialogPrimitive.Root data-slot="dialog" {...props} />;
}

function DialogTrigger({ ...props }: DialogPrimitive.Trigger.Props) {
  return <DialogPrimitive.Trigger data-slot="dialog-trigger" {...props} />;
}

function DialogPortal({ ...props }: DialogPrimitive.Portal.Props) {
  return <DialogPrimitive.Portal data-slot="dialog-portal" {...props} />;
}

function DialogClose({ ...props }: DialogPrimitive.Close.Props) {
  return <DialogPrimitive.Close data-slot="dialog-close" {...props} />;
}

function DialogOverlay({ className, ...props }: DialogPrimitive.Backdrop.Props) {
  return (
    <DialogPrimitive.Backdrop
      data-slot="dialog-overlay"
      className={cn("kk-pop-dialog-overlay fixed inset-0 z-50 bg-black/50", className)}
      {...props}
    />
  );
}

function DialogContent({
  className,
  children,
  showCloseButton = true,
  ...props
}: DialogPrimitive.Popup.Props & {
  showCloseButton?: boolean;
}) {
  return (
    <>
      <style href="kk-pop-dialog" precedence="kirakira">
        {css}
      </style>
      <DialogPortal>
        <DialogOverlay />
        <DialogPrimitive.Popup
          data-slot="dialog-content"
          className={cn(
            "kk-pop-dialog-content fixed top-[50%] left-[50%] z-50 grid w-full max-w-[calc(100%-2rem)] translate-x-[-50%] translate-y-[-50%] gap-4 rounded-2xl border bg-background p-6 shadow-lg outline-none sm:max-w-lg",
            className,
          )}
          {...props}
        >
          {children}
          {showCloseButton && (
            <DialogPrimitive.Close
              data-slot="dialog-close"
              className="kk-pop-dialog-close absolute top-2.5 right-2.5 inline-flex size-7 items-center justify-center rounded-full opacity-70 outline-none hover:bg-accent hover:text-accent-foreground hover:opacity-100 focus-visible:opacity-100 focus-visible:ring-[3px] focus-visible:ring-ring/50 disabled:pointer-events-none [&_svg]:pointer-events-none [&_svg]:shrink-0 [&_svg:not([class*='size-'])]:size-4"
            >
              <XIcon />
              <span className="sr-only">Close</span>
            </DialogPrimitive.Close>
          )}
        </DialogPrimitive.Popup>
      </DialogPortal>
    </>
  );
}

function DialogHeader({ className, ...props }: ComponentProps<"div">) {
  return (
    <div
      data-slot="dialog-header"
      className={cn("flex flex-col gap-2 text-center sm:text-left", className)}
      {...props}
    />
  );
}

function DialogFooter({
  className,
  showCloseButton = false,
  children,
  ...props
}: ComponentProps<"div"> & {
  showCloseButton?: boolean;
}) {
  return (
    <div
      data-slot="dialog-footer"
      className={cn("flex flex-col-reverse gap-2 sm:flex-row sm:justify-end", className)}
      {...props}
    >
      {children}
      {showCloseButton && (
        <DialogPrimitive.Close
          data-slot="dialog-close"
          className="inline-flex h-9 shrink-0 items-center justify-center gap-2 rounded-full border bg-background px-5 py-2 text-sm font-medium whitespace-nowrap shadow-xs outline-none transition-colors hover:bg-accent hover:text-accent-foreground focus-visible:border-ring focus-visible:ring-[3px] focus-visible:ring-ring/50 disabled:pointer-events-none disabled:opacity-50 dark:border-input dark:bg-input/30 dark:hover:bg-input/50"
        >
          Close
        </DialogPrimitive.Close>
      )}
    </div>
  );
}

function DialogTitle({ className, ...props }: DialogPrimitive.Title.Props) {
  return (
    <DialogPrimitive.Title
      data-slot="dialog-title"
      className={cn("text-lg leading-none font-semibold", className)}
      {...props}
    />
  );
}

function DialogDescription({ className, ...props }: DialogPrimitive.Description.Props) {
  return (
    <DialogPrimitive.Description
      data-slot="dialog-description"
      className={cn("text-sm text-muted-foreground", className)}
      {...props}
    />
  );
}

export {
  Dialog,
  DialogClose,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogOverlay,
  DialogPortal,
  DialogTitle,
  DialogTrigger,
};
