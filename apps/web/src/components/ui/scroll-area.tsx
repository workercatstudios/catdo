import * as React from "react";
import * as Primitive from "@radix-ui/react-scroll-area";
import { cn } from "@/lib/utils";
export function ScrollArea({
  children,
  className,
  ...props
}: React.ComponentProps<typeof Primitive.Root>) {
  return (
    <Primitive.Root className={cn("scroll-area", className)} {...props}>
      <Primitive.Viewport className="scroll-viewport">
        {children}
      </Primitive.Viewport>
      <Primitive.Scrollbar orientation="vertical" className="scroll-bar">
        <Primitive.Thumb className="scroll-thumb" />
      </Primitive.Scrollbar>
      <Primitive.Corner />
    </Primitive.Root>
  );
}
