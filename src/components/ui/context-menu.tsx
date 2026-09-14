import * as React from "react";
import * as ContextMenuPrimitive from "@radix-ui/react-context-menu";
import { cn } from "@/lib/utils";

export const ContextMenu = ContextMenuPrimitive.Root;
export const ContextMenuTrigger = ContextMenuPrimitive.Trigger;

export function ContextMenuContent({
  className,
  ...props
}: React.ComponentPropsWithoutRef<typeof ContextMenuPrimitive.Content>) {
  return (
    <ContextMenuPrimitive.Portal>
      <ContextMenuPrimitive.Content
        className={cn(
          "anim-popover z-50 min-w-[11rem] overflow-hidden rounded-md border border-border bg-surface-raised p-1 shadow-2xl shadow-black/30 outline-none",
          className,
        )}
        {...props}
      />
    </ContextMenuPrimitive.Portal>
  );
}

export function ContextMenuItem({
  className,
  danger = false,
  ...props
}: React.ComponentPropsWithoutRef<typeof ContextMenuPrimitive.Item> & {
  danger?: boolean;
}) {
  return (
    <ContextMenuPrimitive.Item
      className={cn(
        "flex cursor-pointer items-center gap-2 rounded-sm px-2 py-1.5 text-sm outline-none transition-colors",
        "data-[highlighted]:bg-border/50",
        danger ? "text-danger data-[highlighted]:bg-danger/10" : "text-foreground",
        "data-[disabled]:pointer-events-none data-[disabled]:opacity-40",
        className,
      )}
      {...props}
    />
  );
}

export function ContextMenuSeparator({
  className,
  ...props
}: React.ComponentPropsWithoutRef<typeof ContextMenuPrimitive.Separator>) {
  return (
    <ContextMenuPrimitive.Separator
      className={cn("-mx-1 my-1 h-px bg-border", className)}
      {...props}
    />
  );
}
