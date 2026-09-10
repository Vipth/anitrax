import * as React from "react";
import { createFileRoute, Link } from "@tanstack/react-router";
import { BarChart3 } from "lucide-react";
import { cn } from "@/lib/utils";
import { Card, Skeleton } from "@/components/ui/primitives";
import { Button } from "@/components/ui/button";
import { useStats } from "@/lib/hooks";
import { FORMAT_LABEL, STATUS_LABEL } from "@/lib/format";
import { errorMessage, type ListStatus, type MediaFormat } from "@/lib/types";
import type { StatBucket } from "@/lib/types";

export const Route = createFileRoute("/stats")({
  component: StatsPage,
});

function StatsPage() {
  const { data, isLoading, error } = useStats();

  if (error) {
    return (
      <div className="p-10 text-center text-sm text-muted-foreground">
        {errorMessage(error)}
      </div>
    );
  }

  if (isLoading || !data) {
    return (
      <div className="mx-auto max-w-[1100px] px-6 py-6">
        <h1 className="mb-6 text-lg font-semibold">Statistics</h1>
        <div className="grid grid-cols-2 gap-3 sm:grid-cols-4">
          {Array.from({ length: 4 }).map((_, i) => (
            <Skeleton key={i} className="h-20 rounded-lg" />
          ))}
        </div>
        <div className="mt-4 grid gap-4 md:grid-cols-2">
          {Array.from({ length: 4 }).map((_, i) => (
            <Skeleton key={i} className="h-56 rounded-lg" />
          ))}
        </div>
      </div>
    );
  }

  if (data.total === 0) {
    return (
      <div className="mx-auto flex max-w-md flex-col items-center gap-3 py-32 text-center">
        <BarChart3 className="size-6 text-muted-foreground" />
        <h2 className="text-base font-semibold">Nothing to chart yet</h2>
        <p className="text-sm text-muted-foreground">
          Sync your AniList list and your watch stats show up here.
        </p>
        <Button asChild variant="secondary" size="sm">
          <Link to="/">Go to library</Link>
        </Button>
      </div>
    );
  }

  const days = Math.floor(data.minutesWatched / 1440);
  const hours = Math.floor((data.minutesWatched % 1440) / 60);

  return (
    <div className="mx-auto max-w-[1100px] px-6 py-6">
      <h1 className="mb-6 text-lg font-semibold">Statistics</h1>

      <div className="grid grid-cols-2 gap-3 sm:grid-cols-4">
        <Kpi label="Entries" value={data.total.toLocaleString()} />
        <Kpi
          label="Episodes watched"
          value={data.episodesWatched.toLocaleString()}
        />
        <Kpi
          label="Time watched"
          value={days > 0 ? `${days}d ${hours}h` : `${hours}h`}
        />
        <Kpi
          label="Mean score"
          value={
            data.scoredCount > 0 ? `${(data.meanScore / 10).toFixed(2)} / 10` : "–"
          }
          sub={data.scoredCount > 0 ? `${data.scoredCount} rated` : "no scores yet"}
        />
      </div>

      <div className="mt-4 grid gap-4 md:grid-cols-2">
        <ChartCard title="Score distribution">
          {data.scoredCount > 0 ? (
            <ColumnChart
              data={data.scoreBuckets.map((b) => ({
                label: b.key,
                value: b.count,
                tip: `Score ${b.key}: ${b.count}`,
              }))}
            />
          ) : (
            <Empty />
          )}
        </ChartCard>

        <ChartCard title="Completions — last 12 months">
          <ColumnChart
            data={data.activity.map((b) => ({
              label: monthLabel(b.key),
              value: b.count,
              tip: `${b.key}: ${b.count} completed`,
            }))}
          />
        </ChartCard>

        <ChartCard title="Status">
          <BarList rows={data.byStatus} label={(k) => STATUS_LABEL[k as ListStatus] ?? k} />
          <p className="mt-3 border-t border-border pt-2 text-xs text-muted-foreground">
            Completion rate{" "}
            <span className="font-semibold text-foreground">
              {(data.completionRate * 100).toFixed(0)}%
            </span>{" "}
            of everything you&apos;ve started
          </p>
        </ChartCard>

        <ChartCard title="Format">
          <BarList rows={data.byFormat} label={(k) => FORMAT_LABEL[k as MediaFormat] ?? k} />
        </ChartCard>

        <ChartCard title="Top genres" className="md:col-span-2">
          {data.topGenres.length > 0 ? (
            <BarList rows={data.topGenres} label={(k) => k} labelWidth="9rem" />
          ) : (
            <Empty />
          )}
        </ChartCard>
      </div>
    </div>
  );
}

