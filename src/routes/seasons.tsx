import * as React from "react";
import { createFileRoute, Link } from "@tanstack/react-router";
import { Check, CloudOff, Plus } from "lucide-react";
import { Segmented, Skeleton } from "@/components/ui/primitives";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { MediaPoster } from "@/components/media/MediaPoster";
import { AiringBadge } from "@/components/media/AiringBadge";
import { useAddEntry, useCurrentSeason, useLibrary, useSeason } from "@/lib/hooks";
import { FORMAT_LABEL, mediaTitle } from "@/lib/format";
import { toast } from "@/stores/toast";
import { errorKind, errorMessage } from "@/lib/types";
import type { Media, MediaFormat, MediaSeasonName } from "@/lib/types";

export const Route = createFileRoute("/seasons")({
  component: SeasonsPage,
});

const SEASONS: { value: MediaSeasonName; label: string }[] = [
  { value: "WINTER", label: "Winter" },
  { value: "SPRING", label: "Spring" },
  { value: "SUMMER", label: "Summer" },
  { value: "FALL", label: "Fall" },
];

const FORMATS: { value: MediaFormat | "ALL"; label: string }[] = [
  { value: "ALL", label: "All formats" },
  { value: "TV", label: "TV" },
  { value: "TV_SHORT", label: "TV Short" },
  { value: "MOVIE", label: "Movie" },
  { value: "ONA", label: "ONA" },
  { value: "OVA", label: "OVA" },
  { value: "SPECIAL", label: "Special" },
];

