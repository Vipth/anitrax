import { Minus, Plus } from "lucide-react";
import { cn } from "@/lib/utils";
import type { MediaListEntry } from "@/lib/types";
import { useEditEntry } from "@/lib/hooks";

export function ProgressControl({
  entry,
  className,
  compact = false,
}: {
  entry: MediaListEntry;
  className?: string;
  compact?: boolean;
}) {
  const edit = useEditEntry();
  const total = entry.media.episodes;
  const atMax = total != null && entry.progress >= total;

  // Whether this also completes the show (reaching the final episode while
  // Watching) is decided backend-side in `edit_entry`, the single place every
  // progress-changing path agrees on that rule.
  const set = (next: number) => {
    if (next < 0) return;
    edit.mutate({
      mediaId: entry.media.id.id,
      remoteId: entry.remoteId,
      progress: next,
    });
  };

  return (
    <div className={cn("flex items-center gap-1.5", className)}>
      {!compact && (
        <button
          onClick={() => set(entry.progress - 1)}
          disabled={entry.progress <= 0}
          className="grid size-7 place-items-center rounded-md border border-border text-muted-foreground transition-colors hover:bg-border/40 disabled:opacity-40"
          aria-label="Decrease progress"
        >
          <Minus className="size-3.5" />
        </button>
      )}
      <span
        className={cn(
          "text-center text-sm tabular-nums",
          compact ? "min-w-[2.75rem]" : "min-w-[3.5rem]",
        )}
      >
        <span className="font-semibold">{entry.progress}</span>
        <span className="text-muted-foreground"> / {total ?? "?"}</span>
      </span>
      <button
        onClick={() => set(entry.progress + 1)}
        disabled={atMax}
        className="inline-flex h-7 items-center gap-1 rounded-md bg-primary px-2 text-xs font-semibold text-primary-foreground transition-colors hover:bg-primary/90 disabled:opacity-40"
        aria-label="Increase progress"
      >
        <Plus className="size-3.5" />
        {!compact && "Episode"}
      </button>
    </div>
  );
}
