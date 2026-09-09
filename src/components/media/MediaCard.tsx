import { Link } from "@tanstack/react-router";
import { HardDrive, Pencil, Star, Tv } from "lucide-react";
import { cn } from "@/lib/utils";
import type { MediaListEntry } from "@/lib/types";
import { FORMAT_LABEL, countdown, mediaTitle, scoreToTen } from "@/lib/format";
import { nextEpisodeOnDisk } from "@/lib/library";
import { AiringBadge } from "./AiringBadge";
import { PlayButton } from "./PlayButton";
import { ProgressControl } from "./ProgressControl";
import { MediaPoster } from "./MediaPoster";

interface CardProps {
  entry: MediaListEntry;
  selected?: boolean;
  owned?: number[];
  onEdit: (entry: MediaListEntry) => void;
}

/** Small "N episodes on disk" pill; tinted when the next unwatched one is ready. */
export function OwnedBadge({
  entry,
  owned,
  className,
}: {
  entry: MediaListEntry;
  owned?: number[];
  className?: string;
}) {
  if (!owned || owned.length === 0) return null;
  const nextReady = nextEpisodeOnDisk(entry, owned) != null;
  return (
    <span
      className={cn(
        "inline-flex items-center gap-1 rounded-md px-1.5 py-0.5 text-[11px] font-medium",
        nextReady
          ? "bg-primary/15 text-primary"
          : "bg-border/50 text-muted-foreground",
        className,
      )}
      title={
        nextReady
          ? `Episode ${entry.progress + 1} is on disk`
          : `${owned.length} episode${owned.length === 1 ? "" : "s"} on disk`
      }
    >
      <HardDrive className="size-3" />
      {owned.length}
    </span>
  );
}

function ProgressBar({ entry }: { entry: MediaListEntry }) {
  const total = entry.media.episodes ?? 0;
  const pct = total > 0 ? Math.min(100, (entry.progress / total) * 100) : 0;
  const behind =
    entry.media.nextAiring != null
      ? Math.max(0, entry.media.nextAiring.episode - 1 - entry.progress)
      : 0;
  return (
    <div className="space-y-1">
      <div className="h-1 overflow-hidden rounded-full bg-border/70">
        <div
          className={cn(
            "h-full rounded-full transition-all",
            behind > 0 ? "bg-warning" : "bg-primary",
          )}
          style={{ width: `${pct}%` }}
        />
      </div>
      {behind > 0 && (
        <p className="text-[11px] font-medium text-warning">
          {behind} episode{behind > 1 ? "s" : ""} behind
        </p>
      )}
    </div>
  );
}

