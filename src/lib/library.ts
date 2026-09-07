import type { LibrarySort } from "@/stores/prefs";
import type { MediaListEntry } from "./types";
import { mediaTitle } from "./format";

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
