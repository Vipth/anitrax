import { create } from "zustand";
import { persist } from "zustand/middleware";
import type { ListStatus } from "@/lib/types";

export type Theme =
  | "light"
  | "dark"
  | "cappuccino"
  | "cappuccino-dark"
  | "jade"
  | "nord"
  | "system";

/** Themes that are their own standalone palette (a class on <html>). */
export const CUSTOM_THEMES = [
  "cappuccino",
  "cappuccino-dark",
  "jade",
  "nord",
] as const;
export type LibraryLayout = "grid" | "list";
export type LibrarySort =
  | "title"
  | "score"
  | "progress"
  | "updated"
  | "nextAiring";

interface PrefsState {
  theme: Theme;
  layout: LibraryLayout;
  sort: LibrarySort;
  sortDir: "asc" | "desc";
  statusTab: ListStatus;
  setTheme: (t: Theme) => void;
  setLayout: (l: LibraryLayout) => void;
  setSort: (s: LibrarySort) => void;
  toggleSortDir: () => void;
  setStatusTab: (s: ListStatus) => void;
}

export const usePrefs = create<PrefsState>()(
  persist(
    (set) => ({
      theme: "system",
      layout: "grid",
      sort: "updated",
      sortDir: "desc",
      statusTab: "CURRENT",
      setTheme: (theme) => set({ theme }),
      setLayout: (layout) => set({ layout }),
      setSort: (sort) => set({ sort }),
      toggleSortDir: () =>
        set((s) => ({ sortDir: s.sortDir === "asc" ? "desc" : "asc" })),
      setStatusTab: (statusTab) => set({ statusTab }),
    }),
    { name: "anime-tracker-prefs" },
  ),
);

/** Apply the theme preference to <html>. Kept in sync with the pre-paint
 * bootstrap script in index.html. */
export function applyTheme(theme: Theme) {
  const root = document.documentElement;
  root.classList.remove("dark", ...CUSTOM_THEMES);
  if ((CUSTOM_THEMES as readonly string[]).includes(theme)) {
    root.classList.add(theme);
    return;
  }
  const dark =
    theme === "dark" ||
    (theme === "system" &&
      window.matchMedia("(prefers-color-scheme: dark)").matches);
  if (dark) root.classList.add("dark");
}
