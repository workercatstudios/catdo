"use client";

import { Accordion as AccordionPrimitive } from "@base-ui/react/accordion";
import { cn } from "@/lib/utils";

// Opening grows the panel to Base UI's measured --accordion-panel-height, runs 6 px past it and
// settles; the text inside fades and rises a beat later. Closing is quicker and falls shut on an
// ease-in, and Base UI waits for it before it unmounts the panel. The chevron turns past its mark
// and settles back (0 → 195° → 175° → 180°), then turns home on close.
const css = `
@layer theme, base, components, utilities;
@layer components {
:where(.kk-pop-accordion-content[data-open]) {
  animation: kk-pop-accordion-open 0.38s ease-in-out;
}
:where(.kk-pop-accordion-content[data-closed]) {
  animation: kk-pop-accordion-close 0.2s var(--kk-ease-in, cubic-bezier(0.8, 0, 1, 1));
}
/* Base UI measures the panel's scrollHeight while data-starting-style is on, and on mount, where a
   panel that starts open keeps an inline animation-name: none until it first closes. The risen text
   would add its 0.5rem to that height, so it waits for both to go. */
:where(.kk-pop-accordion-content[data-open]:not([data-starting-style], [style*="animation-name"]) > .kk-pop-accordion-inner) {
  animation: kk-pop-accordion-rise 0.3s var(--kk-ease-out, cubic-bezier(0.05, 0.3, 0.1, 1)) 0.08s both;
}
:where(.kk-pop-accordion-content[data-closed] > .kk-pop-accordion-inner) {
  animation: kk-pop-accordion-fade 0.2s ease-in both;
}
:where(.kk-pop-accordion-chevron) {
  transition: rotate 0.2s var(--kk-ease-in, cubic-bezier(0.8, 0, 1, 1));
}
:where([data-panel-open] > .kk-pop-accordion-chevron) {
  rotate: 180deg;
  transition: none;
  animation: kk-pop-accordion-flip 0.42s ease-in-out;
}
@keyframes kk-pop-accordion-open {
  0% { height: 0; }
  55% { height: calc(var(--accordion-panel-height) + 6px); }
  80% { height: calc(var(--accordion-panel-height) - 2px); }
  100% { height: var(--accordion-panel-height); }
}
@keyframes kk-pop-accordion-close {
  from { height: var(--accordion-panel-height); }
  to { height: 0; }
}
@keyframes kk-pop-accordion-rise {
  from { opacity: 0; translate: 0 0.5rem; }
}
@keyframes kk-pop-accordion-fade {
  to { opacity: 0; }
}
@keyframes kk-pop-accordion-flip {
  0% { rotate: 0deg; }
  55% { rotate: 195deg; }
  80% { rotate: 175deg; }
  100% { rotate: 180deg; }
}
/* Reduced motion: panels open and close at once, the text fades in and the chevron flips. */
@media (prefers-reduced-motion: reduce) {
  :where(.kk-pop-accordion-content) { animation: none; }
  :where(.kk-pop-accordion-content[data-open] > .kk-pop-accordion-inner) {
    animation: kk-pop-accordion-fade 0.2s ease-out reverse both;
  }
  :where(.kk-pop-accordion-content[data-closed] > .kk-pop-accordion-inner) { animation: none; }
  :where(.kk-pop-accordion-chevron),
  :where([data-panel-open] > .kk-pop-accordion-chevron) { transition: none; animation: none; }
}
}
`;

function Accordion(props: AccordionPrimitive.Root.Props) {
  return (
    <>
      <style href="kk-pop-accordion" precedence="kirakira">
        {css}
      </style>
      <AccordionPrimitive.Root data-slot="accordion" {...props} />
    </>
  );
}

function AccordionItem({ className, ...props }: AccordionPrimitive.Item.Props) {
  return (
    <AccordionPrimitive.Item
      data-slot="accordion-item"
      className={cn("border-b last:border-b-0", className)}
      {...props}
    />
  );
}

function AccordionTrigger({ className, children, ...props }: AccordionPrimitive.Trigger.Props) {
  return (
    <AccordionPrimitive.Header className="flex">
      <AccordionPrimitive.Trigger
        data-slot="accordion-trigger"
        className={cn(
          "flex flex-1 cursor-pointer items-start justify-between gap-4 rounded-lg py-4 text-left text-sm font-medium transition-all outline-none hover:underline focus-visible:border-ring focus-visible:ring-[3px] focus-visible:ring-ring/50 disabled:pointer-events-none disabled:opacity-50 aria-disabled:pointer-events-none aria-disabled:opacity-50",
          className,
        )}
        {...props}
      >
        {children}
        <svg
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          strokeWidth="2"
          strokeLinecap="round"
          strokeLinejoin="round"
          aria-hidden
          className="kk-pop-accordion-chevron pointer-events-none size-4 shrink-0 translate-y-0.5 text-muted-foreground"
        >
          <path d="m6 9 6 6 6-6" />
        </svg>
      </AccordionPrimitive.Trigger>
    </AccordionPrimitive.Header>
  );
}

function AccordionContent({ className, children, ...props }: AccordionPrimitive.Panel.Props) {
  return (
    <AccordionPrimitive.Panel
      data-slot="accordion-content"
      className="kk-pop-accordion-content overflow-hidden text-sm"
      {...props}
    >
      <div className={cn("kk-pop-accordion-inner pt-0 pb-4", className)}>{children}</div>
    </AccordionPrimitive.Panel>
  );
}

export { Accordion, AccordionItem, AccordionTrigger, AccordionContent };
