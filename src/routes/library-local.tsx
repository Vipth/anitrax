import * as React from "react";
import { createFileRoute, Link } from "@tanstack/react-router";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import {
  FolderSearch,
  HardDrive,
  Link2,
  Link2Off,
  RefreshCw,
  Search,
} from "lucide-react";
import { api } from "@/lib/ipc";
import { qk } from "@/lib/query";
import { cn } from "@/lib/utils";
import { Input, Segmented, Skeleton } from "@/components/ui/primitives";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogBody,
  DialogContent,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { MediaPoster } from "@/components/media/MediaPoster";
import { useLibraryFiles, useScanLibrary } from "@/lib/hooks";
import { FORMAT_LABEL, episodeRanges, mediaTitle } from "@/lib/format";
import { toast } from "@/stores/toast";
import { errorMessage } from "@/lib/types";
import type { LibraryFile, Media, MediaTitle } from "@/lib/types";

export const Route = createFileRoute("/library-local")({
  component: LocalLibraryPage,
});

function titleOf(t: MediaTitle | null): string {
  return t?.english || t?.romaji || t?.native || "Unknown";
}

function useDebounced<T>(value: T, ms: number) {
  const [v, setV] = React.useState(value);
  React.useEffect(() => {
    const id = setTimeout(() => setV(value), ms);
    return () => clearTimeout(id);
  }, [value, ms]);
  return v;
}

function LocalLibraryPage() {
  const { data: files, isLoading } = useLibraryFiles();
  const scan = useScanLibrary();
  const [tab, setTab] = React.useState<"shows" | "unmatched">("shows");
  const [linking, setLinking] = React.useState<ReviewGroup | null>(null);

  const matched = React.useMemo(
    () => (files ?? []).filter((f) => f.mediaId != null),
    [files],
  );
  const unmatched = React.useMemo(
    () => (files ?? []).filter((f) => f.mediaId == null),
    [files],
  );

  const shows = React.useMemo(() => groupByMedia(matched), [matched]);
  const groups = React.useMemo(() => groupUnmatched(unmatched), [unmatched]);

  const runScan = () =>
    scan.mutate(undefined, {
      onSuccess: (r) =>
        toast.success(
          "Scan complete",
          `${r.filesSeen} files · ${r.autoMatched + r.ruleMatched} matched · ${r.unmatched} to review`,
        ),
      onError: (e) => toast.error("Scan failed", errorMessage(e)),
    });

  if (!isLoading && (files?.length ?? 0) === 0) {
    return (
      <div className="mx-auto flex max-w-md flex-col items-center gap-3 py-32 text-center">
        <HardDrive className="size-6 text-muted-foreground" />
        <h2 className="text-base font-semibold">No local files yet</h2>
        <p className="text-sm text-muted-foreground">
          Add the folders where your episodes live in Settings, and they&apos;ll
          be scanned and matched to your list here.
        </p>
        <Button asChild variant="secondary" size="sm">
          <Link to="/settings">Open settings</Link>
        </Button>
      </div>
    );
  }

  return (
    <div className="mx-auto max-w-[1100px] px-6 py-5">
      <header className="mb-4 flex flex-wrap items-center justify-between gap-3">
        <div>
          <h1 className="text-lg font-semibold">Local library</h1>
          <p className="text-xs text-muted-foreground">
            {matched.length} matched · {unmatched.length} to review
          </p>
        </div>
        <Button
          variant="secondary"
          size="sm"
          onClick={runScan}
          disabled={scan.isPending}
        >
          <RefreshCw className={cn("size-3.5", scan.isPending && "animate-spin")} />
          Rescan
        </Button>
      </header>

      <div className="mb-4 border-b border-border pb-3">
        <Segmented
          value={tab}
          onChange={setTab}
          options={[
            { value: "shows", label: `Shows (${shows.length})` },
            { value: "unmatched", label: `To review (${groups.length})` },
          ]}
        />
      </div>

      {isLoading ? (
        <div className="space-y-3">
          {Array.from({ length: 4 }).map((_, i) => (
            <Skeleton key={i} className="h-20 w-full rounded-lg" />
          ))}
        </div>
      ) : tab === "shows" ? (
        shows.length === 0 ? (
          <Empty text="Nothing matched yet. Check the “To review” tab." />
        ) : (
          <div className="space-y-2">
            {shows.map((s) => (
              <ShowRow key={s.mediaId} show={s} />
            ))}
          </div>
        )
      ) : groups.length === 0 ? (
        <Empty text="Every file is matched. Nice." />
      ) : (
        <>
          <p className="mb-3 text-xs text-muted-foreground">
            Files are grouped by the show and season we read from the folder.
            Link one group and every episode in it is linked — optionally
            remembered so future episodes match on their own.
          </p>
          <div className="space-y-1.5">
            {groups.map((g) => (
              <ReviewGroupRow
                key={g.key}
                group={g}
                onLink={() => setLinking(g)}
              />
            ))}
          </div>
        </>
      )}

      {linking && (
        <LinkDialog
          group={linking}
          open={!!linking}
          onOpenChange={(o) => !o && setLinking(null)}
        />
      )}
    </div>
  );
}

