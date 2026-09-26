import * as DialogPrimitive from "@radix-ui/react-dialog";
import { X } from "lucide-react";

export function ImageLightbox({
  src,
  alt,
  open,
  onOpenChange,
}: {
  src: string;
  alt: string;
  open: boolean;
  onOpenChange: (v: boolean) => void;
}) {
  return (
    <DialogPrimitive.Root open={open} onOpenChange={onOpenChange}>
      <DialogPrimitive.Portal>
        <DialogPrimitive.Overlay className="anim-overlay fixed inset-0 z-50 bg-black/80 backdrop-blur-sm" />
        <DialogPrimitive.Content
          className="fixed left-1/2 top-1/2 z-50 flex max-h-[90vh] max-w-[90vw] -translate-x-1/2 -translate-y-1/2 flex-col items-center justify-center outline-none"
          aria-describedby={undefined}
          onClick={() => onOpenChange(false)}
        >
          <DialogPrimitive.Title className="sr-only">{alt}</DialogPrimitive.Title>
          <img
            src={src}
            alt={alt}
            onClick={(e) => e.stopPropagation()}
            className="max-h-[90vh] max-w-[90vw] rounded-lg object-contain shadow-2xl"
          />
          <DialogPrimitive.Close
            className="fixed right-4 top-4 z-[60] rounded-full bg-black/50 p-1.5 text-white backdrop-blur-sm transition-colors hover:bg-black/70"
            aria-label="Close"
          >
            <X className="size-5" />
          </DialogPrimitive.Close>
        </DialogPrimitive.Content>
      </DialogPrimitive.Portal>
    </DialogPrimitive.Root>
  );
}
