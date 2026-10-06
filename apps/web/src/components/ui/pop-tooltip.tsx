"use client";

import { Tooltip as TooltipPrimitive } from "@base-ui/react/tooltip";
import { cn } from "@/lib/utils";

// A speech bubble that pops out of its trigger. `scale` runs 0.4 -> 1.1 -> 0.96 -> 1 from Base UI's
// --transform-origin over 0.26 s, while the bubble slides the last 0.375rem away from the trigger.
// Opacity is done by 30 %. The exit is a 0.1 s fade, which Base UI waits for before it unmounts.
const css = `
@layer theme, base, components, utilities;
@layer components {
:where(.kk-pop-tooltip-content) {
  --kk-pop-tooltip-x: 0;
  --kk-pop-tooltip-y: 0;
}
:where(.kk-pop-tooltip-content[data-side="top"]) { --kk-pop-tooltip-y: 0.375rem; }
:where(.kk-pop-tooltip-content[data-side="bottom"]) { --kk-pop-tooltip-y: -0.375rem; }
:where(.kk-pop-tooltip-content[data-side="left"]) { --kk-pop-tooltip-x: 0.375rem; }
:where(.kk-pop-tooltip-content[data-side="right"]) { --kk-pop-tooltip-x: -0.375rem; }
:where(.kk-pop-tooltip-content[data-side="inline-start"]) { --kk-pop-tooltip-x: 0.375rem; }
:where(.kk-pop-tooltip-content[data-side="inline-end"]) { --kk-pop-tooltip-x: -0.375rem; }
:where([dir="rtl"] .kk-pop-tooltip-content[data-side="inline-start"]) { --kk-pop-tooltip-x: -0.375rem; }
:where([dir="rtl"] .kk-pop-tooltip-content[data-side="inline-end"]) { --kk-pop-tooltip-x: 0.375rem; }
:where(.kk-pop-tooltip-content[data-open]) {
  animation: kk-pop-tooltip-in 0.26s ease-in-out backwards;
}
:where(.kk-pop-tooltip-content[data-closed]) {
  animation: kk-pop-tooltip-out 0.1s ease-in both;
}
@keyframes kk-pop-tooltip-in {
  0% {
    opacity: 0;
    scale: 0.4;
    translate: var(--kk-pop-tooltip-x) var(--kk-pop-tooltip-y);
    animation-timing-function: ease-out;
  }
  30% { opacity: 1; }
  50% { scale: 1.1; translate: 0 0; animation-timing-function: ease-in-out; }
  75% { scale: 0.96; }
  100% { opacity: 1; scale: 1; translate: 0 0; }
}
@keyframes kk-pop-tooltip-out {
  to { opacity: 0; }
}
@keyframes kk-pop-tooltip-fade-in {
  from { opacity: 0; }
}
/* Reduced motion: it fades in where it rests. The exit is already a fade. */
@media (prefers-reduced-motion: reduce) {
  :where(.kk-pop-tooltip-content[data-open]) {
    animation: kk-pop-tooltip-fade-in 0.12s ease-out;
  }
}
}
`;

function TooltipProvider({ delay = 0, ...props }: TooltipPrimitive.Provider.Props) {
  return <TooltipPrimitive.Provider data-slot="tooltip-provider" delay={delay} {...props} />;
}

function Tooltip({ ...props }: TooltipPrimitive.Root.Props) {
  return <TooltipPrimitive.Root data-slot="tooltip" {...props} />;
}

function TooltipTrigger({ ...props }: TooltipPrimitive.Trigger.Props) {
  return <TooltipPrimitive.Trigger data-slot="tooltip-trigger" {...props} />;
}

function TooltipContent({
  className,
  side = "top",
  sideOffset = 4,
  align = "center",
  alignOffset = 0,
  children,
  ...props
}: TooltipPrimitive.Popup.Props &
  Pick<TooltipPrimitive.Positioner.Props, "align" | "alignOffset" | "side" | "sideOffset">) {
  return (
    <>
      <style href="kk-pop-tooltip" precedence="kirakira">
        {css}
      </style>
      <TooltipPrimitive.Portal>
        <TooltipPrimitive.Positioner
          align={align}
          alignOffset={alignOffset}
          side={side}
          sideOffset={sideOffset}
          className="isolate z-50"
        >
          <TooltipPrimitive.Popup
            data-slot="tooltip-content"
            className={cn(
              "kk-pop-tooltip-content z-50 w-fit max-w-xs origin-(--transform-origin) rounded-2xl bg-foreground px-3 py-1.5 text-xs text-balance text-background",
              className,
            )}
            {...props}
          >
            {children}
            <TooltipPrimitive.Arrow className="z-50 size-2.5 translate-y-[calc(-50%-2px)] rotate-45 rounded-[2px] bg-foreground fill-foreground data-[side=bottom]:top-1 data-[side=inline-end]:top-1/2! data-[side=inline-end]:-left-1 data-[side=inline-end]:-translate-y-1/2 data-[side=inline-start]:top-1/2! data-[side=inline-start]:-right-1 data-[side=inline-start]:-translate-y-1/2 data-[side=left]:top-1/2! data-[side=left]:-right-1 data-[side=left]:-translate-y-1/2 data-[side=right]:top-1/2! data-[side=right]:-left-1 data-[side=right]:-translate-y-1/2 data-[side=top]:-bottom-2.5" />
          </TooltipPrimitive.Popup>
        </TooltipPrimitive.Positioner>
      </TooltipPrimitive.Portal>
    </>
  );
}

export { Tooltip, TooltipTrigger, TooltipContent, TooltipProvider };
