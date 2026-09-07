// Mirrors `src-tauri/src/tracker/model.rs` (serde camelCase).

export type ServiceKind = "anilist" | "kitsu";

export interface MediaId {
  service: ServiceKind;
  id: number;
}

export interface MediaTitle {
  romaji: string | null;
  english: string | null;
  native: string | null;
}

export type AiringStatus =
  | "FINISHED"
  | "RELEASING"
  | "NOT_YET_RELEASED"
  | "CANCELLED"
  | "HIATUS"
  | "UNKNOWN";

export type MediaFormat =
  | "TV"
  | "TV_SHORT"
  | "MOVIE"
  | "SPECIAL"
  | "OVA"
  | "ONA"
  | "MUSIC"
  | "UNKNOWN";

export type MediaSeasonName = "WINTER" | "SPRING" | "SUMMER" | "FALL";

export type ListStatus =
  | "CURRENT"
  | "PLANNING"
  | "COMPLETED"
  | "DROPPED"
  | "PAUSED"
  | "REPEATING";

export interface AiringInfo {
  episode: number;
  airingAt: string;
}

export interface Media {
  id: MediaId;
  title: MediaTitle;
  format: MediaFormat;
  airingStatus: AiringStatus;
  description: string | null;
  episodes: number | null;
  duration: number | null;
  season: MediaSeasonName | null;
  seasonYear: number | null;
  coverUrl: string | null;
  coverColor: string | null;
  bannerUrl: string | null;
  averageScore: number | null;
  genres: string[];
  synonyms: string[];
  startDate: string | null;
  siteUrl: string | null;
  nextAiring: AiringInfo | null;
}

export interface MediaListEntry {
  media: Media;
  remoteId: number | null;
  status: ListStatus;
  progress: number;
  scoreRaw: number; // 0..100
  repeat: number;
  notes: string | null;
  startedAt: string | null;
  completedAt: string | null;
  updatedAt: string | null;
  dirty: boolean;
}

export interface EntryPatch {
  mediaId: number;
  remoteId?: number | null;
  status?: ListStatus | null;
  progress?: number | null;
  scoreRaw?: number | null;
  repeat?: number | null;
  notes?: string | null;
  startedAt?: string | null;
  completedAt?: string | null;
}

export interface Account {
  service: string;
  userId: string;
  userName: string;
  avatarUrl: string | null;
  scoreFormat: string;
  isPrimary: boolean;
  connectedAt: string;
}

export interface AppSettings {
  anilistClientId: string | null;
  anilistRedirect: string;
  accounts: Account[];
  syncOnStartup: boolean;
}

export interface SyncReport {
  service: string;
  entries: number;
  finishedAt: string;
}

export interface BudgetSnapshot {
  usedLastMinute: number;
  selfLimit: number;
  apiRemaining: number | null;
  parkedForSecs: number;
  queueDepth: number;
  serviceDown: boolean;
}

// Tagged mirror of `error.rs::WireError`.
export type AppError =
  | { kind: "not_authenticated"; data: { service: string } }
  | { kind: "rate_limited"; data: { service: string; retry_after_secs: number } }
  | { kind: "network"; data: { message: string } }
  | { kind: "service_unavailable"; data: { service: string; message: string } }
  | { kind: "api"; data: { service: string; message: string } }
  | { kind: "db"; data: { message: string } }
  | { kind: "keychain"; data: { message: string } }
  | { kind: "other"; data: { message: string } };

export function errorKind(e: unknown): AppError["kind"] | null {
  const err = e as AppError | undefined;
  return err && typeof err === "object" && "kind" in err ? err.kind : null;
}

export function errorMessage(e: unknown): string {
  const err = e as AppError | undefined;
  if (err && typeof err === "object" && "kind" in err) {
    switch (err.kind) {
      case "not_authenticated":
        return `Not signed in to ${err.data.service}.`;
      case "rate_limited":
        return `${err.data.service} is rate limiting us — retrying in ${err.data.retry_after_secs}s.`;
      case "network":
        return `Network error: ${err.data.message}`;
      case "service_unavailable":
        return `AniList's API is temporarily down on their end — not your setup. Your cached list still works; pending changes sync when it's back.`;
      case "api":
        return `${err.data.service}: ${err.data.message}`;
      case "db":
        return `Local database error: ${err.data.message}`;
      case "keychain":
        return `Keychain error: ${err.data.message}`;
      case "other":
        return err.data.message;
    }
  }
  return typeof e === "string" ? e : "Something went wrong.";
}
