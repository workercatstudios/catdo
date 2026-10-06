"use client";

import { useImperativeHandle, useLayoutEffect, useState } from "react";
import { Tabs as TabsPrimitive } from "@base-ui/react/tabs";
import { cva, type VariantProps } from "class-variance-authority";
import { cn } from "@/lib/utils";

// The pill is the list's ::before. A layout effect measures the active trigger and writes its box
// into custom properties on the list, so the pill follows on a springy `translate`/`width`/`height`
// transition without a React render. Heading for a tab at the end of the list it can't overshoot
// (it would poke out of the track), so it eases out instead (`data-wall`). Each change flips
// `data-land` between two identical keyframes, restarting a small squash on `scale`: stretched
// along the travel, squashed as it lands, settled.
const css = `
@layer theme, base, components, utilities;
@layer components {
:where(.kk-pop-tabs-list) {
  --kk-pop-tabs-stretch: 1.06 0.92;
  --kk-pop-tabs-squash: 0.96 1.06;
  --kk-pop-tabs-rebound: 1.015 0.985;
}
:where(.kk-pop-tabs-list[data-orientation="vertical"]) {
  --kk-pop-tabs-stretch: 0.97 1.1;
  --kk-pop-tabs-squash: 1.02 0.92;
  --kk-pop-tabs-rebound: 0.995 1.03;
}
:where(.kk-pop-tabs-list)::before {
  content: "";
  position: absolute;
  top: 0;
  left: 0;
  z-index: -1;
  width: var(--kk-pop-tabs-w, 0px);
  height: var(--kk-pop-tabs-h, 0px);
  translate: var(--kk-pop-tabs-x, 0px) var(--kk-pop-tabs-y, 0px);
  opacity: 0;
  pointer-events: none;
}
:where(.kk-pop-tabs-list[data-pill])::before {
  transition:
    translate 0.42s var(--kk-ease-spring, cubic-bezier(0.34, 1.56, 0.64, 1)),
    width 0.42s var(--kk-ease-spring, cubic-bezier(0.34, 1.56, 0.64, 1)),
    height 0.42s var(--kk-ease-spring, cubic-bezier(0.34, 1.56, 0.64, 1)),
    opacity 0.15s ease-out;
}
:where(.kk-pop-tabs-list[data-pill][data-wall])::before {
  transition-timing-function: var(--kk-ease-out, cubic-bezier(0.05, 0.3, 0.1, 1));
}
:where(.kk-pop-tabs-list[data-pill="on"])::before { opacity: 1; }
:where(.kk-pop-tabs-list[data-land="a"])::before { animation: kk-pop-tabs-land-a 0.46s ease-in-out; }
:where(.kk-pop-tabs-list[data-land="b"])::before { animation: kk-pop-tabs-land-b 0.46s ease-in-out; }
/* The line variant draws the pill as a bar under (or beside) the active trigger. */
:where(.kk-pop-tabs-list[data-variant="line"][data-orientation="horizontal"])::before {
  height: 2px;
  translate: var(--kk-pop-tabs-x, 0px) calc(var(--kk-pop-tabs-y, 0px) + var(--kk-pop-tabs-h, 0px) + 3px);
}
:where(.kk-pop-tabs-list[data-variant="line"][data-orientation="vertical"])::before {
  width: 2px;
  translate: calc(var(--kk-pop-tabs-x, 0px) + var(--kk-pop-tabs-w, 0px) + 2px) var(--kk-pop-tabs-y, 0px);
}
/* Only after a switch: the panel open on first render has no activation direction yet. */
:where(.kk-pop-tabs-content:not([data-activation-direction="none"])) {
  animation: kk-pop-tabs-content 0.25s var(--kk-ease-out, cubic-bezier(0.05, 0.3, 0.1, 1)) both;
}
/* The panel being left goes at once, as the new one comes in. */
:where(.kk-pop-tabs-content[data-ending-style]) { display: none; }
@keyframes kk-pop-tabs-land-a {
  0%, 100% { scale: 1; }
  30% { scale: var(--kk-pop-tabs-stretch); }
  60% { scale: var(--kk-pop-tabs-squash); }
  82% { scale: var(--kk-pop-tabs-rebound); }
}
@keyframes kk-pop-tabs-land-b {
  0%, 100% { scale: 1; }
  30% { scale: var(--kk-pop-tabs-stretch); }
  60% { scale: var(--kk-pop-tabs-squash); }
  82% { scale: var(--kk-pop-tabs-rebound); }
}
@keyframes kk-pop-tabs-content {
  from { opacity: 0; translate: 0 0.5rem; }
}
@keyframes kk-pop-tabs-fade {
  from { opacity: 0; }
}
/* Reduced motion: the pill jumps to the new tab and the panel fades in. */
@media (prefers-reduced-motion: reduce) {
  :where(.kk-pop-tabs-list[data-pill])::before { transition: opacity 0.15s ease-out; animation: none; }
  :where(.kk-pop-tabs-content:not([data-activation-direction="none"])) {
    animation: kk-pop-tabs-fade 0.2s ease-out both;
  }
}
}
`;

function Tabs({ className, orientation = "horizontal", ...props }: TabsPrimitive.Root.Props) {
  return (
    <TabsPrimitive.Root
      data-slot="tabs"
      data-orientation={orientation}
      orientation={orientation}
      className={cn("group/tabs flex gap-2 data-[orientation=horizontal]:flex-col", className)}
      {...props}
    />
  );
}

