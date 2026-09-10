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

const pad = (n: number) => String(n).padStart(2, "0");

/** Live, ticking countdown to a media's next episode — one clean line. */
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
      <p className="mt-4 flex items-center gap-2 text-sm font-medium text-warning">
        <Radio className="size-4 animate-pulse" />
        Episode {episode} is airing now
      </p>
    );
  }

  const d = Math.floor(diff / 86_400_000);
  const h = Math.floor((diff % 86_400_000) / 3_600_000);
  const m = Math.floor((diff % 3_600_000) / 60_000);
  const s = Math.floor((diff % 60_000) / 1_000);

  return (
    <p className="mt-4 flex items-center gap-2 text-sm text-muted-foreground">
      <Timer className="size-4 text-warning" />
      Episode {episode} airs in
      <span className="font-semibold tabular-nums text-warning">
        {d > 0 && `${d}d `}
        {pad(h)}:{pad(m)}:{pad(s)}
      </span>
    </p>
  );
}
