"use client";

import {
  createContext,
  useContext,
  useImperativeHandle,
  useLayoutEffect,
  useRef,
  useState,
  type CSSProperties,
} from "react";
import { Toggle as TogglePrimitive } from "@base-ui/react/toggle";
import { ToggleGroup as ToggleGroupPrimitive } from "@base-ui/react/toggle-group";
import { cn } from "@/lib/utils";

// Single (Base UI's default, no `multiple`): the thumb is the group's ::before. A layout effect measures the item that is on
// and writes its box into custom properties, so the thumb slides there on a springy transition
// without a React render, and squashes as it lands (two identical keyframes, flipped per change).
// Heading for the end item it can't overshoot (it would poke out), so it eases out (`data-wall`).
// `multiple`: each item pops on `scale` when pressed, 0.92 → 1.08 → 0.97 → 1.
const css = `
@layer theme, base, components, utilities;
@layer components {
:where(.kk-pop-toggle-group:not([data-multiple]))::before {
  content: "";
  position: absolute;
  top: 0;
  left: 0;
  z-index: -1;
  width: var(--kk-pop-toggle-group-w, 0px);
  height: var(--kk-pop-toggle-group-h, 0px);
  translate: var(--kk-pop-toggle-group-x, 0px) var(--kk-pop-toggle-group-y, 0px);
  opacity: 0;
  pointer-events: none;
}
:where(.kk-pop-toggle-group[data-thumb])::before {
  transition:
    translate 0.4s var(--kk-ease-spring, cubic-bezier(0.34, 1.56, 0.64, 1)),
    width 0.4s var(--kk-ease-spring, cubic-bezier(0.34, 1.56, 0.64, 1)),
    height 0.4s var(--kk-ease-spring, cubic-bezier(0.34, 1.56, 0.64, 1)),
    opacity 0.15s ease-out,
    scale 0.15s ease-out;
}
:where(.kk-pop-toggle-group[data-thumb][data-wall])::before {
  transition-timing-function: var(--kk-ease-out, cubic-bezier(0.05, 0.3, 0.1, 1));
}
:where(.kk-pop-toggle-group[data-thumb="off"])::before { scale: 0.85; }
:where(.kk-pop-toggle-group[data-thumb="on"])::before { opacity: 1; }
:where(.kk-pop-toggle-group[data-thumb="on"][data-land="a"])::before { animation: kk-pop-toggle-group-land-a 0.44s ease-in-out; }
:where(.kk-pop-toggle-group[data-thumb="on"][data-land="b"])::before { animation: kk-pop-toggle-group-land-b 0.44s ease-in-out; }
:where(.kk-pop-toggle-group-item[data-pop="a"]) { animation: kk-pop-toggle-group-pop-a 0.36s ease-in-out; }
:where(.kk-pop-toggle-group-item[data-pop="b"]) { animation: kk-pop-toggle-group-pop-b 0.36s ease-in-out; }
@keyframes kk-pop-toggle-group-land-a {
  0%, 100% { scale: 1; }
  30% { scale: 1.08 0.9; }
  60% { scale: 0.95 1.06; }
  82% { scale: 1.02 0.98; }
}
@keyframes kk-pop-toggle-group-land-b {
  0%, 100% { scale: 1; }
  30% { scale: 1.08 0.9; }
  60% { scale: 0.95 1.06; }
  82% { scale: 1.02 0.98; }
}
@keyframes kk-pop-toggle-group-pop-a {
  0% { scale: 0.92; }
  40% { scale: 1.08 1.1; }
  70% { scale: 0.97; }
  100% { scale: 1; }
}
@keyframes kk-pop-toggle-group-pop-b {
  0% { scale: 0.92; }
  40% { scale: 1.08 1.1; }
  70% { scale: 0.97; }
  100% { scale: 1; }
}
/* Reduced motion: the thumb jumps to the new item and nothing pops. */
@media (prefers-reduced-motion: reduce) {
  :where(.kk-pop-toggle-group[data-thumb])::before { transition: opacity 0.15s ease-out; animation: none; }
  :where(.kk-pop-toggle-group[data-thumb="off"])::before { scale: none; }
  :where(.kk-pop-toggle-group-item[data-pop]) { animation: none; }
}
}
`;

