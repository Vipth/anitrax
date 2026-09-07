import { create } from "zustand";

interface UiState {
  helpOpen: boolean;
  setHelpOpen: (open: boolean) => void;
  toggleHelp: () => void;
}

export const useUi = create<UiState>((set) => ({
  helpOpen: false,
  setHelpOpen: (helpOpen) => set({ helpOpen }),
  toggleHelp: () => set((s) => ({ helpOpen: !s.helpOpen })),
}));
