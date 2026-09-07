import { Activity, PauseCircle } from "lucide-react";
import { cn } from "@/lib/utils";
import { useBudget } from "@/lib/hooks";

/**
 * Live view of how hard we're leaning on the AniList API. The whole point of the
 * app is to never trip the rate limit, so this stays visible.
 */
export function RequestBudgetMeter({ compact = false }: { compact?: boolean }) {
  const { data } = useBudget();
  if (!data) return null;

  const { usedLastMinute, selfLimit, apiRemaining, parkedForSecs } = data;
  const pct = Math.min(100, (usedLastMinute / selfLimit) * 100);
  const tone =
    parkedForSecs > 0
      ? "bg-danger"
      : pct > 80
        ? "bg-warning"
        : "bg-success";

  if (compact) {
    return (
      <div
        className="flex items-center gap-1.5 text-[11px] text-muted-foreground"
        title={`${usedLastMinute}/${selfLimit} AniList requests in the last minute${
          apiRemaining != null ? ` · API says ${apiRemaining} left` : ""
        }`}
      >
        {parkedForSecs > 0 ? (
          <PauseCircle className="size-3.5 text-danger" />
        ) : (
          <Activity className="size-3.5" />
        )}
        <span className="tabular-nums">
          {parkedForSecs > 0
            ? `paused ${parkedForSecs}s`
            : `${usedLastMinute}/${selfLimit}`}
        </span>
      </div>
    );
  }

  return (
    <div className="space-y-2 rounded-lg border border-border bg-surface p-3">
      <div className="flex items-center justify-between text-xs">
        <span className="font-medium">AniList request budget</span>
        <span className="tabular-nums text-muted-foreground">
          {usedLastMinute} / {selfLimit} per min
        </span>
      </div>
      <div className="h-1.5 overflow-hidden rounded-full bg-border/70">
        <div
          className={cn("h-full rounded-full transition-all", tone)}
          style={{ width: `${pct}%` }}
        />
      </div>
      <p className="text-[11px] leading-relaxed text-muted-foreground">
        {parkedForSecs > 0 ? (
          <span className="text-danger">
            Paused for {parkedForSecs}s after a rate-limit response — edits are
            queued and will sync automatically.
          </span>
        ) : (
          <>
            Self-imposed ceiling is half of AniList&apos;s 90/min limit.
            {apiRemaining != null &&
              ` AniList reports ${apiRemaining} requests left in its window.`}
          </>
        )}
      </p>
    </div>
  );
}
