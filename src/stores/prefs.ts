import { create } from "zustand";
import { persist } from "zustand/middleware";
import type { ListStatus } from "@/lib/types";

export type Theme = "light" | "dark" | "system";
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

/** Apply the theme preference to <html>. */
export function applyTheme(theme: Theme) {
  const root = document.documentElement;
  const dark =
    theme === "dark" ||
    (theme === "system" &&
      window.matchMedia("(prefers-color-scheme: dark)").matches);
  root.classList.toggle("dark", dark);
}
