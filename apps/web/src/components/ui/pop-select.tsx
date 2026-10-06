"use client";

import type { ComponentProps } from "react";
import { Select as SelectPrimitive } from "@base-ui/react/select";
import { cn } from "@/lib/utils";

// The list pops from Base UI's --transform-origin (the trigger's side when it opens below it, the
// chosen row when it opens over the trigger): scale (0.95, 0.6) -> (1.02, 1.05) -> (0.995, 0.98) -> 1
// in 0.3 s, opaque by 30 %. Rows drop in 6 px (or fade in place over the trigger), 25 ms apart from
// 40 ms, capped at ten steps, and the selected row's check pops 0 -> 1.3 -> 0.9 -> 1 once its row
// has landed. Closing fades and shrinks in 0.15 s; Base UI waits for it before it hides the list.
// The chevron flips past half a turn and settles (0 -> 198 -> 174 -> 180 deg in 0.36 s) on open
// and turns back in 0.15 s on close, as long as the list takes to fade.
const css = `
@layer theme, base, components, utilities;
@layer components {
:where(.kk-pop-select-chevron) {
  transition: rotate 0.15s ease-out;
}
:where(.kk-pop-select-trigger[data-popup-open] .kk-pop-select-chevron) {
  rotate: 180deg;
  transition: none;
  animation: kk-pop-select-flip 0.36s ease-in-out;
}
:where(.kk-pop-select-content) {
  transform-origin: var(--transform-origin, 50% 50%);
}
:where(.kk-pop-select-content[data-open]) {
  animation: kk-pop-select-in 0.3s ease-in-out both;
}
:where(.kk-pop-select-content[data-closed]) {
  animation: kk-pop-select-out 0.15s var(--kk-ease-in, cubic-bezier(0.8, 0, 1, 1)) both;
}
/* backwards, not both: once landed, a disabled row's own opacity applies. */
:where(.kk-pop-select-content[data-open] .kk-pop-select-item),
:where(.kk-pop-select-content[data-open] .kk-pop-select-label) {
  animation: kk-pop-select-drop 0.26s ease-out backwards;
  animation-delay: calc(0.04s + var(--kk-pop-select-step) * 0.025s);
}
/* Over the trigger (data-side="none"), Base UI measures the rows as the list opens to put the
   chosen one on the trigger's value, so they fade in place rather than drop. */
:where(.kk-pop-select-content[data-side="none"][data-open] .kk-pop-select-item),
:where(.kk-pop-select-content[data-side="none"][data-open] .kk-pop-select-label) {
  animation-name: kk-pop-select-fade;
}
:where(.kk-pop-select-content[data-open] .kk-pop-select-check) {
  animation: kk-pop-select-check 0.3s ease-in-out both;
  animation-delay: calc(0.2s + var(--kk-pop-select-step) * 0.025s);
}
/* Stagger steps: a row's place in the list plus its place in its group, up to ten. */
:where(.kk-pop-select-list) { --kk-pop-select-i: 0; --kk-pop-select-j: 0; }
:where(.kk-pop-select-list > :nth-child(2)) { --kk-pop-select-i: 1; }
:where(.kk-pop-select-list > :nth-child(3)) { --kk-pop-select-i: 2; }
:where(.kk-pop-select-list > :nth-child(4)) { --kk-pop-select-i: 3; }
:where(.kk-pop-select-list > :nth-child(5)) { --kk-pop-select-i: 4; }
:where(.kk-pop-select-list > :nth-child(6)) { --kk-pop-select-i: 5; }
:where(.kk-pop-select-list > :nth-child(7)) { --kk-pop-select-i: 6; }
:where(.kk-pop-select-list > :nth-child(8)) { --kk-pop-select-i: 7; }
:where(.kk-pop-select-list > :nth-child(9)) { --kk-pop-select-i: 8; }
:where(.kk-pop-select-list > :nth-child(n + 10)) { --kk-pop-select-i: 9; }
:where(.kk-pop-select-list [role="group"] > :nth-child(2)) { --kk-pop-select-j: 1; }
:where(.kk-pop-select-list [role="group"] > :nth-child(3)) { --kk-pop-select-j: 2; }
:where(.kk-pop-select-list [role="group"] > :nth-child(4)) { --kk-pop-select-j: 3; }
:where(.kk-pop-select-list [role="group"] > :nth-child(5)) { --kk-pop-select-j: 4; }
:where(.kk-pop-select-list [role="group"] > :nth-child(6)) { --kk-pop-select-j: 5; }
:where(.kk-pop-select-list [role="group"] > :nth-child(7)) { --kk-pop-select-j: 6; }
:where(.kk-pop-select-list [role="group"] > :nth-child(8)) { --kk-pop-select-j: 7; }
:where(.kk-pop-select-list [role="group"] > :nth-child(9)) { --kk-pop-select-j: 8; }
:where(.kk-pop-select-list [role="group"] > :nth-child(n + 10)) { --kk-pop-select-j: 9; }
:where(.kk-pop-select-item, .kk-pop-select-label) {
  --kk-pop-select-step: min(var(--kk-pop-select-i) + var(--kk-pop-select-j), 9);
}
@keyframes kk-pop-select-in {
  0% { opacity: 0; scale: 0.95 0.6; }
  30% { opacity: 1; }
  55% { scale: 1.02 1.05; }
  80% { scale: 0.995 0.98; }
  100% { opacity: 1; scale: 1; }
}
@keyframes kk-pop-select-out {
  to { opacity: 0; scale: 0.96 0.9; }
}
@keyframes kk-pop-select-drop {
  0% { opacity: 0; translate: 0 -0.375rem; }
  60% { opacity: 1; translate: 0 0.0625rem; }
  100% { opacity: 1; translate: 0 0; }
}
@keyframes kk-pop-select-check {
  0% { scale: 0; }
  50% { scale: 1.3; }
  75% { scale: 0.9; }
  100% { scale: 1; }
}
@keyframes kk-pop-select-flip {
  0% { rotate: 0deg; }
  55% { rotate: 198deg; }
  78% { rotate: 174deg; }
  100% { rotate: 180deg; }
}
/* Reduced motion: the list fades in and out; nothing scales, drops or spins. */
@media (prefers-reduced-motion: reduce) {
  :where(.kk-pop-select-trigger .kk-pop-select-chevron) { animation: none; transition: none; }
  :where(.kk-pop-select-content[data-open]) { animation: kk-pop-select-fade 0.15s ease-out both; }
  :where(.kk-pop-select-content[data-closed]) { animation: kk-pop-select-fade-out 0.1s ease-in both; }
  :where(.kk-pop-select-content .kk-pop-select-item),
  :where(.kk-pop-select-content .kk-pop-select-label),
  :where(.kk-pop-select-content .kk-pop-select-check) { animation: none; }
}
@keyframes kk-pop-select-fade {
  from { opacity: 0; }
  to { opacity: 1; }
}
@keyframes kk-pop-select-fade-out {
  to { opacity: 0; }
}
}
`;

