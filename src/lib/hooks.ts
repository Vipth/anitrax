import { useEffect } from "react";
import {
  useMutation,
  useQuery,
  useQueryClient,
} from "@tanstack/react-query";
import { listen } from "@tauri-apps/api/event";
import { api } from "./ipc";
import { qk } from "./query";
import type { EntryPatch, MediaListEntry } from "./types";

export function useLibrary() {
  return useQuery({
    queryKey: qk.library(),
    queryFn: () => api.getLibrary(),
    staleTime: 30 * 60_000,
  });
}

export function useSettings() {
  return useQuery({ queryKey: qk.settings, queryFn: api.getSettings });
}

export function useStats() {
  return useQuery({
    queryKey: qk.stats,
    queryFn: () => api.getStats(),
    staleTime: 5 * 60_000,
  });
}

export function useLastSync() {
  return useQuery({
    queryKey: qk.lastSync(),
    queryFn: () => api.lastSync(),
    staleTime: 60_000,
  });
}

export function useBudget() {
  return useQuery({
    queryKey: qk.budget,
    queryFn: api.budgetSnapshot,
    refetchInterval: 4000,
    staleTime: 0,
  });
}

export function useSyncNow() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: () => api.syncNow(),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: qk.library() });
      qc.invalidateQueries({ queryKey: qk.lastSync() });
      qc.invalidateQueries({ queryKey: qk.stats });
    },
  });
}

export function useEditEntry() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (patch: EntryPatch) => api.editEntry(patch),
    onMutate: async (patch) => {
      await qc.cancelQueries({ queryKey: qk.library() });
      const prev = qc.getQueryData<MediaListEntry[]>(qk.library());
      if (prev) {
        qc.setQueryData<MediaListEntry[]>(
          qk.library(),
          prev.map((e) =>
            e.media.id.id === patch.mediaId
              ? {
                  ...e,
                  status: patch.status ?? e.status,
                  progress: patch.progress ?? e.progress,
                  scoreRaw: patch.scoreRaw ?? e.scoreRaw,
                  repeat: patch.repeat ?? e.repeat,
                  notes: patch.notes ?? e.notes,
                  startedAt: patch.startedAt ?? e.startedAt,
                  completedAt: patch.completedAt ?? e.completedAt,
                  dirty: true,
                }
              : e,
          ),
        );
      }
      return { prev };
    },
    onError: (_err, _patch, ctx) => {
      if (ctx?.prev) qc.setQueryData(qk.library(), ctx.prev);
    },
    onSettled: () => {
      qc.invalidateQueries({ queryKey: qk.library() });
    },
  });
}

export function useRemoveEntry() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (mediaId: number) => api.removeEntry(mediaId),
    onSuccess: (_d, mediaId) => {
      const prev = qc.getQueryData<MediaListEntry[]>(qk.library());
      if (prev) {
        qc.setQueryData(
          qk.library(),
          prev.filter((e) => e.media.id.id !== mediaId),
        );
      }
    },
  });
}

export function useAddEntry() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (patch: EntryPatch) => api.editEntry(patch),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: qk.library() });
    },
  });
}

export function useLibraryFolders() {
  return useQuery({
    queryKey: qk.libraryFolders,
    queryFn: () => api.libraryFolders(),
    staleTime: 5 * 60_000,
  });
}

export function useLibraryFiles() {
  return useQuery({
    queryKey: qk.libraryFiles,
    queryFn: () => api.libraryFiles(),
    staleTime: 5 * 60_000,
  });
}

export function useOwnedMedia() {
  return useQuery({
    queryKey: qk.libraryOwned,
    queryFn: () => api.libraryOwned(),
    staleTime: 5 * 60_000,
    // Map from media id -> sorted episode list for O(1) lookups in cards.
    select: (rows) => {
      const m = new Map<number, number[]>();
      for (const r of rows) m.set(r.mediaId, r.episodes);
      return m;
    },
  });
}

export function useCurrentSeason() {
  return useQuery({
    queryKey: qk.currentSeason,
    queryFn: () => api.currentSeason(),
    staleTime: 6 * 60 * 60_000,
  });
}

export function useSeason(year: number, season: string, enabled = true) {
  return useQuery({
    queryKey: qk.season(year, season),
    queryFn: () => api.getSeason(year, season),
    enabled,
    staleTime: 30 * 60_000,
  });
}

export function useScanLibrary() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: () => api.scanLibrary(),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: qk.libraryFiles });
      qc.invalidateQueries({ queryKey: qk.libraryFolders });
      qc.invalidateQueries({ queryKey: qk.libraryOwned });
    },
  });
}

export function useRssFeeds() {
  return useQuery({
    queryKey: qk.rssFeeds,
    queryFn: () => api.rssFeeds(),
    staleTime: 5 * 60_000,
  });
}

export function useRssRules() {
  return useQuery({
    queryKey: qk.rssRules,
    queryFn: () => api.rssRules(),
    staleTime: 5 * 60_000,
  });
}

export function useRssHistory() {
  return useQuery({
    queryKey: qk.rssHistory,
    queryFn: () => api.rssHistory(),
    staleTime: 60_000,
  });
}

export function useQbConfig() {
  return useQuery({ queryKey: qk.qbConfig, queryFn: () => api.getQbConfig() });
}

export function useRssPollEnabled() {
  return useQuery({
    queryKey: qk.rssPollEnabled,
    queryFn: () => api.rssPollEnabled(),
    staleTime: 5 * 60_000,
  });
}

export function useSetRssPollEnabled() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (enabled: boolean) => api.setRssPollEnabled(enabled),
    onMutate: async (enabled) => {
      await qc.cancelQueries({ queryKey: qk.rssPollEnabled });
      const prev = qc.getQueryData<boolean>(qk.rssPollEnabled);
      qc.setQueryData(qk.rssPollEnabled, enabled);
      return { prev };
    },
    onError: (_e, _v, ctx) => {
      if (ctx?.prev !== undefined) qc.setQueryData(qk.rssPollEnabled, ctx.prev);
    },
    onSettled: () => qc.invalidateQueries({ queryKey: qk.rssPollEnabled }),
  });
}

export function useCheckFeeds() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: () => api.checkFeedsNow(),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: qk.rssFeeds });
      qc.invalidateQueries({ queryKey: qk.rssHistory });
    },
  });
}

/** Re-fetch library/accounts when the backend emits change events. */
export function useBackendEvents() {
  const qc = useQueryClient();
  useEffect(() => {
    const uns = [
      listen("entries-updated", () => {
        qc.invalidateQueries({ queryKey: qk.library() });
        qc.invalidateQueries({ queryKey: qk.stats });
        qc.invalidateQueries({ queryKey: qk.lastSync() });
      }),
      listen("auth-changed", () => {
        qc.invalidateQueries({ queryKey: qk.settings });
        qc.invalidateQueries({ queryKey: qk.accounts });
        qc.invalidateQueries({ queryKey: qk.library() });
      }),
      listen("library-updated", () => {
        qc.invalidateQueries({ queryKey: qk.libraryFiles });
        qc.invalidateQueries({ queryKey: qk.libraryFolders });
        qc.invalidateQueries({ queryKey: qk.libraryOwned });
      }),
      listen("rss-updated", () => {
        qc.invalidateQueries({ queryKey: qk.rssHistory });
        qc.invalidateQueries({ queryKey: qk.rssFeeds });
      }),
    ];
    return () => {
      uns.forEach((p) => p.then((f) => f()));
    };
  }, [qc]);
}
