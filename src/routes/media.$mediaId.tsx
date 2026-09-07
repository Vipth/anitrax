import * as React from "react";
import { createFileRoute, Link } from "@tanstack/react-router";
import { useQuery } from "@tanstack/react-query";
import { ArrowLeft, ExternalLink, Plus, Star } from "lucide-react";
import { openUrl } from "@tauri-apps/plugin-opener";
import { api } from "@/lib/ipc";
import { qk } from "@/lib/query";
import { Skeleton } from "@/components/ui/primitives";
import { Button } from "@/components/ui/button";
import { MediaPoster } from "@/components/media/MediaPoster";
import { ProgressControl } from "@/components/media/ProgressControl";
import { EditEntrySheet } from "@/components/media/EditEntrySheet";
import { useAddEntry, useLibrary } from "@/lib/hooks";
import {
  FORMAT_LABEL,
  STATUS_LABEL,
  countdown,
  mediaTitle,
  scoreToTen,
  stripHtml,
} from "@/lib/format";
import { toast } from "@/stores/toast";
import { errorMessage } from "@/lib/types";

export const Route = createFileRoute("/media/$mediaId")({
  component: MediaDetailPage,
});

function MediaDetailPage() {
  const { mediaId } = Route.useParams();
  const id = Number(mediaId);
  const [editing, setEditing] = React.useState(false);

  const { data: media, isLoading } = useQuery({
    queryKey: qk.media(id),
    queryFn: () => api.getMedia(id),
  });
  const { data: library } = useLibrary();
  const entry = library?.find((e) => e.media.id.id === id);
  const add = useAddEntry();

  if (isLoading) {
    return (
      <div className="p-6">
        <Skeleton className="h-48 w-full rounded-xl" />
        <div className="mt-4 flex gap-6">
          <Skeleton className="h-64 w-44 rounded-lg" />
          <div className="flex-1 space-y-3">
            <Skeleton className="h-7 w-2/3" />
            <Skeleton className="h-4 w-full" />
            <Skeleton className="h-4 w-5/6" />
          </div>
        </div>
      </div>
    );
  }

  if (!media) {
    return (
      <div className="p-10 text-center text-sm text-muted-foreground">
        Couldn&apos;t load this title.
      </div>
    );
  }

  const handleAdd = () =>
    add.mutate(
      { mediaId: id, status: "PLANNING", progress: 0 },
      {
        onSuccess: () => toast.success("Added to list", "Planning"),
        onError: (e) => toast.error("Couldn't add", errorMessage(e)),
      },
    );

  return (
    <div className="pb-10">
      <div className="relative h-52 w-full overflow-hidden bg-border/40">
        {media.bannerUrl && (
          <img
            src={media.bannerUrl}
            alt=""
            className="size-full object-cover"
          />
        )}
        <div className="absolute inset-0 bg-gradient-to-t from-background via-background/40 to-background/10" />
        <Link
          to="/"
          className="absolute left-4 top-4 inline-flex items-center gap-1.5 rounded-md bg-black/50 px-2.5 py-1.5 text-xs font-medium text-white backdrop-blur-sm hover:bg-black/70"
        >
          <ArrowLeft className="size-3.5" /> Library
        </Link>
      </div>

      <div className="mx-auto -mt-24 max-w-[1000px] px-6">
        <div className="flex flex-col gap-6 sm:flex-row">
          <MediaPoster
            media={media}
            className="h-64 w-44 shrink-0 rounded-lg border border-border shadow-xl"
          />

          <div className="flex-1 pt-24 sm:pt-28">
            <h1 className="text-2xl font-bold leading-tight">
              {mediaTitle(media)}
            </h1>
            {media.title.native && (
              <p className="mt-1 text-sm text-muted-foreground">
                {media.title.native}
              </p>
            )}

            <div className="mt-3 flex flex-wrap items-center gap-x-3 gap-y-1 text-xs text-muted-foreground">
              <span>{FORMAT_LABEL[media.format]}</span>
              {media.episodes && <span>· {media.episodes} episodes</span>}
              {media.seasonYear && (
                <span>
                  · {media.season ? media.season[0] + media.season.slice(1).toLowerCase() + " " : ""}
                  {media.seasonYear}
                </span>
              )}
              {media.averageScore != null && (
                <span className="inline-flex items-center gap-0.5">
                  · <Star className="size-3 fill-warning text-warning" />
                  {media.averageScore}%
                </span>
              )}
              {media.siteUrl && (
                <button
                  onClick={() => openUrl(media.siteUrl!)}
                  className="inline-flex items-center gap-1 hover:text-foreground"
                >
                  · AniList <ExternalLink className="size-3" />
                </button>
              )}
            </div>

            <div className="mt-4 flex flex-wrap gap-1.5">
              {media.genres.map((g) => (
                <span
                  key={g}
                  className="rounded-full bg-border/50 px-2 py-0.5 text-[11px] text-muted-foreground"
                >
                  {g}
                </span>
              ))}
            </div>

            <div className="mt-5">
              {entry ? (
                <div className="flex flex-wrap items-center gap-3 rounded-lg border border-border bg-surface-raised p-3">
                  <span className="rounded-md bg-primary/10 px-2 py-1 text-xs font-medium text-primary">
                    {STATUS_LABEL[entry.status]}
                  </span>
                  <ProgressControl entry={entry} />
                  {entry.scoreRaw > 0 && (
                    <span className="inline-flex items-center gap-0.5 text-xs font-semibold text-muted-foreground">
                      <Star className="size-3 fill-warning text-warning" />
                      {scoreToTen(entry.scoreRaw)}
                    </span>
                  )}
                  <Button
                    variant="secondary"
                    size="sm"
                    onClick={() => setEditing(true)}
                  >
                    Edit
                  </Button>
                </div>
              ) : (
                <Button onClick={handleAdd} disabled={add.isPending}>
                  <Plus className="size-4" /> Add to list
                </Button>
              )}
            </div>

            {media.nextAiring && (
              <p className="mt-3 text-xs font-medium text-warning">
                Episode {media.nextAiring.episode} airs in{" "}
                {countdown(media.nextAiring.airingAt)}
              </p>
            )}
          </div>
        </div>

        {media.description && (
          <div className="mt-8 max-w-3xl">
            <h2 className="mb-2 text-sm font-semibold">Synopsis</h2>
            <p className="whitespace-pre-line text-sm leading-relaxed text-muted-foreground">
              {stripHtml(media.description)}
            </p>
          </div>
        )}
      </div>

      {entry && (
        <EditEntrySheet entry={entry} open={editing} onOpenChange={setEditing} />
      )}
    </div>
  );
}