const tabsListVariants = cva(
  "group/tabs-list relative isolate inline-flex w-fit items-center justify-center p-[3px] text-muted-foreground group-data-[orientation=horizontal]/tabs:h-9 group-data-[orientation=vertical]/tabs:h-fit group-data-[orientation=vertical]/tabs:flex-col",
  {
    variants: {
      variant: {
        default:
          "rounded-full bg-muted before:rounded-full before:bg-background before:shadow-sm group-data-[orientation=vertical]/tabs:rounded-2xl group-data-[orientation=vertical]/tabs:before:rounded-xl dark:before:border dark:before:border-input dark:before:bg-input/30",
        line: "gap-1 rounded-none bg-transparent before:rounded-full before:bg-foreground",
      },
    },
    defaultVariants: {
      variant: "default",
    },
  },
);

function TabsList({
  className,
  variant = "default",
  ref,
  ...props
}: TabsPrimitive.List.Props & VariantProps<typeof tabsListVariants>) {
  // The node lives in state: with `render`, swapping the element swaps the node, and the ref
  // handle and observers must follow it.
  const [root, setRoot] = useState<HTMLDivElement | null>(null);
  useImperativeHandle(ref, () => root as HTMLDivElement, [root]);

  useLayoutEffect(() => {
    if (!root) return;
    let current: HTMLElement | null = null;
    let last = { x: 0, y: 0 };
    const place = () => {
      const tab = root.querySelector<HTMLElement>('[role="tab"][data-active]');
      const moved = tab !== current && current !== null;
      if (tab !== current) {
        if (current) resize.unobserve(current);
        if (tab) resize.observe(tab);
        current = tab;
      }
      // Hidden (inside a closed panel, say): keep the last placement until it has a size.
      if (tab && tab.offsetWidth === 0) return;
      if (!tab) {
        delete root.dataset.pill;
        return;
      }
      const x = tab.offsetLeft;
      const y = tab.offsetTop;
      const w = tab.offsetWidth;
      const h = tab.offsetHeight;
      const wall =
        (x > last.x && x + w >= root.clientWidth - 4) ||
        (x < last.x && x <= 4) ||
        (y > last.y && y + h >= root.clientHeight - 4) ||
        (y < last.y && y <= 4);
      root.toggleAttribute("data-wall", wall);
      root.style.setProperty("--kk-pop-tabs-x", `${x}px`);
      root.style.setProperty("--kk-pop-tabs-y", `${y}px`);
      root.style.setProperty("--kk-pop-tabs-w", `${w}px`);
      root.style.setProperty("--kk-pop-tabs-h", `${h}px`);
      last = { x, y };
      if (root.dataset.pill !== "on") {
        // First placement: land in place with no transition (flush styles), then fade in.
        void root.offsetWidth;
        root.dataset.pill = "on";
      } else if (moved) {
        root.dataset.land = root.dataset.land === "a" ? "b" : "a";
      }
    };
    const resize = new ResizeObserver(place);
    resize.observe(root);
    const mutations = new MutationObserver(place);
    mutations.observe(root, {
      subtree: true,
      childList: true,
      attributes: true,
      attributeFilter: ["data-active"],
    });
    place();
    return () => {
      resize.disconnect();
      mutations.disconnect();
    };
  }, [root]);

  return (
    <>
      <style href="kk-pop-tabs" precedence="kirakira">
        {css}
      </style>
      <TabsPrimitive.List
        ref={setRoot}
        data-slot="tabs-list"
        data-variant={variant ?? "default"}
        className={cn("kk-pop-tabs-list", tabsListVariants({ variant }), className)}
        {...props}
      />
    </>
  );
}

function TabsTrigger({ className, ...props }: TabsPrimitive.Tab.Props) {
  return (
    <TabsPrimitive.Tab
      data-slot="tabs-trigger"
      className={cn(
        "relative inline-flex h-[calc(100%-1px)] flex-1 cursor-pointer items-center justify-center gap-1.5 rounded-full border border-transparent px-2 py-1 text-sm font-medium whitespace-nowrap text-foreground/60 transition-[color,box-shadow] duration-200 group-data-[orientation=vertical]/tabs:w-full group-data-[orientation=vertical]/tabs:justify-start group-data-[orientation=vertical]/tabs:rounded-xl hover:text-foreground focus-visible:border-ring focus-visible:ring-[3px] focus-visible:ring-ring/50 focus-visible:outline-1 focus-visible:outline-ring disabled:pointer-events-none disabled:opacity-50 data-active:text-foreground aria-disabled:pointer-events-none aria-disabled:opacity-50 dark:text-muted-foreground dark:hover:text-foreground dark:data-active:text-foreground [&_svg]:pointer-events-none [&_svg]:shrink-0 [&_svg:not([class*='size-'])]:size-4",
        className,
      )}
      {...props}
    />
  );
}

function TabsContent({ className, ...props }: TabsPrimitive.Panel.Props) {
  return (
    <TabsPrimitive.Panel
      data-slot="tabs-content"
      className={cn("kk-pop-tabs-content flex-1 outline-none", className)}
      {...props}
    />
  );
}

export { Tabs, TabsList, TabsTrigger, TabsContent, tabsListVariants };