function CheckIcon({ className }: { className?: string }) {
  return (
    <svg
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth={2.5}
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
      className={className}
    >
      <path d="M20 6 9 17l-5-5" />
    </svg>
  );
}

function ChevronIcon({ up = false, className }: { up?: boolean; className?: string }) {
  return (
    <svg
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth={2}
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
      className={className}
    >
      <path d={up ? "m18 15-6-6-6 6" : "m6 9 6 6 6-6"} />
    </svg>
  );
}

const Select = SelectPrimitive.Root;

function SelectGroup({ ...props }: SelectPrimitive.Group.Props) {
  return <SelectPrimitive.Group data-slot="select-group" {...props} />;
}

function SelectValue({ className, ...props }: SelectPrimitive.Value.Props) {
  return (
    <SelectPrimitive.Value
      data-slot="select-value"
      className={cn("flex flex-1 text-left", className)}
      {...props}
    />
  );
}

function SelectTrigger({
  className,
  size = "default",
  children,
  ...props
}: SelectPrimitive.Trigger.Props & {
  size?: "sm" | "default";
}) {
  return (
    <>
      <style href="kk-pop-select" precedence="kirakira">
        {css}
      </style>
      <SelectPrimitive.Trigger
        data-slot="select-trigger"
        data-size={size}
        className={cn(
          "kk-pop-select-trigger flex w-fit items-center justify-between gap-2 rounded-xl border border-input bg-transparent px-3 py-2 text-sm whitespace-nowrap shadow-xs transition-[color,box-shadow] outline-none focus-visible:border-ring focus-visible:ring-[3px] focus-visible:ring-ring/50 disabled:cursor-not-allowed disabled:opacity-50 aria-invalid:border-destructive aria-invalid:ring-destructive/20 data-[placeholder]:text-muted-foreground data-[size=default]:h-9 data-[size=sm]:h-8 *:data-[slot=select-value]:line-clamp-1 *:data-[slot=select-value]:flex *:data-[slot=select-value]:items-center *:data-[slot=select-value]:gap-2 dark:bg-input/30 dark:hover:bg-input/50 dark:aria-invalid:ring-destructive/40 [&_svg]:pointer-events-none [&_svg]:shrink-0 [&_svg:not([class*='size-'])]:size-4 [&_svg:not([class*='text-'])]:text-muted-foreground",
          className,
        )}
        {...props}
      >
        {children}
        <SelectPrimitive.Icon
          render={
            <svg
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              strokeWidth={2}
              strokeLinecap="round"
              strokeLinejoin="round"
              className="kk-pop-select-chevron size-4 opacity-50"
            >
              <path d="m6 9 6 6 6-6" />
            </svg>
          }
        />
      </SelectPrimitive.Trigger>
    </>
  );
}

