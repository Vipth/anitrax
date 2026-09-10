import * as React from "react";
import { Radio } from "lucide-react";

function useNow(active = true) {
  const [now, setNow] = React.useState(() => Date.now());
  React.useEffect(() => {
    if (!active) return;
    const id = setInterval(() => setNow(Date.now()), 1000);
    return () => clearInterval(id);
  }, [active]);
  return now;
}

/**
 * Live, ticking countdown to a media's next episode. Shows days/hours/minutes/
 * seconds, dropping empty leading units.
 */
export function Countdown({
  airingAt,
  episode,
}: {
  airingAt: string;
  episode: number;
}) {
  const target = React.useMemo(() => new Date(airingAt).getTime(), [airingAt]);
  const now = useNow(true);
  const diff = target - now;

  if (Number.isNaN(target)) return null;

  if (diff <= 0) {
    return (
      <div className="mt-4 inline-flex items-center gap-2 rounded-lg border border-warning/40 bg-warning/10 px-3 py-2 text-sm font-medium text-warning">
        <Radio className="size-4 animate-pulse" />
        Episode {episode} is airing now
      </div>
    );
  }

  const d = Math.floor(diff / 86_400_000);
  const h = Math.floor((diff % 86_400_000) / 3_600_000);
  const m = Math.floor((diff % 3_600_000) / 60_000);
  const s = Math.floor((diff % 60_000) / 1_000);

  const all = [
    { v: d, label: "days" },
    { v: h, label: "hrs" },
    { v: m, label: "min" },
    { v: s, label: "sec" },
  ];
  // Drop leading zero units, but always keep at least minutes + seconds.
  const firstNonZero = all.findIndex((u) => u.v > 0);
  const start = Math.min(firstNonZero === -1 ? 2 : firstNonZero, 2);
  const units = all.slice(start);

  return (
    <div className="mt-4 flex w-full flex-col gap-2 rounded-lg border border-warning/30 bg-warning/[0.07] px-4 py-3">
      <span className="text-[11px] font-semibold uppercase tracking-wide text-warning/90">
        Episode {episode} airs in
      </span>
      <div className="flex items-end gap-2">
        {units.map((u, i) => (
          <React.Fragment key={u.label}>
            {i > 0 && (
              <span className="pb-4 text-lg font-light text-warning/40">:</span>
            )}
            <div className="flex flex-col items-center">
              <span className="min-w-[2ch] text-center text-2xl font-bold tabular-nums leading-none text-warning">
                {String(u.v).padStart(2, "0")}
              </span>
              <span className="mt-1 text-[10px] uppercase tracking-wide text-warning/60">
                {u.label}
              </span>
            </div>
          </React.Fragment>
        ))}
      </div>
    </div>
  );
}
