import { describe, expect, it } from "vitest";
import { filterEntries, sortEntries } from "./library";
import type { MediaListEntry } from "./types";

function entry(over: Partial<MediaListEntry> & { id: number }): MediaListEntry {
  return {
    media: {
      id: { service: "anilist", id: over.id },
      title: { romaji: `Show ${over.id}`, english: null, native: null },
      format: "TV",
      airingStatus: "FINISHED",
      description: null,
      episodes: 12,
      duration: 24,
      season: null,
      seasonYear: 2020,
      coverUrl: null,
      coverColor: null,
      bannerUrl: null,
      averageScore: null,
      genres: [],
      synonyms: [],
      startDate: null,
      siteUrl: null,
      nextAiring: null,
      ...(over.media ?? {}),
    },
    remoteId: over.id,
    status: "CURRENT",
    progress: 0,
    scoreRaw: 0,
    repeat: 0,
    notes: null,
    startedAt: null,
    completedAt: null,
    updatedAt: null,
    dirty: false,
    ...over,
  };
}

describe("filterEntries", () => {
  it("matches any title language, case-insensitively", () => {
    const a = entry({ id: 1 });
    const b = entry({ id: 2 });
    b.media.title = { romaji: "Bocchi the Rock", english: "BOCCHI", native: "ぼっち" };
    const list = [a, b];
    expect(filterEntries(list, "bocchi").map((e) => e.media.id.id)).toEqual([2]);
    expect(filterEntries(list, "ぼっち").map((e) => e.media.id.id)).toEqual([2]);
    expect(filterEntries(list, "").length).toBe(2);
  });
});

describe("sortEntries", () => {
  const list = [
    { ...entry({ id: 1 }), scoreRaw: 90, progress: 6 },
    { ...entry({ id: 2 }), scoreRaw: 50, progress: 12 },
    { ...entry({ id: 3 }), scoreRaw: 70, progress: 1 },
  ];

  it("sorts by score descending", () => {
    expect(sortEntries(list, "score", "desc").map((e) => e.media.id.id)).toEqual([
      1, 3, 2,
    ]);
  });

  it("sorts by progress ascending (fraction of total)", () => {
    expect(
      sortEntries(list, "progress", "asc").map((e) => e.media.id.id),
    ).toEqual([3, 1, 2]);
  });

  it("does not mutate the input", () => {
    const copy = [...list];
    sortEntries(list, "score", "asc");
    expect(list).toEqual(copy);
  });

  it("pushes entries with no next episode to the end", () => {
    const withAiring = [
      entry({ id: 10 }),
      {
        ...entry({ id: 11 }),
        media: {
          ...entry({ id: 11 }).media,
          nextAiring: { episode: 5, airingAt: new Date(Date.now() + 8.64e7).toISOString() },
        },
      },
    ];
    expect(
      sortEntries(withAiring, "nextAiring", "asc").map((e) => e.media.id.id),
    ).toEqual([11, 10]);
  });
});