// --------------------------------------------------------------------------- //
// Matched — grouped by media
// --------------------------------------------------------------------------- //

interface Show {
  mediaId: number;
  service: string;
  title: MediaTitle | null;
  episodes: number[];
  fileCount: number;
  linked: boolean;
}

function groupByMedia(files: LibraryFile[]): Show[] {
  const map = new Map<number, Show>();
  for (const f of files) {
    if (f.mediaId == null) continue;
    let s = map.get(f.mediaId);
    if (!s) {
      s = {
        mediaId: f.mediaId,
        service: f.service ?? "anilist",
        title: f.mediaTitle,
        episodes: [],
        fileCount: 0,
        linked: false,
      };
      map.set(f.mediaId, s);
    }
    s.fileCount += 1;
    if (f.matchKind === "manual" || f.matchKind === "rule") s.linked = true;
    if (f.parsedEpisode != null && !s.episodes.includes(f.parsedEpisode)) {
      s.episodes.push(f.parsedEpisode);
    }
  }
  const shows = [...map.values()];
  for (const s of shows) s.episodes.sort((a, b) => a - b);
  shows.sort((a, b) => titleOf(a.title).localeCompare(titleOf(b.title)));
  return shows;
}

function ShowRow({ show }: { show: Show }) {
  const qc = useQueryClient();
  const unlink = useMutation({
    mutationFn: async () => {
      const files = qc.getQueryData<LibraryFile[]>(qk.libraryFiles) ?? [];
      const ids = files
        .filter((f) => f.mediaId === show.mediaId)
        .map((f) => f.id);
      for (const id of ids) await api.unlinkLibraryFile(id);
      // Drop any remembered rule that points at this show, so it doesn't
      // silently re-link on the next scan.
      const rules = await api.libraryLinkRules();
      for (const r of rules.filter((r) => r.mediaId === show.mediaId)) {
        await api.deleteLinkRule(r.id);
      }
    },
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: qk.libraryFiles });
      qc.invalidateQueries({ queryKey: qk.libraryOwned });
      toast.info("Unlinked", "Those files moved back to the review queue.");
    },
    onError: (e) => toast.error("Couldn't unlink", errorMessage(e)),
  });

  return (
    <div className="flex items-center gap-3 rounded-lg border border-border bg-surface-raised px-3 py-2.5">
      <Link
        to="/media/$mediaId"
        params={{ mediaId: String(show.mediaId) }}
        className="min-w-0 flex-1"
      >
        <p className="line-clamp-1 text-sm font-medium hover:text-primary">
          {titleOf(show.title)}
        </p>
        <p className="mt-0.5 text-[11px] text-muted-foreground">
          {show.fileCount} file{show.fileCount === 1 ? "" : "s"}
          {show.episodes.length > 0
            ? ` · Ep ${episodeRanges(show.episodes)}`
            : ""}
          {show.linked ? " · linked" : ""}
        </p>
      </Link>
      <button
        onClick={() => unlink.mutate()}
        disabled={unlink.isPending}
        className="grid size-7 shrink-0 place-items-center rounded-md text-muted-foreground transition-colors hover:bg-border/40 hover:text-foreground"
        aria-label="Unlink these files"
        title="Unlink"
      >
        <Link2Off className="size-3.5" />
      </button>
    </div>
  );
}

// --------------------------------------------------------------------------- //
// Review queue — grouped by folder title + season
// --------------------------------------------------------------------------- //

interface ReviewGroup {
  key: string;
  title: string;
  season: number | null;
  fileIds: number[];
  episodes: number[];
  sampleName: string;
}

function groupUnmatched(files: LibraryFile[]): ReviewGroup[] {
  const map = new Map<string, ReviewGroup>();
  for (const f of files) {
    const title = f.folderTitle || f.parsedTitle || "Unrecognised";
    const key = `${title.toLowerCase()} ${f.parsedSeason ?? ""}`;
    let g = map.get(key);
    if (!g) {
      g = {
        key,
        title,
        season: f.parsedSeason,
        fileIds: [],
        episodes: [],
        sampleName: f.fileName,
      };
      map.set(key, g);
    }
    g.fileIds.push(f.id);
    if (f.parsedEpisode != null && !g.episodes.includes(f.parsedEpisode)) {
      g.episodes.push(f.parsedEpisode);
    }
  }
  const groups = [...map.values()];
  for (const g of groups) g.episodes.sort((a, b) => a - b);
  groups.sort(
    (a, b) =>
      a.title.localeCompare(b.title) || (a.season ?? 0) - (b.season ?? 0),
  );
  return groups;
}

