import { create } from "zustand";
import { check, type Update } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";
import { api } from "@/lib/ipc";
import { errorMessage } from "@/lib/types";

interface UpdateState {
  update: Update | null;
  checking: boolean;
  downloading: boolean;
  progress: number;
  error: string | null;
  checkNow: () => Promise<Update | null>;
  installNow: () => Promise<void>;
  dismiss: () => void;
  skip: () => void;
}

export const useUpdateStore = create<UpdateState>((set, get) => ({
  update: null,
  checking: false,
  downloading: false,
  progress: 0,
  error: null,

  checkNow: async () => {
    set({ checking: true, error: null });
    let update: Update | null = null;
    try {
      update = await check();
      set({ update });
    } catch (e) {
      set({ error: errorMessage(e) });
    } finally {
      set({ checking: false });
      api.markUpdateChecked().catch(() => {});
    }
    return update;
  },

  installNow: async () => {
    const { update } = get();
    if (!update) return;
    set({ downloading: true, progress: 0, error: null });
    let total = 0;
    let downloaded = 0;
    try {
      await update.downloadAndInstall((event) => {
        if (event.event === "Started") {
          total = event.data.contentLength ?? 0;
        } else if (event.event === "Progress") {
          downloaded += event.data.chunkLength;
          set({ progress: total > 0 ? Math.min(downloaded / total, 1) : 0 });
        } else if (event.event === "Finished") {
          set({ progress: 1 });
        }
      });
      await relaunch();
    } catch (e) {
      set({ downloading: false, error: errorMessage(e) });
    }
  },

  // In-memory only — reappears on the next check (launch or manual).
  dismiss: () => set({ update: null }),

  // Persisted, so a version you've already declined stays quiet on future
  // launches until a newer one ships.
  skip: () => {
    const { update } = get();
    if (update) api.setSkippedUpdateVersion(update.version).catch(() => {});
    set({ update: null });
  },
}));
