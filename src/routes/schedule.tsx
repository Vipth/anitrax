import * as React from "react";
import { createFileRoute, Link } from "@tanstack/react-router";
import { CalendarClock, ChevronLeft, ChevronRight } from "lucide-react";
import { Skeleton } from "@/components/ui/primitives";
import {
  Dialog,
  DialogBody,
  DialogContent,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { MediaPoster } from "@/components/media/MediaPoster";
import { bucketByLocalDay, monthGrid, type CalendarDay } from "@/lib/calendar";
import { useLibrary, useOwnedMedia, useSchedule, useSettings } from "@/lib/hooks";
import { STATUS_LABEL, formatAirTime, mediaTitle } from "@/lib/format";
import { cn } from "@/lib/utils";
import { useUi } from "@/stores/ui";
import type { ListStatus, MediaListEntry, ScheduleEntry } from "@/lib/types";

export const Route = createFileRoute("/schedule")({
  component: SchedulePage,
});

const CHIP_STATUSES: ListStatus[] = ["CURRENT", "PLANNING", "PAUSED"];
const MAX_VISIBLE_PER_DAY = 3;

function SchedulePage() {
  const { data: settings } = useSettings();
  const weekStartsMonday = settings?.weekStartsMonday ?? true;

  // Month + status filters live in the UI store so opening a show and hitting
  // back returns you to the month/filters you had (see stores/ui.ts).
  const monthSel = useUi((s) => s.scheduleMonth);
  const setMonthSel = useUi((s) => s.setScheduleMonth);
  const clearMonthSel = useUi((s) => s.clearScheduleMonth);
  const statusFilter = useUi((s) => s.scheduleStatusFilter);
  const toggleStatus = useUi((s) => s.toggleScheduleStatus);

  const now = new Date();
  const active = monthSel ?? { year: now.getFullYear(), month: now.getMonth() + 1 };
  const isCurrentMonth =
    active.year === now.getFullYear() && active.month === now.getMonth() + 1;

  const { data: schedule, isLoading } = useSchedule(active.year, active.month);
  const { data: library } = useLibrary();
  const { data: ownedMap } = useOwnedMedia();

  const libraryById = React.useMemo(() => {
    const m = new Map<number, MediaListEntry>();
    for (const e of library ?? []) m.set(e.media.id.id, e);
    return m;
  }, [library]);

  const visibleEntries = React.useMemo(
    () =>
      (schedule ?? []).filter((e) => {
        const le = libraryById.get(e.mediaId);
        return le != null && statusFilter.has(le.status);
      }),
    [schedule, libraryById, statusFilter],
  );

  const buckets = React.useMemo(() => bucketByLocalDay(visibleEntries), [visibleEntries]);
  const grid = React.useMemo(
    () => monthGrid(active.year, active.month, weekStartsMonday),
    [active.year, active.month, weekStartsMonday],
  );

  const gotoMonth = (delta: number) => {
    const d = new Date(active.year, active.month - 1 + delta, 1);
    setMonthSel({ year: d.getFullYear(), month: d.getMonth() + 1 });
  };

  const [openDay, setOpenDay] = React.useState<string | null>(null);

  const dayHeaders = weekStartsMonday
    ? ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"]
    : ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];

  const monthLabel = new Date(active.year, active.month - 1, 1).toLocaleDateString(
    undefined,
    { month: "long", year: "numeric" },
  );

  return (
    <div className="px-6 py-5">
      <h1 className="mb-1 text-lg font-semibold">Schedule</h1>
      <p className="mb-4 text-xs text-muted-foreground">
        When each tracked show's next episode airs, in your local time.
      </p>

      <div className="mb-4 flex flex-wrap items-center gap-2">
        <div className="flex items-center gap-1">
          <button
            onClick={() => gotoMonth(-1)}
            aria-label="Previous month"
            className="grid size-8 place-items-center rounded-md text-muted-foreground transition-colors hover:bg-border/40 hover:text-foreground"
          >
            <ChevronLeft className="size-4" />
          </button>
          <span className="w-40 text-center text-sm font-medium">{monthLabel}</span>
          <button
            onClick={() => gotoMonth(1)}
            aria-label="Next month"
            className="grid size-8 place-items-center rounded-md text-muted-foreground transition-colors hover:bg-border/40 hover:text-foreground"
          >
            <ChevronRight className="size-4" />
          </button>
        </div>

        {!isCurrentMonth && (
          <button
            onClick={clearMonthSel}
            className="inline-flex items-center gap-1 rounded-md border border-border px-2 py-1.5 text-[11px] font-medium text-muted-foreground transition-colors hover:border-primary hover:text-primary"
          >
            <CalendarClock className="size-3" />
            Today
          </button>
        )}

        <div className="mx-1 h-5 w-px bg-border" />

        <nav className="flex flex-wrap gap-1">
          {CHIP_STATUSES.map((s) => (
            <button
              key={s}
              onClick={() => toggleStatus(s)}
              className={cn(
                "rounded-md px-3 py-1.5 text-sm font-medium transition-colors",
                statusFilter.has(s)
                  ? "bg-primary/10 text-primary"
                  : "text-muted-foreground hover:bg-border/40 hover:text-foreground",
              )}
            >
              {STATUS_LABEL[s]}
            </button>
          ))}
        </nav>
      </div>

      {isLoading ? (
        <Skeleton className="h-[600px] w-full rounded-lg" />
      ) : (
        <CalendarGrid
          grid={grid}
          dayHeaders={dayHeaders}
          buckets={buckets}
          libraryById={libraryById}
          ownedMap={ownedMap}
          onOverflow={setOpenDay}
        />
      )}

      <Dialog open={openDay != null} onOpenChange={(o) => !o && setOpenDay(null)}>
        {openDay && (
          <DayDialog
            dayKey={openDay}
            entries={buckets.get(openDay) ?? []}
            libraryById={libraryById}
            ownedMap={ownedMap}
          />
        )}
      </Dialog>
    </div>
  );
}

