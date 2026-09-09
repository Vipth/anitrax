import type { LibrarySort } from "@/stores/prefs";
import type { MediaListEntry } from "./types";
import { mediaTitle } from "./format";

/**
 * The episode you'd watch next, if it's sitting on disk — `progress + 1`, unless
 * you've already finished the show. `null` means "no Play button".
 */
export function nextEpisodeOnDisk(
  entry: MediaListEntry,
  owned: number[] | undefined,
): number | null {
  if (!owned || owned.length === 0) return null;
  const total = entry.media.episodes;
  if (total != null && entry.progress >= total) return null;
  const next = entry.progress + 1;
  return owned.includes(next) ? next : null;
}

export function filterEntries(
  entries: MediaListEntry[],
  query: string,
): MediaListEntry[] {
  const q = query.trim().toLowerCase();
  if (!q) return entries;
  return entries.filter((e) =>
    [e.media.title.romaji, e.media.title.english, e.media.title.native]
      .filter(Boolean)
      .some((t) => (t as string).toLowerCase().includes(q)),
  );
}

export function sortEntries(
  entries: MediaListEntry[],
  sort: LibrarySort,
  dir: "asc" | "desc",
): MediaListEntry[] {
  const mul = dir === "asc" ? 1 : -1;
  const val = (e: MediaListEntry): number | string => {
    switch (sort) {
      case "title":
        return mediaTitle(e.media).toLowerCase();
      case "score":
        return e.scoreRaw;
      case "progress":
        return e.media.episodes ? e.progress / e.media.episodes : e.progress;
      case "nextAiring":
        return e.media.nextAiring
          ? new Date(e.media.nextAiring.airingAt).getTime()
          : Number.MAX_SAFE_INTEGER;
      case "updated":
      default:
        return e.updatedAt ? new Date(e.updatedAt).getTime() : 0;
    }
  };
  return [...entries].sort((a, b) => {
    const av = val(a);
    const bv = val(b);
    if (typeof av === "string" && typeof bv === "string") {
      return av.localeCompare(bv) * mul;
    }
    return ((av as number) - (bv as number)) * mul;
  });
}
