import type { ListStatus, Media, MediaFormat } from "./types";

export const STATUS_ORDER: ListStatus[] = [
  "CURRENT",
  "REPEATING",
  "PLANNING",
  "PAUSED",
  "COMPLETED",
  "DROPPED",
];

export const STATUS_LABEL: Record<ListStatus, string> = {
  CURRENT: "Watching",
  REPEATING: "Rewatching",
  PLANNING: "Planning",
  PAUSED: "On Hold",
  COMPLETED: "Completed",
  DROPPED: "Dropped",
};

export const FORMAT_LABEL: Record<MediaFormat, string> = {
  TV: "TV",
  TV_SHORT: "TV Short",
  MOVIE: "Movie",
  SPECIAL: "Special",
  OVA: "OVA",
  ONA: "ONA",
  MUSIC: "Music",
  UNKNOWN: "—",
};

export function mediaTitle(m: Media): string {
  return m.title.english || m.title.romaji || m.title.native || "Unknown";
}

/** Convert a 0..100 raw score to the user's display scale. */
export function formatScore(scoreRaw: number, scoreFormat: string): string {
  if (scoreRaw <= 0) return "–";
  switch (scoreFormat) {
    case "POINT_100":
      return String(scoreRaw);
    case "POINT_10":
      return String(Math.round(scoreRaw / 10));
    case "POINT_5":
      return `${Math.round(scoreRaw / 20)}★`;
    case "POINT_3":
      return scoreRaw >= 70 ? "🙂" : scoreRaw >= 40 ? "😐" : "🙁";
    case "POINT_10_DECIMAL":
    default:
      return (scoreRaw / 10).toFixed(1);
  }
}

export function scoreToTen(scoreRaw: number): number {
  return Math.round((scoreRaw / 10) * 10) / 10;
}

export function tenToScoreRaw(ten: number): number {
  return Math.max(0, Math.min(100, Math.round(ten * 10)));
}

const RELATIVE = new Intl.RelativeTimeFormat("en", { numeric: "auto" });

export function relativeTime(iso: string | null | undefined): string {
  if (!iso) return "never";
  const then = new Date(iso).getTime();
  if (Number.isNaN(then)) return "never";
  const diff = then - Date.now();
  const abs = Math.abs(diff);
  const mins = abs / 60_000;
  if (mins < 1) return "just now";
  if (mins < 60) return RELATIVE.format(Math.round(diff / 60_000), "minute");
  const hrs = mins / 60;
  if (hrs < 24) return RELATIVE.format(Math.round(diff / 3_600_000), "hour");
  const days = hrs / 24;
  if (days < 30) return RELATIVE.format(Math.round(diff / 86_400_000), "day");
  return new Date(iso).toLocaleDateString();
}

export function countdown(iso: string): string {
  const secs = (new Date(iso).getTime() - Date.now()) / 1000;
  if (secs <= 0) return "airing now";
  const d = Math.floor(secs / 86400);
  const h = Math.floor((secs % 86400) / 3600);
  const m = Math.floor((secs % 3600) / 60);
  if (d > 0) return `${d}d ${h}h`;
  if (h > 0) return `${h}h ${m}m`;
  return `${m}m`;
}

/** Collapse a sorted list of episode numbers into `"1–5, 8, 10–12"`. */
export function episodeRanges(sorted: number[]): string {
  if (sorted.length === 0) return "none";
  const parts: string[] = [];
  let start = sorted[0];
  let prev = sorted[0];
  for (let i = 1; i <= sorted.length; i++) {
    const n = sorted[i];
    if (n === prev + 1) {
      prev = n;
      continue;
    }
    parts.push(start === prev ? `${start}` : `${start}–${prev}`);
    start = n;
    prev = n;
  }
  return parts.join(", ");
}

export function stripHtml(html: string | null | undefined): string {
  if (!html) return "";
  return html
    .replace(/<br\s*\/?>/gi, "\n")
    .replace(/<[^>]+>/g, "")
    .replace(/&mdash;/g, "—")
    .replace(/&nbsp;/g, " ")
    .replace(/&quot;/g, '"')
    .replace(/&#039;/g, "'")
    .replace(/&amp;/g, "&")
    .trim();
}