function Kpi({
  label,
  value,
  sub,
}: {
  label: string;
  value: string;
  sub?: string;
}) {
  return (
    <Card className="p-3">
      <p className="text-[11px] uppercase tracking-wide text-muted-foreground">
        {label}
      </p>
      <p className="mt-1 text-xl font-semibold tabular-nums">{value}</p>
      {sub && <p className="text-[11px] text-muted-foreground">{sub}</p>}
    </Card>
  );
}

function ChartCard({
  title,
  children,
  className,
}: {
  title: string;
  children: React.ReactNode;
  className?: string;
}) {
  return (
    <Card className={cn("p-4", className)}>
      <h2 className="mb-3 text-xs font-semibold uppercase tracking-wide text-muted-foreground">
        {title}
      </h2>
      {children}
    </Card>
  );
}

/** Simple vertical bar chart — proportional heights, labels beneath. */
function ColumnChart({
  data,
}: {
  data: { label: string; value: number; tip?: string }[];
}) {
  const max = Math.max(1, ...data.map((d) => d.value));
  return (
    <div>
      <div className="flex h-36 items-end gap-1.5">
        {data.map((d, i) => (
          <div
            key={i}
            className="flex-1 rounded-sm bg-primary transition-colors hover:bg-primary/80"
            style={{
              height: `${(d.value / max) * 100}%`,
              minHeight: d.value > 0 ? 3 : 0,
            }}
            title={d.tip ?? `${d.label}: ${d.value}`}
          />
        ))}
      </div>
      <div className="mt-1.5 flex gap-1.5">
        {data.map((d, i) => (
          <span
            key={i}
            className="flex-1 truncate text-center text-[10px] tabular-nums text-muted-foreground"
          >
            {d.label}
          </span>
        ))}
      </div>
    </div>
  );
}

/** Horizontal labelled bars. */
function BarList({
  rows,
  label,
  labelWidth = "5rem",
}: {
  rows: StatBucket[];
  label: (key: string) => string;
  labelWidth?: string;
}) {
  const max = Math.max(1, ...rows.map((r) => r.count));
  return (
    <div className="space-y-1.5">
      {rows.map((r) => (
        <div key={r.key} className="flex items-center gap-3 text-xs">
          <span
            className="shrink-0 truncate text-muted-foreground"
            style={{ width: labelWidth }}
            title={label(r.key)}
          >
            {label(r.key)}
          </span>
          <div className="h-2 flex-1 overflow-hidden rounded-full bg-border/60">
            <div
              className="h-full rounded-full bg-primary"
              style={{ width: `${(r.count / max) * 100}%` }}
            />
          </div>
          <span className="w-8 shrink-0 text-right font-medium tabular-nums">
            {r.count}
          </span>
        </div>
      ))}
    </div>
  );
}

function Empty() {
  return (
    <p className="py-12 text-center text-xs text-muted-foreground">No data</p>
  );
}

function monthLabel(key: string): string {
  const [y, m] = key.split("-");
  const name = new Date(Number(y), Number(m) - 1, 1).toLocaleString("en", {
    month: "short",
  });
  return m === "01" ? `${name} '${y.slice(2)}` : name;
}
