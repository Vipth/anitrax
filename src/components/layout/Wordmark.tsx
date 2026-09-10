import { cn } from "@/lib/utils";

const SIZE = {
  sm: "text-sm",
  md: "text-[17px]",
  lg: "text-2xl",
} as const;

/**
 * The AniTrax wordmark — a two-tone extrabold name wrapped in primary brackets.
 * No separate icon, so it doesn't clash with the nav glyphs.
 */
export function Wordmark({
  size = "md",
  className,
}: {
  size?: keyof typeof SIZE;
  className?: string;
}) {
  return (
    <span
      className={cn(
        "inline-flex select-none items-center font-extrabold tracking-tight text-foreground",
        SIZE[size],
        className,
      )}
    >
      <span aria-hidden="true" className="mr-[1px] font-medium text-primary">
        [
      </span>
      Ani<span className="text-primary">Trax</span>
      <span aria-hidden="true" className="ml-[1px] font-medium text-primary">
        ]
      </span>
    </span>
  );
}