function SelectContent({
  className,
  children,
  side = "bottom",
  sideOffset = 4,
  align = "center",
  alignOffset = 0,
  alignItemWithTrigger = true,
  ...props
}: SelectPrimitive.Popup.Props &
  Pick<
    SelectPrimitive.Positioner.Props,
    "align" | "alignOffset" | "side" | "sideOffset" | "alignItemWithTrigger"
  >) {
  return (
    <>
      <style href="kk-pop-select" precedence="kirakira">
        {css}
      </style>
      <SelectPrimitive.Portal>
        <SelectPrimitive.Positioner
          side={side}
          sideOffset={sideOffset}
          align={align}
          alignOffset={alignOffset}
          alignItemWithTrigger={alignItemWithTrigger}
          className="isolate z-50"
        >
          <SelectPrimitive.Popup
            data-slot="select-content"
            data-align-trigger={alignItemWithTrigger}
            className={cn(
              "kk-pop-select-content relative isolate z-50 flex max-h-(--available-height) w-(--anchor-width) min-w-36 flex-col overflow-hidden rounded-xl border bg-popover text-popover-foreground shadow-md",
              className,
            )}
            {...props}
          >
            <SelectScrollUpButton />
            <SelectPrimitive.List className="kk-pop-select-list min-h-0 scroll-my-1 overflow-x-hidden overflow-y-auto p-1">
              {children}
            </SelectPrimitive.List>
            <SelectScrollDownButton />
          </SelectPrimitive.Popup>
        </SelectPrimitive.Positioner>
      </SelectPrimitive.Portal>
    </>
  );
}

function SelectLabel({ className, ...props }: SelectPrimitive.GroupLabel.Props) {
  return (
    <SelectPrimitive.GroupLabel
      data-slot="select-label"
      className={cn("kk-pop-select-label px-2 py-1.5 text-xs text-muted-foreground", className)}
      {...props}
    />
  );
}

function SelectItem({ className, children, ...props }: SelectPrimitive.Item.Props) {
  return (
    <SelectPrimitive.Item
      data-slot="select-item"
      className={cn(
        "kk-pop-select-item relative flex w-full cursor-default items-center gap-2 rounded-lg py-1.5 pr-8 pl-2 text-sm outline-hidden select-none focus:bg-accent focus:text-accent-foreground data-[disabled]:pointer-events-none data-[disabled]:opacity-50 [&_svg]:pointer-events-none [&_svg]:shrink-0 [&_svg:not([class*='size-'])]:size-4 [&_svg:not([class*='text-'])]:text-muted-foreground",
        className,
      )}
      {...props}
    >
      <SelectPrimitive.ItemText className="flex flex-1 shrink-0 items-center gap-2 whitespace-nowrap">
        {children}
      </SelectPrimitive.ItemText>
      <SelectPrimitive.ItemIndicator
        render={
          <span className="kk-pop-select-check pointer-events-none absolute right-2 flex size-4 items-center justify-center" />
        }
      >
        <CheckIcon className="size-4 text-primary" />
      </SelectPrimitive.ItemIndicator>
    </SelectPrimitive.Item>
  );
}

function SelectSeparator({ className, ...props }: SelectPrimitive.Separator.Props) {
  return (
    <SelectPrimitive.Separator
      data-slot="select-separator"
      className={cn("pointer-events-none -mx-1 my-1 h-px bg-border", className)}
      {...props}
    />
  );
}

function SelectScrollUpButton({
  className,
  ...props
}: ComponentProps<typeof SelectPrimitive.ScrollUpArrow>) {
  return (
    <SelectPrimitive.ScrollUpArrow
      data-slot="select-scroll-up-button"
      className={cn(
        "top-0 z-10 flex w-full cursor-default items-center justify-center bg-popover py-1",
        className,
      )}
      {...props}
    >
      <ChevronIcon up className="size-4" />
    </SelectPrimitive.ScrollUpArrow>
  );
}

function SelectScrollDownButton({
  className,
  ...props
}: ComponentProps<typeof SelectPrimitive.ScrollDownArrow>) {
  return (
    <SelectPrimitive.ScrollDownArrow
      data-slot="select-scroll-down-button"
      className={cn(
        "bottom-0 z-10 flex w-full cursor-default items-center justify-center bg-popover py-1",
        className,
      )}
      {...props}
    >
      <ChevronIcon className="size-4" />
    </SelectPrimitive.ScrollDownArrow>
  );
}

export {
  Select,
  SelectContent,
  SelectGroup,
  SelectItem,
  SelectLabel,
  SelectScrollDownButton,
  SelectScrollUpButton,
  SelectSeparator,
  SelectTrigger,
  SelectValue,
};