type Variant = "default" | "outline";
type Size = "default" | "sm" | "lg";

// shadcn's toggleVariants, written out so this file needs no class-variance-authority.
const sizes: Record<Size, string> = {
  default: "h-9 min-w-9 px-2",
  sm: "h-8 min-w-8 px-1.5",
  lg: "h-10 min-w-10 px-2.5",
};
// In a single group the thumb paints the "on" background, so items only fill on hover while off.
const fills: Record<"single" | "multiple", Record<Variant, string>> = {
  single: {
    default: "bg-transparent not-data-pressed:hover:bg-muted",
    outline:
      "border border-input bg-transparent shadow-xs hover:text-accent-foreground not-data-pressed:hover:bg-accent",
  },
  multiple: {
    default: "bg-transparent hover:bg-muted data-pressed:bg-accent",
    outline:
      "border border-input bg-transparent shadow-xs hover:bg-accent hover:text-accent-foreground data-pressed:bg-accent",
  },
};

const ToggleGroupContext = createContext<{
  variant?: Variant | null;
  size?: Size | null;
  spacing?: number;
  orientation?: "horizontal" | "vertical";
  multiple?: boolean;
}>({ size: "default", variant: "default", spacing: 0, orientation: "horizontal" });

function ToggleGroup({
  className,
  variant,
  size,
  spacing = 0,
  orientation = "horizontal",
  multiple = false,
  children,
  ref,
  ...props
}: ToggleGroupPrimitive.Props & {
  variant?: Variant | null;
  size?: Size | null;
  spacing?: number;
  orientation?: "horizontal" | "vertical";
}) {
  const group = useRef<HTMLDivElement>(null);
  useImperativeHandle(ref, () => group.current!, []);

  useLayoutEffect(() => {
    const root = group.current;
    if (!root || multiple) return;
    let current: HTMLElement | null = null;
    let last = { x: 0, y: 0 };
    const place = () => {
      const item = root.querySelector<HTMLElement>('[data-slot="toggle-group-item"][data-pressed]');
      if (!item) {
        // Nothing on: the thumb shrinks and fades where it is.
        if (root.dataset.thumb) root.dataset.thumb = "off";
        current = null;
        return;
      }
      const moved = item !== current;
      if (moved) {
        if (current) resize.unobserve(current);
        resize.observe(item);
        current = item;
      }
      // Hidden: keep the last placement until it has a size.
      if (item.offsetWidth === 0) return;
      if (root.dataset.thumb !== "on") {
        // Appearing: land in place with no slide (flush styles first), then fade in.
        delete root.dataset.thumb;
        delete root.dataset.land;
      }
      const x = item.offsetLeft;
      const y = item.offsetTop;
      const w = item.offsetWidth;
      const h = item.offsetHeight;
      const wall =
        (x > last.x && x + w >= root.clientWidth - 1) ||
        (x < last.x && x <= 1) ||
        (y > last.y && y + h >= root.clientHeight - 1) ||
        (y < last.y && y <= 1);
      root.toggleAttribute("data-wall", wall);
      root.style.setProperty("--kk-pop-toggle-group-x", `${x}px`);
      root.style.setProperty("--kk-pop-toggle-group-y", `${y}px`);
      root.style.setProperty("--kk-pop-toggle-group-w", `${w}px`);
      root.style.setProperty("--kk-pop-toggle-group-h", `${h}px`);
      last = { x, y };
      if (!root.dataset.thumb) {
        void root.offsetWidth;
        root.dataset.thumb = "on";
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
      attributeFilter: ["data-pressed"],
    });
    place();
    return () => {
      resize.disconnect();
      mutations.disconnect();
      delete root.dataset.thumb;
      delete root.dataset.land;
      delete root.dataset.wall;
    };
  }, [multiple]);

  return (
    <>
      <style href="kk-pop-toggle-group" precedence="kirakira">
        {css}
      </style>
      <ToggleGroupPrimitive
        ref={group}
        data-slot="toggle-group"
        data-variant={variant}
        data-size={size}
        data-spacing={spacing}
        orientation={orientation}
        multiple={multiple}
        style={{ "--gap": spacing } as CSSProperties}
        className={cn(
          "kk-pop-toggle-group group/toggle-group relative isolate flex w-fit flex-row items-center gap-[--spacing(var(--gap))] rounded-lg not-data-multiple:before:rounded-lg not-data-multiple:before:bg-accent data-[orientation=vertical]:flex-col data-[orientation=vertical]:items-stretch data-[spacing=default]:data-[variant=outline]:shadow-xs",
          className,
        )}
        {...props}
      >
        <ToggleGroupContext.Provider value={{ variant, size, spacing, orientation, multiple }}>
          {children}
        </ToggleGroupContext.Provider>
      </ToggleGroupPrimitive>
    </>
  );
}