function CalendarGrid({
  grid,
  dayHeaders,
  buckets,
  libraryById,
  ownedMap,
  onOverflow,
}: {
  grid: CalendarDay[][];
  dayHeaders: string[];
  buckets: Map<string, ScheduleEntry[]>;
  libraryById: Map<number, MediaListEntry>;
  ownedMap: Map<number, number[]> | undefined;
  onOverflow: (dayKey: string) => void;
}) {
  return (
    <div className="overflow-hidden rounded-lg border border-border">
      <div className="grid grid-cols-7 border-b border-border bg-surface">
        {dayHeaders.map((h) => (
          <div
            key={h}
            className="px-2 py-1.5 text-center text-[11px] font-medium uppercase tracking-wide text-muted-foreground"
          >
            {h}
          </div>
        ))}
      </div>
      <div className="grid grid-cols-7">
        {grid.flat().map((day) => (
          <DayCell
            key={day.date.toDateString()}
            day={day}
            entries={buckets.get(dayKeyOf(day)) ?? []}
            libraryById={libraryById}
            ownedMap={ownedMap}
            onOverflow={onOverflow}
          />
        ))}
      </div>
    </div>
  );
}

function dayKeyOf(day: CalendarDay): string {
  const y = day.date.getFullYear();
  const m = String(day.date.getMonth() + 1).padStart(2, "0");
  const d = String(day.date.getDate()).padStart(2, "0");
  return `${y}-${m}-${d}`;
}