export function MediaCard({ entry, selected, owned, onEdit }: CardProps) {
  const ten = scoreToTen(entry.scoreRaw);

  return (
    <div
      data-entry-id={entry.media.id.id}
      className="group relative flex scroll-mt-24 flex-col gap-2"
    >
      <div
        className={cn(
          "relative aspect-[2/3] overflow-hidden rounded-lg border transition-shadow",
          selected
            ? "border-primary ring-2 ring-primary ring-offset-2 ring-offset-background"
            : "border-border",
        )}
      >
        <Link
          to="/media/$mediaId"
          params={{ mediaId: String(entry.media.id.id) }}
          className="block size-full"
        >
          <MediaPoster media={entry.media} className="size-full" />
        </Link>

        <div className="absolute left-1.5 top-1.5 flex flex-col items-start gap-1">
          <AiringBadge status={entry.media.airingStatus} />
          {entry.scoreRaw > 0 && (
            <span className="inline-flex items-center gap-0.5 rounded-md bg-black/70 px-1.5 py-0.5 text-[11px] font-semibold text-white backdrop-blur-sm">
              <Star className="size-3 fill-warning text-warning" />
              {ten}
            </span>
          )}
        </div>
        {entry.dirty && (
          <span
            className="absolute right-1.5 top-1.5 size-2 rounded-full bg-primary ring-2 ring-black/40"
            title="Not yet synced"
          />
        )}

        {entry.media.nextAiring && (
          <span className="absolute inset-x-0 bottom-0 bg-gradient-to-t from-black/80 to-transparent px-2 pb-1.5 pt-4 text-[11px] font-medium text-white">
            Ep {entry.media.nextAiring.episode} in{" "}
            {countdown(entry.media.nextAiring.airingAt)}
          </span>
        )}

        <PlayButton entry={entry} owned={owned} />

        <button
          onClick={() => onEdit(entry)}
          className="absolute bottom-1.5 right-1.5 grid size-7 place-items-center rounded-md bg-black/70 text-white opacity-0 backdrop-blur-sm transition-opacity hover:bg-black/90 focus-visible:opacity-100 group-hover:opacity-100"
          aria-label="Edit entry"
        >
          <Pencil className="size-3.5" />
        </button>
      </div>

      <div className="min-w-0 space-y-1.5">
        <Link
          to="/media/$mediaId"
          params={{ mediaId: String(entry.media.id.id) }}
          className={cn(
            "line-clamp-2 text-sm font-medium leading-tight hover:text-primary",
            selected && "text-primary",
          )}
          title={mediaTitle(entry.media)}
        >
          {mediaTitle(entry.media)}
        </Link>
        <ProgressBar entry={entry} />
        <div className="flex items-center justify-between gap-1">
          <span className="inline-flex min-w-0 items-center gap-1 text-[11px] text-muted-foreground">
            <Tv className="size-3 shrink-0" />
            <span className="truncate">
              {FORMAT_LABEL[entry.media.format]}
              {entry.media.seasonYear ? ` · ${entry.media.seasonYear}` : ""}
            </span>
          </span>
          <div className="flex shrink-0 items-center gap-1">
            <OwnedBadge entry={entry} owned={owned} />
            <ProgressControl entry={entry} compact />
          </div>
        </div>
      </div>
    </div>
  );
}

export function MediaListRow({ entry, selected, owned, onEdit }: CardProps) {
  const ten = scoreToTen(entry.scoreRaw);
  const total = entry.media.episodes ?? 0;
  const pct = total > 0 ? Math.min(100, (entry.progress / total) * 100) : 0;

  return (
    <div
      data-entry-id={entry.media.id.id}
      className={cn(
        "group flex scroll-mt-24 items-center gap-3 rounded-lg border bg-surface-raised px-3 py-2 transition-colors",
        selected
          ? "border-primary ring-1 ring-primary"
          : "border-border",
      )}
    >
      <Link
        to="/media/$mediaId"
        params={{ mediaId: String(entry.media.id.id) }}
        className="shrink-0"
      >
        <MediaPoster media={entry.media} className="h-14 w-10 rounded-md" />
      </Link>
      <div className="min-w-0 flex-1">
        <Link
          to="/media/$mediaId"
          params={{ mediaId: String(entry.media.id.id) }}
          className={cn(
            "line-clamp-1 text-sm font-medium hover:text-primary",
            selected && "text-primary",
          )}
        >
          {mediaTitle(entry.media)}
        </Link>
        <div className="mt-1 flex items-center gap-2">
          <div className="h-1 w-32 overflow-hidden rounded-full bg-border/70">
            <div
              className="h-full rounded-full bg-primary"
              style={{ width: `${pct}%` }}
            />
          </div>
          <span className="text-[11px] text-muted-foreground">
            {FORMAT_LABEL[entry.media.format]}
            {entry.media.seasonYear ? ` · ${entry.media.seasonYear}` : ""}
          </span>
          <AiringBadge
            status={entry.media.airingStatus}
            variant="plain"
            className="px-1 py-0"
          />
        </div>
      </div>

      <OwnedBadge entry={entry} owned={owned} />
      {entry.scoreRaw > 0 && (
        <span className="inline-flex items-center gap-0.5 text-xs font-semibold text-muted-foreground">
          <Star className="size-3 fill-warning text-warning" />
          {ten}
        </span>
      )}
      <PlayButton entry={entry} owned={owned} variant="icon" />
      <ProgressControl entry={entry} />
      <button
        onClick={() => onEdit(entry)}
        className="grid size-7 place-items-center rounded-md text-muted-foreground opacity-0 transition-opacity hover:bg-border/40 hover:text-foreground focus-visible:opacity-100 group-hover:opacity-100"
        aria-label="Edit entry"
      >
        <Pencil className="size-3.5" />
      </button>
    </div>
  );
}
