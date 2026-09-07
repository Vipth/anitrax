import { create } from "zustand";

interface UiState {
  helpOpen: boolean;
  setHelpOpen: (open: boolean) => void;
  toggleHelp: () => void;

  /** Controls the sidebar theme picker so `T` can open it. */
  themeMenuOpen: boolean;
  setThemeMenuOpen: (open: boolean) => void;
  toggleThemeMenu: () => void;
}

export const useUi = create<UiState>((set) => ({
  helpOpen: false,
  setHelpOpen: (helpOpen) => set({ helpOpen }),
  toggleHelp: () => set((s) => ({ helpOpen: !s.helpOpen })),

  themeMenuOpen: false,
  setThemeMenuOpen: (themeMenuOpen) => set({ themeMenuOpen }),
  toggleThemeMenu: () => set((s) => ({ themeMenuOpen: !s.themeMenuOpen })),
}));
