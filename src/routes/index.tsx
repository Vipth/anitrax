import * as React from "react";
import { createFileRoute, Link, useNavigate } from "@tanstack/react-router";
import {
  ArrowDownUp,
  LayoutGrid,
  List as ListIcon,
  RefreshCw,
  Search,
} from "lucide-react";
import { cn } from "@/lib/utils";
import { Input, Segmented, Skeleton } from "@/components/ui/primitives";
import { Button } from "@/components/ui/button";
import { MediaCard, MediaListRow } from "@/components/media/MediaCard";
import { EditEntrySheet } from "@/components/media/EditEntrySheet";
import {
  useEditEntry,
  useLibrary,
  useLastSync,
  useSettings,
  useSyncNow,
} from "@/lib/hooks";
import { useHotkeys } from "@/lib/hotkeys";
import { usePrefs, type LibrarySort } from "@/stores/prefs";
import { STATUS_LABEL, STATUS_ORDER, relativeTime } from "@/lib/format";
import { filterEntries, sortEntries } from "@/lib/library";
import { toast } from "@/stores/toast";
import { errorMessage, type MediaListEntry } from "@/lib/types";

export const Route = createFileRoute("/")({
  component: LibraryPage,
});

const SORTS: { value: LibrarySort; label: string }[] = [
  { value: "updated", label: "Last updated" },
  { value: "title", label: "Title" },
  { value: "score", label: "Score" },
  { value: "progress", label: "Progress" },
  { value: "nextAiring", label: "Next episode" },
];

