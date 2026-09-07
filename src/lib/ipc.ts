import { invoke } from "@tauri-apps/api/core";
import type {
  Account,
  AppSettings,
  BudgetSnapshot,
  EntryPatch,
  Media,
  MediaListEntry,
  ServiceKind,
  SyncReport,
} from "./types";

/** Typed wrappers around every Tauri command. Nothing else calls `invoke`. */
export const api = {
  getSettings: () => invoke<AppSettings>("get_settings"),

  setAnilistClientId: (clientId: string) =>
    invoke<void>("set_anilist_client_id", { clientId }),

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

  budgetSnapshot: () => invoke<BudgetSnapshot>("budget_snapshot"),

  lastSync: (service?: ServiceKind) =>
    invoke<string | null>("last_sync", { service }),
};
