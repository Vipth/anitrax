import * as React from "react";
import { Radio, Timer } from "lucide-react";

function useNow(active = true) {
  const [now, setNow] = React.useState(() => Date.now());
  React.useEffect(() => {
    if (!active) return;
    const id = setInterval(() => setNow(Date.now()), 1000);
    return () => clearInterval(id);
  }, [active]);
  return now;
}

/** Live, ticking countdown to a media's next episode — one inline line. */
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
      <span className="inline-flex items-center gap-1.5 text-sm font-medium text-warning">
        <Radio className="size-3.5 animate-pulse" />
        Episode {episode} is airing now
      </span>
    );
  }

  const units = [
    { v: Math.floor(diff / 86_400_000), u: "d" },
    { v: Math.floor((diff % 86_400_000) / 3_600_000), u: "h" },
    { v: Math.floor((diff % 3_600_000) / 60_000), u: "m" },
    { v: Math.floor((diff % 60_000) / 1_000), u: "s" },
  ];
  const first = units.findIndex((x) => x.v > 0);
  const shown = units.slice(first === -1 ? 3 : first);

  return (
    <span className="inline-flex items-center gap-1.5 text-sm text-muted-foreground">
      <Timer className="size-3.5 text-warning" />
      Episode {episode} airs in
      <span className="font-semibold tabular-nums text-warning">
        {shown.map((x) => `${x.v}${x.u}`).join(" ")}
      </span>
    </span>
  );
}
