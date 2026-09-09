import { cn } from "@/lib/utils";
import type { AiringStatus } from "@/lib/types";

const STATUS: Partial<
  Record<AiringStatus, { label: string; dot: string; pulse?: boolean }>
> = {
  RELEASING: { label: "Airing", dot: "bg-success", pulse: true },
  NOT_YET_RELEASED: { label: "Upcoming", dot: "bg-primary" },
  HIATUS: { label: "Hiatus", dot: "bg-warning" },
  CANCELLED: { label: "Cancelled", dot: "bg-danger" },
};

/**
 * At-a-glance broadcast status. Renders nothing for finished / unknown shows —
 * absence of a badge means "done", a green pulsing dot means "still airing".
 *
 * `overlay` sits on a poster (dark translucent pill); `plain` sits in a text row.
 */
export function AiringBadge({
  status,
  variant = "overlay",
  className,
}: {
  status: AiringStatus;
  variant?: "overlay" | "plain";
  className?: string;
}) {
  const s = STATUS[status];
  if (!s) return null;
  return (
    <span
      className={cn(
        "inline-flex shrink-0 items-center gap-1 rounded-md px-1.5 py-0.5 text-[11px] font-medium",
        variant === "overlay"
          ? "bg-black/70 text-white backdrop-blur-sm"
          : "bg-border/50 text-muted-foreground",
        className,
      )}
      title={`Broadcast status: ${s.label}`}
    >
      <span className="relative flex size-1.5">
        {s.pulse && (
          <span
            className={cn(
              "absolute inline-flex size-full animate-ping rounded-full opacity-75",
              s.dot,
            )}
          />
        )}
        <span
          className={cn("relative inline-flex size-1.5 rounded-full", s.dot)}
        />
      </span>
      {s.label}
    </span>
  );
}
