"use client";

import type { ReactNode } from "react";
import { Toast as ToastPrimitive } from "@base-ui/react/toast";
import { cn } from "@/lib/utils";

// A toast hangs from a pin at its top centre: it drops in and swings to rest, each swing half the
// last (6° → −3° → 1.5° → −0.5° → 0). It leaves fast on an ease-in, sliding on in the swipe
// direction from wherever a swipe let go.
//
// One job per property: Base UI's stack (index, expanded offset, swipe movement) lives in
// `transform`, which transitions; the drop and the exit slide run on `translate`, the swing on
// `rotate`. While a finger drags, Base UI writes the dragged `transform` inline.
const css = `
@layer theme, base, components, utilities;
@layer components {
:where(.kk-pop-toast) {
  --kk-pop-toast-exit-x: 105%;
  --kk-pop-toast-exit-y: 0%;
  --kk-pop-toast-gap: 0.75rem;
  --kk-pop-toast-peek: 0.75rem;
  position: absolute;
  right: 0;
  bottom: 0;
  width: 100%;
  z-index: calc(1000 - var(--toast-index));
  height: var(--toast-frontmost-height, var(--toast-height));
  transform-origin: 50% 0;
  transform:
    translateX(var(--toast-swipe-movement-x))
    translateY(calc(var(--toast-swipe-movement-y) - var(--toast-index) * var(--kk-pop-toast-peek)))
    scale(max(0, 1 - var(--toast-index) * 0.1));
  transition:
    transform 0.5s cubic-bezier(0.22, 1, 0.36, 1),
    opacity 0.5s,
    height 0.15s;
  animation: kk-pop-toast-in 0.55s ease-in-out backwards;
}
/* Hover or focus fans the stack out; the strip under each toast keeps the hover across the gap. */
:where(.kk-pop-toast)::after {
  content: "";
  position: absolute;
  top: 100%;
  left: 0;
  width: 100%;
  height: calc(var(--kk-pop-toast-gap) + 1px);
}
:where(.kk-pop-toast[data-expanded]) {
  height: var(--toast-height);
  transform:
    translateX(var(--toast-swipe-movement-x))
    translateY(calc(var(--toast-swipe-movement-y) - var(--toast-offset-y) - var(--toast-index) * var(--kk-pop-toast-gap)));
}
:where(.kk-pop-toast[data-limited]) { opacity: 0; }
:where(.kk-pop-toast[data-swipe-direction="left"]) { --kk-pop-toast-exit-x: -105%; }
:where(.kk-pop-toast[data-swipe-direction="up"]) { --kk-pop-toast-exit-x: 0%; --kk-pop-toast-exit-y: -105%; }
:where(.kk-pop-toast[data-swipe-direction="down"]) { --kk-pop-toast-exit-x: 0%; --kk-pop-toast-exit-y: 105%; }
:where(.kk-pop-toast[data-ending-style]) {
  animation: kk-pop-toast-out 0.2s var(--kk-ease-in, cubic-bezier(0.8, 0, 1, 1)) forwards;
}
:where(.kk-pop-toast-spin) {
  animation: kk-pop-toast-spin 0.8s linear infinite;
}
@keyframes kk-pop-toast-in {
  0% { opacity: 0; translate: 0 -1.25rem; rotate: 6deg; }
  20% { opacity: 1; }
  40% { translate: 0 0.2rem; rotate: -3deg; }
  62% { translate: 0 0; rotate: 1.5deg; }
  82% { rotate: -0.5deg; }
  100% { rotate: 0deg; }
}
@keyframes kk-pop-toast-out {
  to {
    opacity: 0;
    translate: var(--kk-pop-toast-exit-x) var(--kk-pop-toast-exit-y);
  }
}
@keyframes kk-pop-toast-fade {
  from { opacity: 0; }
}
/* A name of its own: the same name as the entrance wouldn't restart, so the exit wouldn't play. */
@keyframes kk-pop-toast-fade-out {
  to { opacity: 0; }
}
@keyframes kk-pop-toast-spin {
  to { rotate: 1turn; }
}
/* Reduced motion: toasts fade in and out and the stack moves without sliding; a swipe still
   follows the finger. */
@media (prefers-reduced-motion: reduce) {
  :where(.kk-pop-toast) {
    transition: opacity 0.2s;
    animation: kk-pop-toast-fade 0.2s ease-out backwards;
  }
  :where(.kk-pop-toast[data-ending-style]) {
    animation: kk-pop-toast-fade-out 0.15s ease-in forwards;
  }
  :where(.kk-pop-toast-spin) { animation: none; }
}
}
`;

const toast = ToastPrimitive.createToastManager();

function ToastProvider({ ...props }: ToastPrimitive.Provider.Props) {
  return <ToastPrimitive.Provider {...props} />;
}

function ToastPortal({ ...props }: ToastPrimitive.Portal.Props) {
  return <ToastPrimitive.Portal data-slot="toast-portal" {...props} />;
}

function ToastViewport({ className, ...props }: ToastPrimitive.Viewport.Props) {
  return (
    <ToastPrimitive.Viewport
      data-slot="toast-viewport"
      className={cn(
        "pointer-events-none fixed inset-x-4 bottom-4 z-[100] mx-auto w-auto max-w-sm outline-none sm:right-4 sm:left-auto sm:mx-0 sm:w-full",
        className,
      )}
      {...props}
    />
  );
}

