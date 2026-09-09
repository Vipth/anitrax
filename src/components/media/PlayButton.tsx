import * as React from "react";
import { Play } from "lucide-react";
import { api } from "@/lib/ipc";
import { cn } from "@/lib/utils";
import { nextEpisodeOnDisk } from "@/lib/library";
import { toast } from "@/stores/toast";
import { errorMessage, type MediaListEntry } from "@/lib/types";

/**
 * Opens the next-to-watch episode in the OS default player, if that file is in
 * the local library. Renders nothing otherwise. Progress stays manual — hit the
 * +1 control when you've finished watching.
 */
export function PlayButton({
  entry,
  owned,
  variant = "overlay",
  className,
}: {
  entry: MediaListEntry;
  owned: number[] | undefined;
  variant?: "overlay" | "icon" | "full";
  className?: string;
}) {
  const ep = nextEpisodeOnDisk(entry, owned);
  const [busy, setBusy] = React.useState(false);
  if (ep == null) return null;

  const play = async (e: React.MouseEvent) => {
    e.preventDefault();
    e.stopPropagation();
    setBusy(true);
    try {
      await api.playEpisode(entry.media.id.id, ep, entry.media.id.service);
    } catch (err) {
      toast.error("Couldn't play", errorMessage(err));
    } finally {
      setBusy(false);
    }
  };

  const label = `Play episode ${ep}`;

  if (variant === "full") {
    return (
      <button
        onClick={play}
        disabled={busy}
        aria-label={label}
        className={cn(
          "inline-flex h-8 items-center gap-1.5 rounded-md bg-primary px-3 text-xs font-semibold text-primary-foreground transition-colors hover:bg-primary/90 disabled:opacity-50",
          className,
        )}
      >
        <Play className="size-3.5 fill-current" />
        Play Ep {ep}
      </button>
    );
  }

  if (variant === "icon") {
    return (
      <button
        onClick={play}
        disabled={busy}
        aria-label={label}
        title={label}
        className={cn(
          "grid size-7 shrink-0 place-items-center rounded-md text-muted-foreground transition-colors hover:bg-primary/10 hover:text-primary disabled:opacity-50",
          className,
        )}
      >
        <Play className="size-3.5 fill-current" />
      </button>
    );
  }

  // overlay — a frosted play chip centred on the poster, shown on hover
  return (
    <button
      onClick={play}
      disabled={busy}
      aria-label={label}
      title={label}
      className={cn(
        "absolute left-1/2 top-1/2 grid size-9 -translate-x-1/2 -translate-y-1/2 place-items-center rounded-full",
        "bg-background/85 text-foreground shadow-md ring-1 ring-border backdrop-blur-md",
        "opacity-0 transition-all duration-150 hover:scale-110 hover:bg-background",
        "focus-visible:opacity-100 group-hover:opacity-100 disabled:opacity-50",
        className,
      )}
    >
      <Play className="size-4 translate-x-px fill-current" />
    </button>
  );
}
