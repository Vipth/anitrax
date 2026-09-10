import * as React from "react";
import {
  createFileRoute,
  Link,
  useCanGoBack,
  useRouter,
} from "@tanstack/react-router";
import { useQuery } from "@tanstack/react-query";
import {
  ArrowLeft,
  CalendarRange,
  ExternalLink,
  Film,
  HardDrive,
  Plus,
  Star,
  Tv,
} from "lucide-react";
import { openUrl } from "@tauri-apps/plugin-opener";
import { api } from "@/lib/ipc";
import { qk } from "@/lib/query";
import { Skeleton } from "@/components/ui/primitives";
import { Button } from "@/components/ui/button";
import { AiringBadge } from "@/components/media/AiringBadge";
import { Countdown } from "@/components/media/Countdown";
import { MediaPoster } from "@/components/media/MediaPoster";
import { PlayButton } from "@/components/media/PlayButton";
import { ProgressControl } from "@/components/media/ProgressControl";
import { EditEntryDialog } from "@/components/media/EditEntryDialog";
import { useAddEntry, useLibrary, useOwnedMedia } from "@/lib/hooks";
import {
  FORMAT_LABEL,
  STATUS_LABEL,
  episodeRanges,
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
  const router = useRouter();
  const canGoBack = useCanGoBack();

  const { data: media, isLoading } = useQuery({
    queryKey: qk.media(id),
    queryFn: () => api.getMedia(id),
  });
  const { data: library } = useLibrary();
  const { data: owned } = useOwnedMedia();
  const entry = library?.find((e) => e.media.id.id === id);
  const ownedEpisodes = owned?.get(id) ?? [];
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
        {canGoBack ? (
          <button
            onClick={() => router.history.back()}
            className="absolute left-4 top-4 inline-flex items-center gap-1.5 rounded-md bg-black/50 px-2.5 py-1.5 text-xs font-medium text-white backdrop-blur-sm hover:bg-black/70"
          >
            <ArrowLeft className="size-3.5" /> Back
          </button>
        ) : (
          <Link
            to="/"
            className="absolute left-4 top-4 inline-flex items-center gap-1.5 rounded-md bg-black/50 px-2.5 py-1.5 text-xs font-medium text-white backdrop-blur-sm hover:bg-black/70"
          >
            <ArrowLeft className="size-3.5" /> Library
          </Link>
        )}
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

            <div className="mt-2.5 flex flex-wrap items-center gap-x-3 gap-y-2">
              <AiringBadge status={media.airingStatus} variant="plain" />
              {media.nextAiring && (
                <Countdown
                  airingAt={media.nextAiring.airingAt}
                  episode={media.nextAiring.episode}
                />
              )}
            </div>

            <div className="mt-4 flex flex-wrap gap-2">
              <FactTile
                icon={<Tv className="size-3.5" />}
                value={FORMAT_LABEL[media.format]}
                label="Format"
              />
              {media.episodes != null && (
                <FactTile
                  icon={<Film className="size-3.5" />}
                  value={String(media.episodes)}
                  label={media.episodes === 1 ? "Episode" : "Episodes"}
                />
              )}
              {airedLabel(media) && (
                <FactTile
                  icon={<CalendarRange className="size-3.5" />}
                  value={airedLabel(media)!}
                  label="Aired"
                />
              )}
              {media.averageScore != null && (
                <FactTile
                  icon={
                    <Star className="size-3.5 fill-warning text-warning" />
                  }
                  value={`${media.averageScore}%`}
                  label="Rating"
                />
              )}
              {ownedEpisodes.length > 0 && (
                <FactTile
                  icon={<HardDrive className="size-3.5" />}
                  value={String(ownedEpisodes.length)}
                  label="On disk"
                  hint={`Ep ${episodeRanges(ownedEpisodes)}`}
                />
              )}
            </div>

            <div className="mt-3 flex flex-wrap items-center gap-1.5">
              {media.genres.map((g) => (
                <span
                  key={g}
                  className="rounded-full bg-border/50 px-2 py-0.5 text-[11px] text-muted-foreground"
                >
                  {g}
                </span>
              ))}
              {media.siteUrl && (
                <button
                  onClick={() => openUrl(media.siteUrl!)}
                  className="inline-flex items-center gap-1 rounded-full px-2 py-0.5 text-[11px] text-muted-foreground transition-colors hover:bg-border/50 hover:text-foreground"
                >
                  AniList <ExternalLink className="size-3" />
                </button>
              )}
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
                  <PlayButton entry={entry} owned={ownedEpisodes} variant="full" />
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
        <EditEntryDialog entry={entry} open={editing} onOpenChange={setEditing} />
      )}
    </div>
  );
}

function FactTile({
  icon,
  value,
  label,
  hint,
}: {
  icon: React.ReactNode;
  value: string;
  label: string;
  hint?: string;
}) {
  return (
    <div
      className="flex items-center gap-2.5 rounded-lg border border-border bg-surface-raised px-3 py-2"
      title={hint}
    >
      <span className="grid size-7 shrink-0 place-items-center rounded-md bg-border/50 text-muted-foreground">
        {icon}
      </span>
      <div className="min-w-0 leading-tight">
        <p className="truncate text-sm font-semibold">{value}</p>
        <p className="text-[10px] uppercase tracking-wide text-muted-foreground">
          {label}
        </p>
      </div>
    </div>
  );
}

/** "Summer 2026", or "2026" when the season is unknown. */
function airedLabel(media: {
  season: string | null;
  seasonYear: number | null;
  startDate: string | null;
}): string | null {
  const year =
    media.seasonYear ?? (Number(media.startDate?.slice(0, 4)) || null);
  if (!year) return null;
  if (media.season) {
    const s = media.season[0] + media.season.slice(1).toLowerCase();
    return `${s} ${year}`;
  }
  return String(year);
}