function Toast({ className, ...props }: ToastPrimitive.Root.Props) {
  return (
    <>
      <style href="kk-pop-toast" precedence="kirakira">
        {css}
      </style>
      <ToastPrimitive.Root
        data-slot="toast"
        className={cn(
          "kk-pop-toast group/toast pointer-events-auto rounded-2xl border border-border bg-popover text-popover-foreground shadow-lg outline-none select-none focus-visible:border-ring focus-visible:ring-[3px] focus-visible:ring-ring/50",
          "data-[type=error]:border-destructive/40 data-[type=error]:bg-[color-mix(in_oklab,var(--destructive)_10%,var(--popover))] data-[type=error]:text-destructive",
          className,
        )}
        {...props}
      />
    </>
  );
}

function ToastContent({ className, ...props }: ToastPrimitive.Content.Props) {
  return (
    <ToastPrimitive.Content
      data-slot="toast-content"
      className={cn(
        "relative flex h-full items-center gap-3 overflow-hidden p-4 pr-9 transition-opacity duration-250 ease-[cubic-bezier(0.22,1,0.36,1)] data-behind:opacity-0 data-expanded:opacity-100",
        className,
      )}
      {...props}
    />
  );
}

function ToastTitle({ className, ...props }: ToastPrimitive.Title.Props) {
  return (
    <ToastPrimitive.Title
      data-slot="toast-title"
      className={cn("text-sm font-semibold", className)}
      {...props}
    />
  );
}

function ToastDescription({ className, ...props }: ToastPrimitive.Description.Props) {
  return (
    <ToastPrimitive.Description
      data-slot="toast-description"
      className={cn("text-sm opacity-90", className)}
      {...props}
    />
  );
}

function ToastAction({ className, ...props }: ToastPrimitive.Action.Props) {
  return (
    <ToastPrimitive.Action
      data-slot="toast-action"
      className={cn(
        "inline-flex h-8 shrink-0 cursor-pointer items-center justify-center rounded-full border bg-transparent px-3 text-sm font-medium transition-colors outline-none hover:bg-secondary focus-visible:border-ring focus-visible:ring-[3px] focus-visible:ring-ring/50 disabled:pointer-events-none disabled:opacity-50 group-data-[type=error]/toast:border-destructive/40 group-data-[type=error]/toast:hover:bg-destructive/10",
        className,
      )}
      {...props}
    />
  );
}

function ToastClose({ className, children, ...props }: ToastPrimitive.Close.Props) {
  return (
    <ToastPrimitive.Close
      data-slot="toast-close"
      aria-label="Close toast"
      className={cn(
        "absolute top-2 right-2 cursor-pointer rounded-full p-1 opacity-50 transition-opacity outline-none hover:opacity-100 focus-visible:opacity-100 focus-visible:ring-[3px] focus-visible:ring-ring/50",
        className,
      )}
      {...props}
    >
      {children ?? (
        <Icon className="size-4">
          <path d="M18 6 6 18M6 6l12 12" />
        </Icon>
      )}
    </ToastPrimitive.Close>
  );
}

function Icon({ className, children }: { className?: string; children: ReactNode }) {
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
      {children}
    </svg>
  );
}

// shadcn's Toaster shows an icon for the built-in types.
const icons: Record<string, ReactNode> = {
  success: (
    <Icon>
      <circle cx="12" cy="12" r="10" />
      <path d="m8.5 12 2.5 2.5 4.5-5" />
    </Icon>
  ),
  info: (
    <Icon>
      <circle cx="12" cy="12" r="10" />
      <path d="M12 16v-4M12 8h.01" />
    </Icon>
  ),
  warning: (
    <Icon>
      <path d="M10.3 3.9 1.8 18a2 2 0 0 0 1.7 3h17a2 2 0 0 0 1.7-3L13.7 3.9a2 2 0 0 0-3.4 0Z" />
      <path d="M12 9v4M12 17h.01" />
    </Icon>
  ),
  error: (
    <Icon className="text-destructive">
      <path d="M7.9 2h8.2L22 7.9v8.2L16.1 22H7.9L2 16.1V7.9Z" />
      <path d="m15 9-6 6M9 9l6 6" />
    </Icon>
  ),
  loading: (
    <Icon className="kk-pop-toast-spin">
      <path d="M21 12a9 9 0 1 1-6.2-8.6" />
    </Icon>
  ),
};

function ToastIcon({ type }: { type: string | undefined }) {
  const icon = type ? icons[type] : undefined;
  if (!icon) return null;
  return (
    <span
      data-slot="toast-icon"
      className="shrink-0 [&_svg]:pointer-events-none [&_svg:not([class*='size-'])]:size-4"
    >
      {icon}
    </span>
  );
}

function ToastList() {
  const { toasts } = ToastPrimitive.useToastManager();
  return toasts.map((toastItem) => (
    <Toast key={toastItem.id} toast={toastItem}>
      <ToastContent>
        <ToastIcon type={toastItem.type} />
        <div className="flex min-w-0 flex-1 flex-col gap-1">
          <ToastTitle />
          <ToastDescription />
        </div>
        <ToastAction />
        <ToastClose />
      </ToastContent>
    </Toast>
  ));
}

function Toaster({ children, toastManager = toast, ...props }: ToastPrimitive.Provider.Props) {
  return (
    <ToastProvider toastManager={toastManager} {...props}>
      {children}
      <ToastPortal>
        <ToastViewport>
          <ToastList />
        </ToastViewport>
      </ToastPortal>
    </ToastProvider>
  );
}

const createToastManager = ToastPrimitive.createToastManager;
const useToastManager = ToastPrimitive.useToastManager;

export {
  Toaster,
  Toast,
  ToastAction,
  ToastClose,
  ToastContent,
  ToastDescription,
  ToastPortal,
  ToastProvider,
  ToastTitle,
  ToastViewport,
  createToastManager,
  toast,
  useToastManager,
};
