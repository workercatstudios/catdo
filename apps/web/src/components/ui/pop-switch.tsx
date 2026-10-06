"use client";

import { useState } from "react";
import { Switch as SwitchPrimitive } from "@base-ui/react/switch";
import { cn } from "@/lib/utils";

// The thumb slides on `translate` (0.25 s, a strong ease-out) and pops on `scale`, so the two
// never fight. Pressing shrinks it to 0.85; a toggle then ducks it to 0.78 while it travels (20 %)
// and pops it to 1.12 as it lands (55 %), 0.97, then 1, over 0.4 s. The keyframe is named after
// the new state, so every toggle starts it afresh. Nothing plays on first render.
const css = `
@layer theme, base, components, utilities;
@layer components {
:where(.kk-pop-switch-thumb) {
  transition:
    translate 0.25s var(--kk-ease-out, cubic-bezier(0.05, 0.3, 0.1, 1)),
    background-color 0.2s ease-out;
}
:where(.kk-pop-switch[data-animate] .kk-pop-switch-thumb[data-checked]) {
  animation: kk-pop-switch-on 0.4s ease-in-out;
}
:where(.kk-pop-switch[data-animate] .kk-pop-switch-thumb[data-unchecked]) {
  animation: kk-pop-switch-off 0.4s ease-in-out;
}
/* Held down, or its label is: the thumb shrinks, even mid-pop. */
:where(
  .kk-pop-switch:active:not([data-disabled]) .kk-pop-switch-thumb,
  label:active .kk-pop-switch:not([data-disabled]) .kk-pop-switch-thumb
) {
  animation: kk-pop-switch-press 0.1s ease-out forwards;
}
@keyframes kk-pop-switch-press {
  to { scale: 0.85; }
}
/* Two names, one per direction. */
@keyframes kk-pop-switch-on {
  0% { scale: 0.85; }
  20% { scale: 0.78; }
  55% { scale: 1.12; }
  80% { scale: 0.97; }
  100% { scale: 1; }
}
@keyframes kk-pop-switch-off {
  0% { scale: 0.85; }
  20% { scale: 0.78; }
  55% { scale: 1.12; }
  80% { scale: 0.97; }
  100% { scale: 1; }
}
/* Reduced motion: the thumb jumps across and keeps its size. The colours still change. */
@media (prefers-reduced-motion: reduce) {
  :where(.kk-pop-switch .kk-pop-switch-thumb),
  :where(
    .kk-pop-switch:active:not([data-disabled]) .kk-pop-switch-thumb,
    label:active .kk-pop-switch:not([data-disabled]) .kk-pop-switch-thumb
  ) {
    animation: none;
    transition: background-color 0.2s ease-out;
  }
}
}
`;

function Switch({
  className,
  size = "default",
  checked,
  onCheckedChange,
  ...props
}: SwitchPrimitive.Root.Props & {
  size?: "sm" | "default";
}) {
  // Armed by the first change (a click, Space, or a click on its label) or a new controlled value,
  // so switches that render on don't all pop on load.
  const [animate, setAnimate] = useState(false);
  const [seen, setSeen] = useState(checked);
  if (seen !== checked) {
    setSeen(checked);
    setAnimate(true);
  }

  return (
    <>
      <style href="kk-pop-switch" precedence="kirakira">
        {css}
      </style>
      <SwitchPrimitive.Root
        data-slot="switch"
        data-size={size}
        data-animate={animate ? "" : undefined}
        checked={checked}
        onCheckedChange={(value, eventDetails) => {
          setAnimate(true);
          onCheckedChange?.(value, eventDetails);
        }}
        className={cn(
          "kk-pop-switch peer group/switch inline-flex shrink-0 items-center rounded-full border border-transparent shadow-xs transition-all outline-none focus-visible:border-ring focus-visible:ring-[3px] focus-visible:ring-ring/50 data-disabled:cursor-not-allowed data-disabled:opacity-50 data-[size=default]:h-[1.15rem] data-[size=default]:w-8 data-[size=sm]:h-3.5 data-[size=sm]:w-6 data-checked:bg-primary data-unchecked:bg-input dark:data-unchecked:bg-input/80",
          className,
        )}
        {...props}
      >
        <SwitchPrimitive.Thumb
          data-slot="switch-thumb"
          className="kk-pop-switch-thumb pointer-events-none block rounded-full bg-background ring-0 group-data-[size=default]/switch:size-4 group-data-[size=sm]/switch:size-3 data-checked:translate-x-[calc(100%-2px)] data-unchecked:translate-x-0 dark:data-checked:bg-primary-foreground dark:data-unchecked:bg-foreground"
        />
      </SwitchPrimitive.Root>
    </>
  );
}

export { Switch };