function ReviewGroupRow({
  group,
  onLink,
}: {
  group: ReviewGroup;
  onLink: () => void;
}) {
  return (
    <div className="flex items-center gap-3 rounded-lg border border-border px-3 py-2.5">
      <FolderSearch className="size-4 shrink-0 text-muted-foreground" />
      <div className="min-w-0 flex-1">
        <p className="line-clamp-1 text-sm font-medium" title={group.sampleName}>
          {group.title}
          {group.season != null && (
            <span className="ml-1.5 rounded bg-border/60 px-1.5 py-0.5 text-[10px] font-medium text-muted-foreground">
              S{group.season}
            </span>
          )}
        </p>
        <p className="mt-0.5 text-[11px] text-muted-foreground">
          {group.fileIds.length} file{group.fileIds.length === 1 ? "" : "s"}
          {group.episodes.length > 0
            ? ` · Ep ${episodeRanges(group.episodes)}`
            : ""}
        </p>
      </div>
      <Button size="sm" variant="secondary" onClick={onLink}>
        <Link2 className="size-3.5" /> Link
      </Button>
    </div>
  );
}

function LinkDialog({
  group,
  open,
  onOpenChange,
}: {
  group: ReviewGroup;
  open: boolean;
  onOpenChange: (o: boolean) => void;
}) {
  const qc = useQueryClient();
  const [query, setQuery] = React.useState(group.title);
  const [remember, setRemember] = React.useState(true);
  const debounced = useDebounced(query.trim(), 500);
  const canSearch = debounced.length >= 3;

  const { data: results, isFetching } = useQuery({
    queryKey: qk.search(debounced),
    queryFn: () => api.searchAnime(debounced),
    enabled: open && canSearch,
    staleTime: 10 * 60_000,
  });

  const link = useMutation({
    mutationFn: (media: Media) =>
      api.linkLibraryFiles(
        group.fileIds,
        media.id.id,
        remember,
        media.id.service,
      ),
    onSuccess: (_d, media) => {
      qc.invalidateQueries({ queryKey: qk.libraryFiles });
      qc.invalidateQueries({ queryKey: qk.libraryOwned });
      toast.success(
        "Linked",
        `${group.fileIds.length} file${group.fileIds.length === 1 ? "" : "s"} → ${mediaTitle(media)}`,
      );
      onOpenChange(false);
    },
    onError: (e) => toast.error("Couldn't link", errorMessage(e)),
  });

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-lg">
        <DialogHeader>
          <DialogTitle>
            Link {group.title}
            {group.season != null ? ` · Season ${group.season}` : ""}
          </DialogTitle>
        </DialogHeader>
        <DialogBody className="space-y-3">
          <p className="text-xs text-muted-foreground">
            {group.fileIds.length} file
            {group.fileIds.length === 1 ? "" : "s"} — e.g.{" "}
            <span className="text-foreground">{group.sampleName}</span>
          </p>
          <div className="relative">
            <Search className="pointer-events-none absolute left-3 top-1/2 size-4 -translate-y-1/2 text-muted-foreground" />
            <Input
              autoFocus
              value={query}
              onChange={(e) => setQuery(e.target.value)}
              placeholder="Search AniList by title…"
              className="pl-9"
            />
          </div>
          <p className="text-[11px] text-muted-foreground">
            Searching AniList costs one request. Cached results are reused.
          </p>

          {!canSearch ? (
            <p className="py-8 text-center text-xs text-muted-foreground">
              Type at least 3 characters.
            </p>
          ) : isFetching && !results ? (
            <div className="space-y-2">
              {Array.from({ length: 3 }).map((_, i) => (
                <Skeleton key={i} className="h-16 w-full rounded-md" />
              ))}
            </div>
          ) : (results?.length ?? 0) === 0 ? (
            <p className="py-8 text-center text-xs text-muted-foreground">
              No results for “{debounced}”.
            </p>
          ) : (
            <ul className="space-y-1.5">
              {results!.map((m) => (
                <li key={m.id.id}>
                  <button
                    onClick={() => link.mutate(m)}
                    disabled={link.isPending}
                    className="flex w-full items-center gap-3 rounded-md border border-border p-2 text-left transition-colors hover:border-primary disabled:opacity-50"
                  >
                    <MediaPoster media={m} className="h-14 w-10 rounded" />
                    <div className="min-w-0 flex-1">
                      <p className="line-clamp-1 text-sm font-medium">
                        {mediaTitle(m)}
                      </p>
                      <p className="text-[11px] text-muted-foreground">
                        {FORMAT_LABEL[m.format]}
                        {m.episodes ? ` · ${m.episodes} ep` : ""}
                        {m.seasonYear ? ` · ${m.seasonYear}` : ""}
                      </p>
                    </div>
                  </button>
                </li>
              ))}
            </ul>
          )}

          <label className="flex items-center gap-2 pt-1 text-xs text-muted-foreground">
            <input
              type="checkbox"
              checked={remember}
              onChange={(e) => setRemember(e.target.checked)}
              className="size-3.5 rounded border-border"
            />
            Remember this for future episodes of{" "}
            {group.title}
            {group.season != null ? ` Season ${group.season}` : ""}
          </label>
        </DialogBody>
      </DialogContent>
    </Dialog>
  );
}

function Empty({ text }: { text: string }) {
  return (
    <p className="py-16 text-center text-sm text-muted-foreground">{text}</p>
  );
}
