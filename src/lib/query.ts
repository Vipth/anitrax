import { QueryClient } from "@tanstack/react-query";

/**
 * Cache-first defaults. The Rust side already holds the durable SQLite cache and
 * paces every network call, so the query layer just needs to avoid needless
 * refetches: no refetch on focus/reconnect, and long stale times that mirror the
 * backend TTLs.
 */
export const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      staleTime: 5 * 60_000,
      gcTime: 60 * 60_000,
      refetchOnWindowFocus: false,
      refetchOnReconnect: false,
      retry: (count, error) => {
        const e = error as { kind?: string } | undefined;
        if (
          e?.kind === "rate_limited" ||
          e?.kind === "not_authenticated" ||
          e?.kind === "service_unavailable"
        )
          return false;
        return count < 2;
      },
    },
  },
});

// Handy for poking at cache state from the devtools console.
if (import.meta.env.DEV) {
  (window as unknown as { __qc?: QueryClient }).__qc = queryClient;
}

export const qk = {
  settings: ["settings"] as const,
  accounts: ["accounts"] as const,
  library: (service?: string) => ["library", service ?? "anilist"] as const,
  media: (id: number) => ["media", id] as const,
  search: (q: string) => ["search", q] as const,
  budget: ["budget"] as const,
  lastSync: (service?: string) => ["lastSync", service ?? "anilist"] as const,
  libraryFolders: ["libraryFolders"] as const,
  libraryFiles: ["libraryFiles"] as const,
  libraryOwned: ["libraryOwned"] as const,
};