function SeasonsPage() {
  const { data: current } = useCurrentSeason();
  const [sel, setSel] = React.useState<{
    year: number;
    season: MediaSeasonName;
  } | null>(null);
  const [format, setFormat] = React.useState<MediaFormat | "ALL">("ALL");
  const [genre, setGenre] = React.useState<string>("ALL");

  const active = sel ?? current ?? null;

  const { data, isLoading, isFetching, error } = useSeason(
    active?.year ?? 0,
    active?.season ?? "WINTER",
    active != null,
  );

  const genres = React.useMemo(() => {
    const s = new Set<string>();
    for (const m of data ?? []) for (const g of m.genres) s.add(g);
    return [...s].sort();
  }, [data]);

  const visible = React.useMemo(() => {
    return (data ?? []).filter(
      (m) =>
        (format === "ALL" || m.format === format) &&
        (genre === "ALL" || m.genres.includes(genre)),
    );
  }, [data, format, genre]);

  const nowYear = current?.year ?? new Date().getFullYear();
  const years: number[] = [];
  for (let y = nowYear + 1; y >= 1990; y--) years.push(y);

  const setYear = (y: number) =>
    setSel({ year: y, season: active?.season ?? "WINTER" });
  const setSeason = (s: MediaSeasonName) =>
    setSel({ year: active?.year ?? nowYear, season: s });

  return (
    <div className="mx-auto max-w-[1400px] px-6 py-5">
      <h1 className="mb-1 text-lg font-semibold">Seasons</h1>
      <p className="mb-4 text-xs text-muted-foreground">
        Browse an anime season, most popular first. Each season is fetched once
        and cached.
      </p>

      <div className="mb-5 flex flex-wrap items-center gap-2">
        <Segmented
          value={active?.season ?? "WINTER"}
          onChange={(v) => setSeason(v as MediaSeasonName)}
          options={SEASONS.map((s) => ({ value: s.value, label: s.label }))}
        />
        <Select
          value={String(active?.year ?? nowYear)}
          onValueChange={(v) => setYear(Number(v))}
        >
          <SelectTrigger className="h-8 w-24 text-xs" aria-label="Year">
            <SelectValue />
          </SelectTrigger>
          <SelectContent className="max-h-72">
            {years.map((y) => (
              <SelectItem key={y} value={String(y)}>
                {y}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>

        <div className="mx-1 h-5 w-px bg-border" />

        <Select
          value={format}
          onValueChange={(v) => setFormat(v as MediaFormat | "ALL")}
        >
          <SelectTrigger className="h-8 min-w-[7.5rem] text-xs" aria-label="Format">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            {FORMATS.map((f) => (
              <SelectItem key={f.value} value={f.value}>
                {f.label}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
        <Select value={genre} onValueChange={setGenre}>
          <SelectTrigger className="h-8 min-w-[7.5rem] text-xs" aria-label="Genre">
            <SelectValue />
          </SelectTrigger>
          <SelectContent className="max-h-72">
            <SelectItem value="ALL">All genres</SelectItem>
            {genres.map((g) => (
              <SelectItem key={g} value={g}>
                {g}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>

        {isFetching && !isLoading && (
          <span className="text-[11px] text-muted-foreground">refreshing…</span>
        )}
      </div>

      {isLoading || !active ? (
        <Grid>
          {Array.from({ length: 18 }).map((_, i) => (
            <div key={i} className="space-y-2">
              <Skeleton className="aspect-[2/3] w-full rounded-lg" />
              <Skeleton className="h-4 w-3/4" />
            </div>
          ))}
        </Grid>
      ) : error ? (
        <div className="flex flex-col items-center gap-2 py-24 text-center text-sm">
          {errorKind(error) === "service_unavailable" ? (
            <>
              <CloudOff className="size-5 text-warning" />
              <p className="max-w-sm text-muted-foreground">
                Season data needs AniList&apos;s API, which is temporarily down on
                their end. Try again later.
              </p>
            </>
          ) : (
            <p className="max-w-sm text-muted-foreground">{errorMessage(error)}</p>
          )}
        </div>
      ) : visible.length === 0 ? (
        <p className="py-24 text-center text-sm text-muted-foreground">
          Nothing matches those filters.
        </p>
      ) : (
        <Grid>
          {visible.map((m) => (
            <SeasonCard key={m.id.id} media={m} />
          ))}
        </Grid>
      )}
    </div>
  );
}

function Grid({ children }: { children: React.ReactNode }) {
  return (
    <div className="grid grid-cols-[repeat(auto-fill,minmax(150px,1fr))] gap-x-4 gap-y-6">
      {children}
    </div>
  );
}

function SeasonCard({ media }: { media: Media }) {
  const { data: library } = useLibrary();
  const onList = React.useMemo(
    () => new Set((library ?? []).map((e) => e.media.id.id)),
    [library],
  );
  const add = useAddEntry();
  const listed = onList.has(media.id.id);

  const handleAdd = () =>
    add.mutate(
      { mediaId: media.id.id, status: "PLANNING", progress: 0 },
      {
        onSuccess: () =>
          toast.success("Added to list", `${mediaTitle(media)} · Planning`),
        onError: (e) => toast.error("Couldn't add", errorMessage(e)),
      },
    );

  return (
    <div className="group flex flex-col gap-2">
      <div className="relative aspect-[2/3] overflow-hidden rounded-lg border border-border">
        <Link
          to="/media/$mediaId"
          params={{ mediaId: String(media.id.id) }}
          className="block size-full"
        >
          <MediaPoster media={media} className="size-full" />
        </Link>
        <div className="absolute left-1.5 top-1.5">
          <AiringBadge status={media.airingStatus} />
        </div>
        {media.averageScore != null && (
          <span className="absolute right-1.5 top-1.5 rounded-md bg-black/70 px-1.5 py-0.5 text-[11px] font-semibold text-white backdrop-blur-sm">
            {media.averageScore}%
          </span>
        )}
        <button
          onClick={listed ? undefined : handleAdd}
          disabled={listed || add.isPending}
          className="absolute bottom-1.5 right-1.5 inline-flex items-center gap-1 rounded-md bg-black/70 px-2 py-1 text-[11px] font-semibold text-white opacity-0 backdrop-blur-sm transition-opacity hover:bg-black/90 focus-visible:opacity-100 group-hover:opacity-100 disabled:opacity-100"
        >
          {listed ? (
            <>
              <Check className="size-3" /> On list
            </>
          ) : (
            <>
              <Plus className="size-3" /> Add
            </>
          )}
        </button>
      </div>
      <div className="min-w-0 space-y-1">
        <Link
          to="/media/$mediaId"
          params={{ mediaId: String(media.id.id) }}
          className="line-clamp-2 text-sm font-medium leading-tight hover:text-primary"
          title={mediaTitle(media)}
        >
          {mediaTitle(media)}
        </Link>
        <p className="text-[11px] text-muted-foreground">
          {FORMAT_LABEL[media.format]}
          {media.episodes ? ` · ${media.episodes} ep` : ""}
        </p>
      </div>
    </div>
  );
}
