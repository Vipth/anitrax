import type * as React from "react";
import { cn } from "@/lib/utils";

/** A single keycap, with a subtle "physical key" bottom shadow. */
export function Kbd({
  className,
  ...props
}: React.HTMLAttributes<HTMLElement>) {
  return (
    <kbd
      className={cn(
        "inline-grid h-[22px] min-w-[22px] place-items-center rounded-[6px] border border-border-strong/50 bg-surface px-1.5 font-sans text-[11px] font-medium leading-none text-muted-foreground shadow-[inset_0_-1px_0_hsl(var(--border-strong)/0.6)]",
        className,
      )}
      {...props}
    />
  );
}