function LibraryPage() {
  const { data: settings } = useSettings();
  const { data: entries, isLoading } = useLibrary();
  const { data: lastSync } = useLastSync();
  const sync = useSyncNow();
  const edit = useEditEntry();
  const navigate = useNavigate();
  const {
    layout,
    setLayout,
    sort,
    setSort,
    sortDir,
    toggleSortDir,
    statusTab,
    setStatusTab,
  } = usePrefs();
  const [filter, setFilter] = React.useState("");
  const [selectedId, setSelectedId] = React.useState<number | null>(null);
  const [kbActive, setKbActive] = React.useState(false);
  const [editingEntry, setEditingEntry] = React.useState<MediaListEntry | null>(
    null,
  );
  const filterRef = React.useRef<HTMLInputElement>(null);
  const gridRef = React.useRef<HTMLDivElement>(null);

  const connected = (settings?.accounts.length ?? 0) > 0;

  const counts = React.useMemo(() => {
    const c: Record<string, number> = {};
    for (const e of entries ?? []) c[e.status] = (c[e.status] ?? 0) + 1;
    return c;
  }, [entries]);

  const visible = React.useMemo(() => {
    const list = (entries ?? []).filter((e) => e.status === statusTab);
    return sortEntries(filterEntries(list, filter), sort, sortDir);
  }, [entries, statusTab, filter, sort, sortDir]);

  // Keep the selection valid as the visible set changes.
  React.useEffect(() => {
    if (visible.length === 0) {
      setSelectedId(null);
    } else if (!visible.some((e) => e.media.id.id === selectedId)) {
      setSelectedId(visible[0].media.id.id);
    }
  }, [visible, selectedId]);

  const selectedIndex = visible.findIndex((e) => e.media.id.id === selectedId);
  const selectedEntry = selectedIndex >= 0 ? visible[selectedIndex] : null;

  const selectAt = React.useCallback(
    (index: number) => {
      if (visible.length === 0) return;
      setKbActive(true);
      const clamped = Math.max(0, Math.min(visible.length - 1, index));
      const id = visible[clamped].media.id.id;
      setSelectedId(id);
      requestAnimationFrame(() => {
        gridRef.current
          ?.querySelector(`[data-entry-id="${id}"]`)
          ?.scrollIntoView({ block: "nearest" });
      });
    },
    [visible],
  );

  const columns = () => {
    if (layout === "list" || !gridRef.current) return 1;
    const cols = getComputedStyle(gridRef.current).gridTemplateColumns;
    return Math.max(1, cols.split(" ").filter(Boolean).length);
  };

  const bumpSelected = (delta: number) => {
    if (!selectedEntry) return;
    const total = selectedEntry.media.episodes;
    const next = Math.max(0, selectedEntry.progress + delta);
    if (total != null && next > total) return;
    edit.mutate({
      mediaId: selectedEntry.media.id.id,
      remoteId: selectedEntry.remoteId,
      progress: next,
      status:
        total != null && next >= total && selectedEntry.status === "CURRENT"
          ? "COMPLETED"
          : undefined,
    });
  };

  const cycleTab = (delta: number) => {
    const i = STATUS_ORDER.indexOf(statusTab);
    const next = (i + delta + STATUS_ORDER.length) % STATUS_ORDER.length;
    setStatusTab(STATUS_ORDER[next]);
  };

  const doSync = () =>
    sync.mutate(undefined, {
      onSuccess: (r) =>
        toast.success("Synced", `${r.entries} entries from AniList.`),
      onError: (e) => toast.error("Sync failed", errorMessage(e)),
    });

  useHotkeys(
    {
      "/": (e) => {
        e.preventDefault();
        filterRef.current?.focus();
        filterRef.current?.select();
      },
      j: () => selectAt(selectedIndex + columns()),
      ArrowDown: () => selectAt(selectedIndex + columns()),
      k: () => selectAt(selectedIndex - columns()),
      ArrowUp: () => selectAt(selectedIndex - columns()),
      h: () => selectAt(selectedIndex - 1),
      ArrowLeft: () => selectAt(selectedIndex - 1),
      l: () => selectAt(selectedIndex + 1),
      ArrowRight: () => selectAt(selectedIndex + 1),
      Enter: () => {
        if (selectedEntry)
          navigate({
            to: "/media/$mediaId",
            params: { mediaId: String(selectedEntry.media.id.id) },
          });
      },
      e: () => selectedEntry && setEditingEntry(selectedEntry),
      "+": () => bumpSelected(1),
      "=": () => bumpSelected(1),
      "-": () => bumpSelected(-1),
      "[": () => cycleTab(-1),
      "]": () => cycleTab(1),
      r: () => !sync.isPending && doSync(),
    },
    { enabled: connected && !editingEntry },
  );

  if (!connected && !isLoading) {
    return (
      <EmptyState
        title="Connect an account to get started"
        body="Anime Tracker syncs your list with AniList. Add your account in settings and your library shows up here."
        action={
          <Button asChild>
            <Link to="/settings">Open settings</Link>
          </Button>
        }
      />
    );
  }

  return (
    <div className="mx-auto max-w-[1400px] px-6 py-5">
      <header className="mb-4 flex flex-wrap items-center justify-between gap-3">
        <div>
          <h1 className="text-lg font-semibold">Library</h1>
          <p className="text-xs text-muted-foreground">
            Last synced {relativeTime(lastSync)}
          </p>
        </div>
        <div className="flex items-center gap-2">
          <div className="relative">
            <Search className="pointer-events-none absolute left-2.5 top-1/2 size-3.5 -translate-y-1/2 text-muted-foreground" />
            <Input
              ref={filterRef}
              value={filter}
              onChange={(e) => setFilter(e.target.value)}
              onKeyDown={(e) => e.key === "Escape" && filterRef.current?.blur()}
              placeholder="Filter…"
              className="h-8 w-44 pl-8"
            />
          </div>
          <Button
            variant="secondary"
            size="sm"
            onClick={doSync}
            disabled={sync.isPending}
          >
            <RefreshCw
              className={cn("size-3.5", sync.isPending && "animate-spin")}
            />
            Sync
          </Button>
        </div>
      </header>

      <div className="mb-4 flex flex-wrap items-center justify-between gap-3 border-b border-border pb-3">
        <nav className="flex flex-wrap gap-1">
          {STATUS_ORDER.map((s) => (
            <button
              key={s}
              onClick={() => setStatusTab(s)}
              className={cn(
                "rounded-md px-3 py-1.5 text-sm font-medium transition-colors",
                statusTab === s
                  ? "bg-primary/10 text-primary"
                  : "text-muted-foreground hover:bg-border/40 hover:text-foreground",
              )}
            >
              {STATUS_LABEL[s]}
              {counts[s] ? (
                <span className="ml-1.5 text-xs text-muted-foreground">
                  {counts[s]}
                </span>
              ) : null}
            </button>
          ))}
        </nav>

        <div className="flex items-center gap-2">
          <select
            value={sort}
            onChange={(e) => setSort(e.target.value as LibrarySort)}
            className="h-8 rounded-md border border-border bg-surface px-2 text-xs outline-none focus:border-primary"
          >
            {SORTS.map((s) => (
              <option key={s.value} value={s.value}>
                {s.label}
              </option>
            ))}
          </select>
          <button
            onClick={toggleSortDir}
            className="grid size-8 place-items-center rounded-md border border-border text-muted-foreground hover:bg-border/40"
            title={sortDir === "asc" ? "Ascending" : "Descending"}
          >
            <ArrowDownUp className="size-3.5" />
          </button>
          <Segmented
            value={layout}
            onChange={setLayout}
            options={[
              { value: "grid", label: <LayoutGrid className="size-3.5" /> },
              { value: "list", label: <ListIcon className="size-3.5" /> },
            ]}
          />
        </div>
      </div>

      {isLoading ? (
        <GridSkeleton />
      ) : visible.length === 0 ? (
        <EmptyState
          compact
          title={`Nothing in ${STATUS_LABEL[statusTab]}`}
          body="Add shows from the Discover tab or change status filters."
          action={
            <Button asChild variant="secondary" size="sm">
              <Link to="/discover">Discover anime</Link>
            </Button>
          }
        />
      ) : layout === "grid" ? (
        <div
          ref={gridRef}
          className="grid grid-cols-[repeat(auto-fill,minmax(150px,1fr))] gap-x-4 gap-y-6"
        >
          {visible.map((e) => (
            <MediaCard
              key={e.media.id.id}
              entry={e}
              selected={kbActive && e.media.id.id === selectedId}
              onEdit={setEditingEntry}
            />
          ))}
        </div>
      ) : (
        <div ref={gridRef} className="space-y-1.5">
          {visible.map((e) => (
            <MediaListRow
              key={e.media.id.id}
              entry={e}
              selected={kbActive && e.media.id.id === selectedId}
              onEdit={setEditingEntry}
            />
          ))}
        </div>
      )}

      {editingEntry && (
        <EditEntrySheet
          entry={editingEntry}
          open={!!editingEntry}
          onOpenChange={(o) => !o && setEditingEntry(null)}
        />
      )}
    </div>
  );
}

function GridSkeleton() {
  return (
    <div className="grid grid-cols-[repeat(auto-fill,minmax(150px,1fr))] gap-x-4 gap-y-6">
      {Array.from({ length: 12 }).map((_, i) => (
        <div key={i} className="space-y-2">
          <Skeleton className="aspect-[2/3] w-full rounded-lg" />
          <Skeleton className="h-4 w-3/4" />
          <Skeleton className="h-3 w-1/2" />
        </div>
      ))}
    </div>
  );
}

function EmptyState({
  title,
  body,
  action,
  compact,
}: {
  title: string;
  body: string;
  action?: React.ReactNode;
  compact?: boolean;
}) {
  return (
    <div
      className={cn(
        "mx-auto flex max-w-sm flex-col items-center gap-3 text-center",
        compact ? "py-16" : "py-32",
      )}
    >
      <h2 className="text-base font-semibold">{title}</h2>
      <p className="text-sm text-muted-foreground">{body}</p>
      {action}
    </div>
  );
}
