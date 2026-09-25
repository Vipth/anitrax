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
  popularity: number | null;
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

/** M9 — a single episode's air time. Deliberately thin: no title/poster/
 * progress — every media id it references is already tracked, so the rest
 * comes from the existing `useLibrary()` cache, not a second round-trip. */
export interface ScheduleEntry {
  mediaId: number;
  episode: number;
  airingAt: string;
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

export type PlaybackMode = "confirm" | "silent";

// M6c — live-position players supported so far.
export type PlayerIntegrationKind = "vlc" | "mpv";

export interface AppSettings {
  anilistClientId: string | null;
  anilistRedirect: string;
  accounts: Account[];
  syncOnStartup: boolean;
  closeToTray: boolean;
  startOnLogin: boolean;
  startMinimized: boolean;
  playbackEnabled: boolean;
  playbackMode: PlaybackMode;
  playbackWindowDetect: boolean;
  monitoredPlayers: string[];
  playerIntegrationKind: PlayerIntegrationKind | null;
  playerIntegrationPath: string | null;
  appVersion: string;
  autoUpdateCheck: boolean;
  lastUpdateCheck: string | null;
  skippedUpdateVersion: string | null;
  weekStartsMonday: boolean;
}

export interface KnownPlayer {
  exe: string;
  label: string;
}

// M6a — playback detection (mirrors `src-tauri/src/playback/mod.rs`).
export interface WatchSessionView {
  service: ServiceKind;
  mediaId: number;
  episode: number;
  title: string;
  episodesTotal: number | null;
  elapsedSecs: number;
  thresholdSecs: number;
  // M6c — set only for a live-position session, once its poller has reported in.
  positionSecs: number | null;
  durationSecs: number | null;
}

export interface SyncReport {
  service: string;
  entries: number;
  finishedAt: string;
}

export interface CurrentSeason {
  year: number;
  season: MediaSeasonName;
}

export interface StatBucket {
  key: string;
  count: number;
}

export interface StatsData {
  total: number;
  byStatus: StatBucket[];
  episodesWatched: number;
  minutesWatched: number;
  meanScore: number;
  scoredCount: number;
  scoreBuckets: StatBucket[];
  topGenres: StatBucket[];
  byFormat: StatBucket[];
  completionRate: number;
  activity: StatBucket[];
}

export interface LibraryFolder {
  id: number;
  path: string;
  enabled: boolean;
  addedAt: string;
  scannedAt: string | null;
  fileCount: number;
}

export interface LibraryFile {
  id: number;
  folderId: number;
  path: string;
  fileName: string;
  sizeBytes: number | null;
  modifiedAt: string | null;
  parsedTitle: string | null;
  folderTitle: string | null;
  parsedEpisode: number | null;
  parsedSeason: number | null;
  resolution: string | null;
  releaseGroup: string | null;
  service: string | null;
  mediaId: number | null;
  matchKind: "auto" | "manual" | "rule" | null;
  matchScore: number | null;
  scannedAt: string;
  mediaTitle: MediaTitle | null;
}

export interface OwnedMedia {
  mediaId: number;
  episodes: number[];
}

export interface LinkRule {
  id: number;
  titleKey: string;
  season: number | null;
  service: string;
  mediaId: number;
  createdAt: string;
}

export interface ScanReport {
  folders: number;
  filesSeen: number;
  filesRemoved: number;
  autoMatched: number;
  ruleMatched: number;
  unmatched: number;
  finishedAt: string;
}

// RSS auto-download (M5) — mirrors `src-tauri/src/rss/mod.rs`.

export interface RssFeed {
  id: number;
  name: string;
  url: string;
  enabled: boolean;
  addedAt: string;
  lastFetchedAt: string | null;
  lastError: string | null;
}

export interface RssRule {
  id: number;
  name: string;
  enabled: boolean;
  feedId: number | null;
  service: string | null;
  mediaId: number | null;
  titleContains: string | null;
  excludeContains: string | null;
  releaseGroup: string | null;
  minResolution: number | null;
  season: number | null;
  episodeFrom: number | null;
  episodeTo: number | null;
  destPath: string | null;
  category: string | null;
  paused: boolean;
  createdAt: string;
  mediaTitle: MediaTitle | null;
}

export interface RssRuleInput {
  name: string;
  enabled: boolean;
  feedId: number | null;
  service: string | null;
  mediaId: number | null;
  titleContains: string | null;
  excludeContains: string | null;
  releaseGroup: string | null;
  minResolution: number | null;
  season: number | null;
  episodeFrom: number | null;
  episodeTo: number | null;
  destPath: string | null;
  category: string | null;
  paused: boolean;
}

export interface RssHistoryEntry {
  guid: string;
  ruleId: number | null;
  ruleName: string | null;
  feedId: number | null;
  title: string;
  link: string;
  episode: number | null;
  downloadedAt: string;
}

export interface RssCheckReport {
  feedsChecked: number;
  itemsSeen: number;
  added: number;
  errors: string[];
  finishedAt: string;
}

export interface QbConfig {
  baseUrl: string;
  username: string;
  password: string;
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