function ToggleGroupItem({
  className,
  children,
  variant,
  size,
  onClick,
  ...props
}: TogglePrimitive.Props & {
  variant?: Variant | null;
  size?: Size | null;
}) {
  const context = useContext(ToggleGroupContext);
  const itemVariant = context.variant || variant || "default";
  const itemSize = context.size || size || "default";
  // Alternating between two identical keyframes restarts the pop on every press.
  const [pop, setPop] = useState<"a" | "b">();
  const handleClick: TogglePrimitive.Props["onClick"] = (event) => {
    if (context.multiple) setPop((current) => (current === "a" ? "b" : "a"));
    onClick?.(event);
  };

  return (
    <TogglePrimitive
      data-slot="toggle-group-item"
      data-variant={context.variant || variant}
      data-size={context.size || size}
      data-spacing={context.spacing}
      data-pop={pop}
      onClick={handleClick}
      className={cn(
        "kk-pop-toggle-group-item inline-flex cursor-pointer items-center justify-center gap-2 rounded-lg text-sm font-medium whitespace-nowrap transition-[color,background-color,box-shadow] duration-200 outline-none hover:text-muted-foreground focus-visible:border-ring focus-visible:ring-[3px] focus-visible:ring-ring/50 disabled:pointer-events-none disabled:opacity-50 aria-invalid:border-destructive aria-invalid:ring-destructive/20 data-pressed:text-accent-foreground dark:aria-invalid:ring-destructive/40 [&_svg]:pointer-events-none [&_svg]:shrink-0 [&_svg:not([class*='size-'])]:size-4",
        fills[context.multiple ? "multiple" : "single"][itemVariant],
        sizes[itemSize],
        "w-auto min-w-0 shrink-0 px-3 focus:z-10 focus-visible:z-10 data-[pop]:z-10",
        "data-[spacing=0]:rounded-none data-[spacing=0]:shadow-none",
        context.orientation === "vertical"
          ? "data-[spacing=0]:first:rounded-t-lg data-[spacing=0]:last:rounded-b-lg data-[spacing=0]:data-[variant=outline]:border-t-0 data-[spacing=0]:data-[variant=outline]:first:border-t"
          : "data-[spacing=0]:first:rounded-l-lg data-[spacing=0]:last:rounded-r-lg data-[spacing=0]:data-[variant=outline]:border-l-0 data-[spacing=0]:data-[variant=outline]:first:border-l",
        className,
      )}
      {...props}
    >
      {children}
    </TogglePrimitive>
  );
}

export { ToggleGroup, ToggleGroupItem };
