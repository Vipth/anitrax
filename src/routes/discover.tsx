import * as React from "react";
import { createFileRoute, Link } from "@tanstack/react-router";
import { useQuery } from "@tanstack/react-query";
import { Check, CloudOff, Plus, Search } from "lucide-react";
import { api } from "@/lib/ipc";
import { qk } from "@/lib/query";
import { Input, Skeleton } from "@/components/ui/primitives";
import { MediaPoster } from "@/components/media/MediaPoster";
import { useAddEntry, useLibrary } from "@/lib/hooks";
import { FORMAT_LABEL, mediaTitle, stripHtml } from "@/lib/format";
import { toast } from "@/stores/toast";
import { errorKind, errorMessage } from "@/lib/types";
import type { Media } from "@/lib/types";

export const Route = createFileRoute("/discover")({
  component: DiscoverPage,
});

function useDebounced<T>(value: T, ms: number) {
  const [v, setV] = React.useState(value);
  React.useEffect(() => {
    const id = setTimeout(() => setV(value), ms);
    return () => clearTimeout(id);
  }, [value, ms]);
  return v;
}

function DiscoverPage() {
  const [query, setQuery] = React.useState("");
  const debounced = useDebounced(query.trim(), 500);
  const enabled = debounced.length >= 3;

  const { data, isFetching, error } = useQuery({
    queryKey: qk.search(debounced),
    queryFn: () => api.searchAnime(debounced),
    enabled,
    staleTime: 10 * 60_000,
  });

  const { data: library } = useLibrary();
  const onList = React.useMemo(
    () => new Set((library ?? []).map((e) => e.media.id.id)),
    [library],
  );

  return (
    <div className="mx-auto max-w-[1100px] px-6 py-5">
      <h1 className="mb-1 text-lg font-semibold">Discover</h1>
      <p className="mb-4 text-xs text-muted-foreground">
        Search AniList. Results are cached, so repeat searches cost no requests.
      </p>

      <div className="relative mb-6 max-w-md">
        <Search className="pointer-events-none absolute left-3 top-1/2 size-4 -translate-y-1/2 text-muted-foreground" />
        <Input
          autoFocus
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          placeholder="Search anime by title…"
          className="h-10 pl-9"
        />
      </div>

      {!enabled ? (
        <p className="py-16 text-center text-sm text-muted-foreground">
          Type at least 3 characters to search.
        </p>
      ) : isFetching && !data ? (
        <div className="space-y-3">
          {Array.from({ length: 5 }).map((_, i) => (
            <Skeleton key={i} className="h-24 w-full rounded-lg" />
          ))}
        </div>
      ) : error ? (
        <div className="flex flex-col items-center gap-2 py-16 text-center text-sm">
          {errorKind(error) === "service_unavailable" ? (
            <>
              <CloudOff className="size-5 text-warning" />
              <p className="max-w-sm text-muted-foreground">
                Search needs AniList&apos;s API, which is temporarily down on
                their end. Your existing library still works — try again later.
              </p>
            </>
          ) : (
            <p className="max-w-sm text-muted-foreground">
              {errorMessage(error)}
            </p>
          )}
        </div>
      ) : (data?.length ?? 0) === 0 ? (
        <p className="py-16 text-center text-sm text-muted-foreground">
          No results for “{debounced}”.
        </p>
      ) : (
        <div className="space-y-3">
          {data!.map((m) => (
            <ResultRow key={m.id.id} media={m} onList={onList.has(m.id.id)} />
          ))}
        </div>
      )}
    </div>
  );
}

function ResultRow({ media, onList }: { media: Media; onList: boolean }) {
  const add = useAddEntry();

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
    <div className="flex gap-4 rounded-lg border border-border bg-surface-raised p-3">
      <Link
        to="/media/$mediaId"
        params={{ mediaId: String(media.id.id) }}
        className="shrink-0"
      >
        <MediaPoster media={media} className="h-28 w-20 rounded-md" />
      </Link>
      <div className="min-w-0 flex-1">
        <Link
          to="/media/$mediaId"
          params={{ mediaId: String(media.id.id) }}
          className="font-medium hover:text-primary"
        >
          {mediaTitle(media)}
        </Link>
        <p className="mt-0.5 text-xs text-muted-foreground">
          {FORMAT_LABEL[media.format]}
          {media.episodes ? ` · ${media.episodes} ep` : ""}
          {media.seasonYear ? ` · ${media.seasonYear}` : ""}
          {media.averageScore != null ? ` · ${media.averageScore}%` : ""}
        </p>
        <p className="mt-1.5 line-clamp-2 text-xs leading-relaxed text-muted-foreground">
          {stripHtml(media.description)}
        </p>
      </div>
      <div className="flex shrink-0 items-start">
        {onList ? (
          <span className="inline-flex items-center gap-1 rounded-md border border-border px-2.5 py-1.5 text-xs font-medium text-muted-foreground">
            <Check className="size-3.5" /> On list
          </span>
        ) : (
          <button
            onClick={handleAdd}
            disabled={add.isPending}
            className="inline-flex items-center gap-1 rounded-md bg-primary px-2.5 py-1.5 text-xs font-semibold text-primary-foreground hover:bg-primary/90 disabled:opacity-50"
          >
            <Plus className="size-3.5" /> Add
          </button>
        )}
      </div>
    </div>
  );
}
