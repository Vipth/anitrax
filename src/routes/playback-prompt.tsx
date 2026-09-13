import * as React from "react";
import { createFileRoute } from "@tanstack/react-router";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { emit } from "@tauri-apps/api/event";
import { Check, X } from "lucide-react";
import { api } from "@/lib/ipc";
import { useLibrary } from "@/lib/hooks";
import type { ListStatus } from "@/lib/types";

export const Route = createFileRoute("/playback-prompt")({
  component: PlaybackPromptPage,
});

/** Written into `window` by `show_playback_popup` (lib.rs) via an
 * initialization script, before this page's own JS runs. */
interface PopupData {
  service: string;
  mediaId: number;
  episode: number;
  title: string;
  episodesTotal: number | null;
}

// A window this small doesn't get to close itself indefinitely.
const AUTO_DISMISS_MS = 30_000;

function PlaybackPromptPage() {
  const [data] = React.useState<PopupData | null>(
    () => (window as unknown as { __playbackPopup?: PopupData }).__playbackPopup ?? null,
  );
  const { data: library } = useLibrary();
  const [busy, setBusy] = React.useState(false);

  React.useEffect(() => {
    const id = setTimeout(() => {
      getCurrentWindow().close();
    }, AUTO_DISMISS_MS);
    return () => clearTimeout(id);
  }, []);

  if (!data) return null;

  const entry = library?.find((e) => e.media.id.id === data.mediaId);

  const bump = async () => {
    setBusy(true);
    const status: ListStatus | undefined =
      data.episodesTotal === data.episode && entry?.status === "CURRENT"
        ? "COMPLETED"
        : undefined;
    try {
      await api.editEntry({
        mediaId: data.mediaId,
        remoteId: entry?.remoteId,
        progress: data.episode,
        status,
      });
      await emit("entries-updated");
    } finally {
      getCurrentWindow().close();
    }
  };

  return (
    <div className="flex h-screen flex-col justify-between bg-surface-raised p-3">
      <div className="min-w-0">
        <p className="text-sm font-medium">Finished episode {data.episode}?</p>
        <p className="mt-0.5 line-clamp-2 text-xs text-muted-foreground">{data.title}</p>
      </div>
      <div className="flex justify-end gap-2">
        <button
          onClick={() => getCurrentWindow().close()}
          className="inline-flex items-center gap-1 rounded-md border border-border px-2.5 py-1.5 text-xs font-medium text-muted-foreground transition-colors hover:bg-border/40"
        >
          <X className="size-3.5" /> Not yet
        </button>
        <button
          onClick={bump}
          disabled={busy}
          className="inline-flex items-center gap-1 rounded-md bg-primary px-2.5 py-1.5 text-xs font-semibold text-primary-foreground transition-colors hover:bg-primary/90 disabled:opacity-50"
        >
          <Check className="size-3.5" /> Bump progress
        </button>
      </div>
    </div>
  );
}
