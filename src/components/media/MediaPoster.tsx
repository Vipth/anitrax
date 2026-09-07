import { cn } from "@/lib/utils";
import type { Media } from "@/lib/types";
import { mediaTitle } from "@/lib/format";

export function MediaPoster({
  media,
  className,
}: {
  media: Media;
  className?: string;
}) {
  return (
    <div
      className={cn(
        "relative overflow-hidden bg-border/40",
        className,
      )}
      style={
        media.coverColor
          ? { backgroundColor: media.coverColor + "33" }
          : undefined
      }
    >
      {media.coverUrl ? (
        <img
          src={media.coverUrl}
          alt={mediaTitle(media)}
          loading="lazy"
          className="size-full object-cover"
          draggable={false}
        />
      ) : (
        <div className="flex size-full items-center justify-center p-2 text-center text-xs text-muted-foreground">
          {mediaTitle(media)}
        </div>
      )}
    </div>
  );
}
