"use client";

import { useState } from "react";
import { Button as ButtonPrimitive } from "@base-ui/react/button";
import { cva, type VariantProps } from "class-variance-authority";
import { cn } from "@/lib/utils";

// Pressing sinks the button evenly to 0.95 in 0.1 s; letting go pops it back past full size,
// 0.95 -> 1.05 -> 0.98 -> 1 in 0.3 s. Both are animations on `scale` (never a static scale), so
// shadcn's `transition-all` has nothing to transition and never holds the pop back. The release
// flips between two identical keyframes so the pop restarts on every click.
const css = `
@layer theme, base, components, utilities;
@layer components {
:where(.kk-pop-button[data-pop="a"]) { animation: kk-pop-button-a 0.3s ease-in-out; }
:where(.kk-pop-button[data-pop="b"]) { animation: kk-pop-button-b 0.3s ease-in-out; }
/* Held down: sink, even mid-pop. The pop starts from this same 0.95. */
:where(.kk-pop-button:active) { animation: kk-pop-button-press 0.1s ease-out forwards; }
@keyframes kk-pop-button-press {
  to { scale: 0.95; }
}
@keyframes kk-pop-button-a {
  0% { scale: 0.95; }
  45% { scale: 1.05; }
  75% { scale: 0.98; }
  100% { scale: 1; }
}
@keyframes kk-pop-button-b {
  0% { scale: 0.95; }
  45% { scale: 1.05; }
  75% { scale: 0.98; }
  100% { scale: 1; }
}
/* Reduced motion: no sink or pop. The colours still change on hover and press. */
@media (prefers-reduced-motion: reduce) {
  :where(.kk-pop-button[data-pop]),
  :where(.kk-pop-button:active) { animation: none; }
}
}
`;

// shadcn's buttonVariants, unchanged. The motion comes with <Button>.
const buttonVariants = cva(
  "inline-flex shrink-0 items-center justify-center gap-2 rounded-md text-sm font-medium whitespace-nowrap transition-all outline-none focus-visible:border-ring focus-visible:ring-[3px] focus-visible:ring-ring/50 disabled:pointer-events-none disabled:opacity-50 aria-invalid:border-destructive aria-invalid:ring-destructive/20 dark:aria-invalid:ring-destructive/40 [&_svg]:pointer-events-none [&_svg]:shrink-0 [&_svg:not([class*='size-'])]:size-4",
  {
    variants: {
      variant: {
        default: "bg-primary text-primary-foreground hover:bg-primary/90",
        destructive:
          "bg-destructive text-white hover:bg-destructive/90 focus-visible:ring-destructive/20 dark:bg-destructive/60 dark:focus-visible:ring-destructive/40",
        outline:
          "border bg-background shadow-xs hover:bg-accent hover:text-accent-foreground dark:border-input dark:bg-input/30 dark:hover:bg-input/50",
        secondary: "bg-secondary text-secondary-foreground hover:bg-secondary/80",
        ghost: "hover:bg-accent hover:text-accent-foreground dark:hover:bg-accent/50",
        link: "text-primary underline-offset-4 hover:underline",
      },
      size: {
        default: "h-9 px-4 py-2 has-[>svg]:px-3",
        xs: "h-6 gap-1 rounded-md px-2 text-xs has-[>svg]:px-1.5 [&_svg:not([class*='size-'])]:size-3",
        sm: "h-8 gap-1.5 rounded-md px-3 has-[>svg]:px-2.5",
        lg: "h-10 rounded-md px-6 has-[>svg]:px-4",
        icon: "size-9",
        "icon-xs": "size-6 rounded-md [&_svg:not([class*='size-'])]:size-3",
        "icon-sm": "size-8",
        "icon-lg": "size-10",
      },
    },
    defaultVariants: {
      variant: "default",
      size: "default",
    },
  },
);

/** `render` swaps the `<button>` for another element (a link, say) with the same styles and motion. */
function Button({
  className,
  variant = "default",
  size = "default",
  onClick,
  ...props
}: ButtonPrimitive.Props & VariantProps<typeof buttonVariants>) {
  const [pop, setPop] = useState<"a" | "b">();

  return (
    <>
      <style href="kk-pop-button" precedence="kirakira">
        {css}
      </style>
      <ButtonPrimitive
        data-slot="button"
        data-variant={variant}
        data-size={size}
        data-pop={pop}
        className={cn("kk-pop-button", buttonVariants({ variant, size, className }))}
        onClick={(event) => {
          setPop((current) => (current === "a" ? "b" : "a"));
          onClick?.(event);
        }}
        {...props}
      />
    </>
  );
}

export { Button, buttonVariants };
