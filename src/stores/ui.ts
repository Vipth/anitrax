import { create } from "zustand";
import type { MediaFormat, MediaSeasonName } from "@/lib/types";

interface UiState {
  helpOpen: boolean;
  setHelpOpen: (open: boolean) => void;
  toggleHelp: () => void;

  /** Controls the sidebar theme picker so `T` can open it. */
  themeMenuOpen: boolean;
  setThemeMenuOpen: (open: boolean) => void;
  toggleThemeMenu: () => void;

  /** True while the library grid has keyboard focus — arrows drive the grid,
   * not the sidebar sections. Cleared on Escape or leaving the Library route. */
  gridNavEngaged: boolean;
  setGridNavEngaged: (v: boolean) => void;

  /** Library filter text — kept in the store (not persisted) so it survives
   * navigating away to Discover/Settings and back. */
  libraryFilter: string;
  setLibraryFilter: (v: string) => void;

  /** Discover search text — same idea: opening a result and coming back
   * should land you on your search, not a blank box. */
  discoverQuery: string;
  setDiscoverQuery: (v: string) => void;

  /** Season browser selection + filters — same idea again: opening a show
   * and hitting back should return you to the season you were browsing, not
   * snap to the current one. `null` season = follow the current season. */
  seasonSel: { year: number; season: MediaSeasonName } | null;
  setSeasonSel: (v: { year: number; season: MediaSeasonName }) => void;
  /** Back to "follow the current season". */
  clearSeasonSel: () => void;
  seasonFormat: MediaFormat | "ALL";
  setSeasonFormat: (v: MediaFormat | "ALL") => void;
  seasonGenre: string;
  setSeasonGenre: (v: string) => void;

  /** One-shot handoff: a library entry's "Make RSS rule" context-menu action
   * sets this, then navigates to /rss, which opens New Rule pre-filled with
   * that show and clears it right after — not persisted navigation state
   * like the fields above, just a single pending request. */
  rssRulePrefillMediaId: number | null;
  setRssRulePrefillMediaId: (v: number | null) => void;
}

export const useUi = create<UiState>((set) => ({
  helpOpen: false,
  setHelpOpen: (helpOpen) => set({ helpOpen }),
  toggleHelp: () => set((s) => ({ helpOpen: !s.helpOpen })),

  themeMenuOpen: false,
  setThemeMenuOpen: (themeMenuOpen) => set({ themeMenuOpen }),
  toggleThemeMenu: () => set((s) => ({ themeMenuOpen: !s.themeMenuOpen })),

  gridNavEngaged: false,
  setGridNavEngaged: (gridNavEngaged) => set({ gridNavEngaged }),

  libraryFilter: "",
  setLibraryFilter: (libraryFilter) => set({ libraryFilter }),

  discoverQuery: "",
  setDiscoverQuery: (discoverQuery) => set({ discoverQuery }),

  seasonSel: null,
  setSeasonSel: (seasonSel) => set({ seasonSel }),
  clearSeasonSel: () => set({ seasonSel: null }),
  seasonFormat: "ALL",
  setSeasonFormat: (seasonFormat) => set({ seasonFormat }),
  seasonGenre: "ALL",
  setSeasonGenre: (seasonGenre) => set({ seasonGenre }),

  rssRulePrefillMediaId: null,
  setRssRulePrefillMediaId: (rssRulePrefillMediaId) => set({ rssRulePrefillMediaId }),
}));
