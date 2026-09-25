import type { ScheduleEntry } from "./types";

export interface CalendarDay {
  date: Date;
  inCurrentMonth: boolean;
  isToday: boolean;
}

function isSameLocalDay(a: Date, b: Date): boolean {
  return (
    a.getFullYear() === b.getFullYear() &&
    a.getMonth() === b.getMonth() &&
    a.getDate() === b.getDate()
  );
}

/** Weeks of a calendar month, always full 7-day rows — leading/trailing days
 * from adjacent months are included (`inCurrentMonth: false`) so the grid
 * never has a ragged first/last row. `month` is 1-12. */
export function monthGrid(
  year: number,
  month: number,
  weekStartsMonday: boolean,
): CalendarDay[][] {
  const first = new Date(year, month - 1, 1);
  // Date#getDay(): Sun=0..Sat=6. Shift so the grid starts on the right day.
  const startOffset = weekStartsMonday ? (first.getDay() + 6) % 7 : first.getDay();
  const gridStart = new Date(year, month - 1, 1 - startOffset);

  const daysInMonth = new Date(year, month, 0).getDate();
  const rows = Math.ceil((startOffset + daysInMonth) / 7);
  const today = new Date();

  const days: CalendarDay[] = [];
  for (let i = 0; i < rows * 7; i++) {
    const date = new Date(gridStart);
    date.setDate(gridStart.getDate() + i);
    days.push({
      date,
      inCurrentMonth: date.getMonth() === month - 1 && date.getFullYear() === year,
      isToday: isSameLocalDay(date, today),
    });
  }

  const weeks: CalendarDay[][] = [];
  for (let i = 0; i < days.length; i += 7) weeks.push(days.slice(i, i + 7));
  return weeks;
}

/** `"2026-09-24"` in the viewer's local timezone — deliberately not
 * `toISOString`, which is UTC and would shift entries near local midnight
 * onto the wrong day. */
export function localDayKey(date: Date): string {
  const y = date.getFullYear();
  const m = String(date.getMonth() + 1).padStart(2, "0");
  const d = String(date.getDate()).padStart(2, "0");
  return `${y}-${m}-${d}`;
}

/** Group schedule entries by the local calendar day they air on. Using
 * `Date`'s local getters throughout means this is DST-safe for free — no
 * manual offset math to get wrong across a transition. */
export function bucketByLocalDay(entries: ScheduleEntry[]): Map<string, ScheduleEntry[]> {
  const map = new Map<string, ScheduleEntry[]>();
  for (const entry of entries) {
    const key = localDayKey(new Date(entry.airingAt));
    const bucket = map.get(key);
    if (bucket) bucket.push(entry);
    else map.set(key, [entry]);
  }
  return map;
}
