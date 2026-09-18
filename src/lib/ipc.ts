import { invoke } from "@tauri-apps/api/core";
import type {
  Account,
  AppSettings,
  BudgetSnapshot,
  CurrentSeason,
  EntryPatch,
  KnownPlayer,
  LibraryFile,
  LibraryFolder,
  LinkRule,
  Media,
  MediaListEntry,
  OwnedMedia,
  PlaybackMode,
  PlayerIntegrationKind,
  QbConfig,
  RssCheckReport,
  RssFeed,
  RssHistoryEntry,
  RssRule,
  RssRuleInput,
  ScanReport,
  ServiceKind,
  StatsData,
  SyncReport,
  WatchSessionView,
} from "./types";

/** Typed wrappers around every Tauri command. Nothing else calls `invoke`. */
export const api = {
  getSettings: () => invoke<AppSettings>("get_settings"),

  setAnilistClientId: (clientId: string) =>
    invoke<void>("set_anilist_client_id", { clientId }),

  setSyncOnStartup: (enabled: boolean) =>
    invoke<void>("set_sync_on_startup", { enabled }),

  setCloseToTray: (enabled: boolean) =>
    invoke<void>("set_close_to_tray", { enabled }),

  setStartOnLogin: (enabled: boolean) =>
    invoke<void>("set_start_on_login", { enabled }),

  setStartMinimized: (enabled: boolean) =>
    invoke<void>("set_start_minimized", { enabled }),

  setPlaybackEnabled: (enabled: boolean) =>
    invoke<void>("set_playback_enabled", { enabled }),

  setPlaybackMode: (mode: PlaybackMode) =>
    invoke<void>("set_playback_mode", { mode }),

  nowWatching: () => invoke<WatchSessionView[]>("now_watching"),

  knownPlayers: () => invoke<KnownPlayer[]>("known_players"),

  setPlaybackWindowDetect: (enabled: boolean) =>
    invoke<void>("set_playback_window_detect", { enabled }),

  setMonitoredPlayers: (players: string[]) =>
    invoke<void>("set_monitored_players", { players }),

  setPlayerIntegration: (kind: PlayerIntegrationKind | null, path: string | null) =>
    invoke<void>("set_player_integration", { kind, path }),

  anilistLoginUrl: () => invoke<string>("anilist_login_url"),

  anilistCompleteLogin: (redirectUrl: string) =>
    invoke<Account>("anilist_complete_login", { redirectUrl }),

  anilistConnectToken: (token: string) =>
    invoke<Account>("anilist_connect_token", { token }),

  listAccounts: () => invoke<Account[]>("list_accounts"),

  disconnect: (service?: ServiceKind) =>
    invoke<void>("disconnect_account", { service }),

  getLibrary: (service?: ServiceKind) =>
    invoke<MediaListEntry[]>("get_library", { service }),

  getStats: (service?: ServiceKind) => invoke<StatsData>("get_stats", { service }),

  syncNow: (service?: ServiceKind) =>
    invoke<SyncReport>("sync_now", { service }),

  getMedia: (mediaId: number, service?: ServiceKind) =>
    invoke<Media>("get_media", { mediaId, service }),

  editEntry: (patch: EntryPatch, service?: ServiceKind) =>
    invoke<MediaListEntry>("edit_entry", { patch, service }),

  removeEntry: (mediaId: number, service?: ServiceKind) =>
    invoke<void>("remove_entry", { mediaId, service }),

  searchAnime: (query: string, service?: ServiceKind) =>
    invoke<Media[]>("search_anime", { query, service }),

  currentSeason: () => invoke<CurrentSeason>("current_season"),

  getSeason: (year: number, season: string) =>
    invoke<Media[]>("get_season", { year, season }),

  budgetSnapshot: () => invoke<BudgetSnapshot>("budget_snapshot"),

  lastSync: (service?: ServiceKind) =>
    invoke<string | null>("last_sync", { service }),

  // Local library (M3)
  libraryFolders: () => invoke<LibraryFolder[]>("library_folders"),

  addLibraryFolder: (path: string) =>
    invoke<LibraryFolder>("add_library_folder", { path }),

  removeLibraryFolder: (id: number) =>
    invoke<void>("remove_library_folder", { id }),

  setLibraryFolderEnabled: (id: number, enabled: boolean) =>
    invoke<void>("set_library_folder_enabled", { id, enabled }),

  scanLibrary: () => invoke<ScanReport>("scan_library"),

  libraryFiles: () => invoke<LibraryFile[]>("library_files"),

  libraryOwned: () => invoke<OwnedMedia[]>("library_owned"),

  playEpisode: (mediaId: number, episode: number, service?: ServiceKind) =>
    invoke<void>("play_episode", { mediaId, episode, service }),

  openMediaFolder: (mediaId: number) =>
    invoke<void>("open_media_folder", { mediaId }),

  linkLibraryFiles: (
    fileIds: number[],
    mediaId: number,
    remember: boolean,
    service?: ServiceKind,
  ) =>
    invoke<void>("link_library_files", { fileIds, mediaId, remember, service }),

  unlinkLibraryFile: (fileId: number) =>
    invoke<void>("unlink_library_file", { fileId }),

  libraryLinkRules: () => invoke<LinkRule[]>("library_link_rules"),

  deleteLinkRule: (id: number) => invoke<void>("delete_link_rule", { id }),

  // RSS auto-download (M5)
  rssFeeds: () => invoke<RssFeed[]>("rss_feeds"),

  addRssFeed: (name: string, url: string) =>
    invoke<RssFeed>("add_rss_feed", { name, url }),

  removeRssFeed: (id: number) => invoke<void>("remove_rss_feed", { id }),

  setRssFeedEnabled: (id: number, enabled: boolean) =>
    invoke<void>("set_rss_feed_enabled", { id, enabled }),

  rssRules: () => invoke<RssRule[]>("rss_rules"),

  saveRssRule: (rule: RssRuleInput, id?: number) =>
    invoke<RssRule>("save_rss_rule", { id: id ?? null, rule }),

  deleteRssRule: (id: number) => invoke<void>("delete_rss_rule", { id }),

  setRssRuleEnabled: (id: number, enabled: boolean) =>
    invoke<void>("set_rss_rule_enabled", { id, enabled }),

  rssHistory: (limit?: number) =>
    invoke<RssHistoryEntry[]>("rss_history", { limit: limit ?? null }),

  clearRssHistory: () => invoke<number>("clear_rss_history"),

  checkFeedsNow: () => invoke<RssCheckReport>("check_feeds_now"),

  getQbConfig: () => invoke<QbConfig>("get_qb_config"),

  setQbConfig: (config: QbConfig) => invoke<void>("set_qb_config", { config }),

  testQbConnection: (config: QbConfig) =>
    invoke<string>("test_qb_connection", { config }),

  rssPollEnabled: () => invoke<boolean>("rss_poll_enabled"),

  setRssPollEnabled: (enabled: boolean) =>
    invoke<void>("set_rss_poll_enabled", { enabled }),
};
