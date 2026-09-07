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

/** Re-fetch library/accounts when the backend emits change events. */
export function useBackendEvents() {
  const qc = useQueryClient();
  useEffect(() => {
    const uns = [
      listen("entries-updated", () =>
        qc.invalidateQueries({ queryKey: qk.library() }),
      ),
      listen("auth-changed", () => {
        qc.invalidateQueries({ queryKey: qk.settings });
        qc.invalidateQueries({ queryKey: qk.accounts });
        qc.invalidateQueries({ queryKey: qk.library() });
      }),
    ];
    return () => {
      uns.forEach((p) => p.then((f) => f()));
    };
  }, [qc]);
}
