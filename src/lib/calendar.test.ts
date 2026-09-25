import { describe, expect, it } from "vitest";
import { bucketByLocalDay, localDayKey, monthGrid } from "./calendar";
import type { ScheduleEntry } from "./types";

function entry(airingAt: string, mediaId = 1, episode = 1): ScheduleEntry {
  return { mediaId, episode, airingAt };
}

describe("monthGrid", () => {
  it("always produces full 7-day weeks", () => {
    for (const [year, month] of [[2026, 1], [2026, 2], [2027, 12]] as const) {
      const grid = monthGrid(year, month, true);
      for (const week of grid) expect(week).toHaveLength(7);
    }
  });

  it("starts each week on the configured day", () => {
    const monday = monthGrid(2026, 6, true);
    expect(monday[0][0].date.getDay()).toBe(1); // Monday

    const sunday = monthGrid(2026, 6, false);
    expect(sunday[0][0].date.getDay()).toBe(0); // Sunday
  });

  it("includes every real day of the month, marked inCurrentMonth", () => {
    const year = 2026;
    const month = 3; // March
    const daysInMonth = new Date(year, month, 0).getDate();
    const grid = monthGrid(year, month, true);
    const flat = grid.flat();

    const inMonth = flat.filter((d) => d.inCurrentMonth);
    expect(inMonth).toHaveLength(daysInMonth);
    expect(inMonth[0].date.getDate()).toBe(1);
    expect(inMonth[inMonth.length - 1].date.getDate()).toBe(daysInMonth);

    // Leading/trailing padding days are explicitly not in the target month.
    const leading = flat.slice(0, flat.indexOf(inMonth[0]));
    for (const d of leading) expect(d.inCurrentMonth).toBe(false);
  });

  it("produces 5 or 6 rows depending on how the month falls", () => {
    for (const [year, month] of [[2026, 1], [2026, 2], [2026, 3], [2026, 8], [2026, 10]] as const) {
      const startOffset = (new Date(year, month - 1, 1).getDay() + 6) % 7;
      const daysInMonth = new Date(year, month, 0).getDate();
      const expectedRows = Math.ceil((startOffset + daysInMonth) / 7);
      expect(monthGrid(year, month, true)).toHaveLength(expectedRows);
    }
  });

  it("marks today when it falls in the grid", () => {
    const now = new Date();
    const grid = monthGrid(now.getFullYear(), now.getMonth() + 1, true);
    const todays = grid.flat().filter((d) => d.isToday);
    expect(todays).toHaveLength(1);
    expect(todays[0].date.getDate()).toBe(now.getDate());
  });
});

describe("localDayKey / bucketByLocalDay", () => {
  it("formats as zero-padded YYYY-MM-DD", () => {
    expect(localDayKey(new Date(2026, 0, 5))).toBe("2026-01-05");
  });

  it("buckets entries on the same local day together", () => {
    const morning = new Date(2026, 8, 24, 8, 0).toISOString();
    const evening = new Date(2026, 8, 24, 22, 0).toISOString();
    const buckets = bucketByLocalDay([entry(morning), entry(evening, 2)]);
    expect(buckets.size).toBe(1);
    expect(buckets.get("2026-09-24")).toHaveLength(2);
  });

  it("splits entries on either side of local midnight into different days", () => {
    const lateNight = new Date(2026, 8, 24, 23, 30).toISOString();
    const justAfter = new Date(2026, 8, 25, 0, 30).toISOString();
    const buckets = bucketByLocalDay([entry(lateNight), entry(justAfter, 2)]);
    expect(buckets.size).toBe(2);
    expect(buckets.get("2026-09-24")).toHaveLength(1);
    expect(buckets.get("2026-09-25")).toHaveLength(1);
  });

  it("is stable across a DST transition (local-midnight-crossing is what matters, not UTC offset)", () => {
    // Whatever the runner's own timezone/DST rules are, a moment just before
    // local midnight and a moment just after must still land on different
    // local days — the bucketing never touches UTC offsets directly.
    const before = new Date(2026, 2, 8, 23, 45).toISOString();
    const after = new Date(2026, 2, 9, 0, 15).toISOString();
    const buckets = bucketByLocalDay([entry(before), entry(after, 2)]);
    expect(buckets.size).toBe(2);
  });
});