function DayCell({
  day,
  entries,
  libraryById,
  ownedMap,
  onOverflow,
}: {
  day: CalendarDay;
  entries: ScheduleEntry[];
  libraryById: Map<number, MediaListEntry>;
  ownedMap: Map<number, number[]> | undefined;
  onOverflow: (dayKey: string) => void;
}) {
  const sorted = [...entries].sort((a, b) => a.airingAt.localeCompare(b.airingAt));
  const visible = sorted.slice(0, MAX_VISIBLE_PER_DAY);
  const overflow = sorted.length - visible.length;

  return (
    <div
      className={cn(
        "min-h-40 border-b border-r border-border p-1.5 last:border-r-0",
        !day.inCurrentMonth && "bg-surface/50",
      )}
    >
      <div className="mb-1 flex justify-end">
        <span
          className={cn(
            "grid size-5 place-items-center rounded-full text-[11px]",
            day.isToday
              ? "bg-primary font-semibold text-primary-foreground"
              : day.inCurrentMonth
                ? "text-foreground"
                : "text-muted-foreground/50",
          )}
        >
          {day.date.getDate()}
        </span>
      </div>
      <div className="space-y-1">
        {visible.map((e) => (
          <ScheduleRow
            key={`${e.mediaId}-${e.episode}`}
            entry={e}
            listEntry={libraryById.get(e.mediaId)}
            owned={ownedMap?.get(e.mediaId)?.includes(e.episode) ?? false}
          />
        ))}
        {overflow > 0 && (
          <button
            onClick={() => onOverflow(dayKeyOf(day))}
            className="w-full rounded px-1 py-0.5 text-left text-[10px] font-medium text-muted-foreground hover:bg-border/40 hover:text-foreground"
          >
            +{overflow} more
          </button>
        )}
      </div>
    </div>
  );
}

function ScheduleRow({
  entry,
  listEntry,
  owned,
}: {
  entry: ScheduleEntry;
  listEntry: MediaListEntry | undefined;
  owned: boolean;
}) {
  if (!listEntry) return null;
  const media = listEntry.media;
  const isNext = entry.episode === listEntry.progress + 1;
  const isPremiere = entry.episode === 1;
  const isFinale = media.episodes != null && entry.episode === media.episodes;

  return (
    <Link
      to="/media/$mediaId"
      params={{ mediaId: String(entry.mediaId) }}
      className={cn(
        "flex items-start gap-1.5 rounded-md px-1 py-1 transition-colors hover:bg-surface-raised",
        isNext ? "bg-primary/10 text-primary" : isFinale && "bg-warning/10 text-warning",
      )}
      title={`${mediaTitle(media)} · Episode ${entry.episode} · ${formatAirTime(entry.airingAt)}`}
    >
      <MediaPoster media={media} className="mt-0.5 size-6 shrink-0 rounded-sm" />
      <div className="min-w-0 flex-1">
        <p className="line-clamp-2 text-xs leading-tight">{mediaTitle(media)}</p>
        <p
          className={cn(
            "mt-0.5 flex items-center gap-1 text-[10px]",
            isNext || isFinale ? "text-current opacity-80" : "text-muted-foreground",
          )}
        >
          {owned && <span className="size-1.5 shrink-0 rounded-full bg-success" title="On disk" />}
          <span>{isPremiere ? "Premiere" : isFinale ? "Finale" : `Ep ${entry.episode}`}</span>
          <span>· {formatAirTime(entry.airingAt)}</span>
        </p>
      </div>
    </Link>
  );
}

function DayDialog({
  dayKey,
  entries,
  libraryById,
  ownedMap,
}: {
  dayKey: string;
  entries: ScheduleEntry[];
  libraryById: Map<number, MediaListEntry>;
  ownedMap: Map<number, number[]> | undefined;
}) {
  const sorted = [...entries].sort((a, b) => a.airingAt.localeCompare(b.airingAt));
  const label = new Date(`${dayKey}T00:00:00`).toLocaleDateString(undefined, {
    weekday: "long",
    month: "long",
    day: "numeric",
  });

  return (
    <DialogContent className="max-w-sm">
      <DialogHeader>
        <DialogTitle>{label}</DialogTitle>
      </DialogHeader>
      <DialogBody>
        <div className="space-y-1">
          {sorted.map((e) => (
            <ScheduleRow
              key={`${e.mediaId}-${e.episode}`}
              entry={e}
              listEntry={libraryById.get(e.mediaId)}
              owned={ownedMap?.get(e.mediaId)?.includes(e.episode) ?? false}
            />
          ))}
        </div>
      </DialogBody>
    </DialogContent>
  );
}
