//! Thin `#[tauri::command]` wrappers. All real logic lives in `sync` / `db` /
//! `tracker`; these just adapt arguments and surface `AppError` to the frontend.

use serde::Serialize;
use tauri::State;

use crate::auth;
use crate::db::repo::{self, Account};
use crate::download::{qbittorrent::QbClient, DownloadClient, QbConfig};
use crate::error::AppResult;
use crate::library::{LibraryFile, LibraryFolder, LinkRule, OwnedMedia, ScanReport};
use crate::rss::{scheduler, CheckReport, Feed, HistoryEntry, Rule, RuleInput};
use crate::state::AppState;
use crate::sync::{self, SyncReport};
use crate::tracker::anilist::BudgetSnapshot;
use crate::tracker::model::*;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    pub anilist_client_id: Option<String>,
    pub anilist_redirect: String,
    pub accounts: Vec<Account>,
    pub sync_on_startup: bool,
    pub close_to_tray: bool,
    pub start_on_login: bool,
    pub start_minimized: bool,
    pub playback_enabled: bool,
    pub playback_mode: String,
    pub playback_window_detect: bool,
    pub monitored_players: Vec<String>,
}

#[tauri::command]
pub async fn get_settings(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> AppResult<AppSettings> {
    use tauri_plugin_autostart::ManagerExt;
    let anilist_client_id = repo::get_setting(&state.db, "anilist_client_id")
        .await?
        .and_then(|v| v.as_str().map(str::to_owned));
    // The OS registry entry is the source of truth — the user could've removed
    // it outside the app — so read it live rather than trusting our own DB copy.
    let start_on_login = app.autolaunch().is_enabled().unwrap_or(false);
    Ok(AppSettings {
        anilist_client_id,
        anilist_redirect: auth::ANILIST_REDIRECT.to_string(),
        accounts: repo::list_accounts(&state.db).await?,
        sync_on_startup: repo::get_bool_setting(&state.db, sync::SYNC_ON_STARTUP_KEY, true).await?,
        close_to_tray: state.is_close_to_tray(),
        start_on_login,
        start_minimized: repo::get_bool_setting(&state.db, sync::START_MINIMIZED_KEY, false)
            .await?,
        playback_enabled: repo::get_bool_setting(&state.db, sync::PLAYBACK_ENABLED_KEY, true)
            .await?,
        playback_mode: repo::get_setting(&state.db, sync::PLAYBACK_MODE_KEY)
            .await?
            .and_then(|v| v.as_str().map(str::to_owned))
            .unwrap_or_else(|| "confirm".into()),
        playback_window_detect: repo::get_bool_setting(
            &state.db,
            sync::PLAYBACK_WINDOW_DETECT_KEY,
            false,
        )
        .await?,
        monitored_players: repo::enabled_players(&state.db).await?,
    })
}

#[tauri::command]
pub async fn set_sync_on_startup(state: State<'_, AppState>, enabled: bool) -> AppResult<()> {
    repo::set_bool_setting(&state.db, sync::SYNC_ON_STARTUP_KEY, enabled).await
}

/// M7 — tray + background running.
#[tauri::command]
pub async fn set_close_to_tray(state: State<'_, AppState>, enabled: bool) -> AppResult<()> {
    repo::set_bool_setting(&state.db, sync::CLOSE_TO_TRAY_KEY, enabled).await?;
    state.set_close_to_tray_flag(enabled);
    Ok(())
}

#[tauri::command]
pub async fn set_start_on_login(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    enabled: bool,
) -> AppResult<()> {
    use tauri_plugin_autostart::ManagerExt;
    let al = app.autolaunch();
    let result = if enabled { al.enable() } else { al.disable() };
    result.map_err(|e| {
        crate::error::AppError::other(format!("Couldn't update the login-launch setting: {e}"))
    })?;
    repo::set_bool_setting(&state.db, sync::START_ON_LOGIN_KEY, enabled).await
}

#[tauri::command]
pub async fn set_start_minimized(state: State<'_, AppState>, enabled: bool) -> AppResult<()> {
    repo::set_bool_setting(&state.db, sync::START_MINIMIZED_KEY, enabled).await
}

/// M6a — playback detection.
#[tauri::command]
pub async fn set_playback_enabled(state: State<'_, AppState>, enabled: bool) -> AppResult<()> {
    repo::set_bool_setting(&state.db, sync::PLAYBACK_ENABLED_KEY, enabled).await
}

#[tauri::command]
pub async fn set_playback_mode(state: State<'_, AppState>, mode: String) -> AppResult<()> {
    let mode = if mode == "silent" { "silent" } else { "confirm" };
    repo::set_setting(
        &state.db,
        sync::PLAYBACK_MODE_KEY,
        &serde_json::Value::String(mode.into()),
    )
    .await
}

/// Episodes currently being tracked for auto-progress ("Now watching").
#[tauri::command]
pub fn now_watching(state: State<'_, AppState>) -> Vec<crate::playback::WatchSessionView> {
    state.playback.list()
}

/// M6b — foreground-window detection.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KnownPlayer {
    pub exe: String,
    pub label: String,
}

#[tauri::command]
pub fn known_players() -> Vec<KnownPlayer> {
    crate::playback::detect::KNOWN_PLAYERS
        .iter()
        .map(|(exe, label)| KnownPlayer {
            exe: exe.to_string(),
            label: label.to_string(),
        })
        .collect()
}

#[tauri::command]
pub async fn set_playback_window_detect(
    state: State<'_, AppState>,
    enabled: bool,
) -> AppResult<()> {
    repo::set_bool_setting(&state.db, sync::PLAYBACK_WINDOW_DETECT_KEY, enabled).await
}

#[tauri::command]
pub async fn set_monitored_players(
    state: State<'_, AppState>,
    players: Vec<String>,
) -> AppResult<()> {
    repo::set_enabled_players(&state.db, &players).await
}

#[tauri::command]
pub async fn set_anilist_client_id(state: State<'_, AppState>, client_id: String) -> AppResult<()> {
    let trimmed = client_id.trim();
    repo::set_setting(
        &state.db,
        "anilist_client_id",
        &serde_json::Value::String(trimmed.to_string()),
    )
    .await
}

/// Returns the URL the frontend should open in the system browser to start login.
#[tauri::command]
pub async fn anilist_login_url(state: State<'_, AppState>) -> AppResult<String> {
    let client_id = repo::get_setting(&state.db, "anilist_client_id")
        .await?
        .and_then(|v| v.as_str().map(str::to_owned))
        .filter(|s| !s.is_empty())
        .ok_or_else(|| {
            crate::error::AppError::other(
                "Set your AniList client ID in Settings first (anilist.co/settings/developer).",
            )
        })?;
    Ok(auth::anilist_authorize_url(&client_id))
}

/// Complete login from a redirect URL (used as a manual fallback if the deep
/// link doesn't fire, e.g. during development).
#[tauri::command]
pub async fn anilist_complete_login(
    state: State<'_, AppState>,
    redirect_url: String,
) -> AppResult<Account> {
    let token = auth::parse_anilist_redirect(&redirect_url)
        .ok_or_else(|| crate::error::AppError::other("no access_token in that URL"))?;
    sync::connect_anilist(&state, &token).await
}

/// Complete login from a raw access token (AniList lets developers mint one
/// directly on the developer settings page — handy for first-run testing).
#[tauri::command]
pub async fn anilist_connect_token(
    state: State<'_, AppState>,
    token: String,
) -> AppResult<Account> {
    sync::connect_anilist(&state, token.trim()).await
}

#[tauri::command]
pub async fn list_accounts(state: State<'_, AppState>) -> AppResult<Vec<Account>> {
    repo::list_accounts(&state.db).await
}

#[tauri::command]
pub async fn disconnect_account(
    state: State<'_, AppState>,
    service: Option<String>,
) -> AppResult<()> {
    sync::disconnect(&state, service.as_deref()).await
}

#[tauri::command]
pub async fn get_library(
    state: State<'_, AppState>,
    service: Option<String>,
) -> AppResult<Vec<MediaListEntry>> {
    sync::library(&state, service.as_deref()).await
}

#[tauri::command]
pub async fn get_stats(
    state: State<'_, AppState>,
    service: Option<String>,
) -> AppResult<crate::stats::StatsData> {
    sync::stats(&state, service.as_deref()).await
}

#[tauri::command]
pub async fn sync_now(
    state: State<'_, AppState>,
    service: Option<String>,
) -> AppResult<SyncReport> {
    sync::full_sync(&state, service.as_deref()).await
}

#[tauri::command]
pub async fn get_media(
    state: State<'_, AppState>,
    service: Option<String>,
    media_id: i64,
) -> AppResult<Media> {
    sync::get_media(&state, service.as_deref(), media_id).await
}

#[tauri::command]
pub async fn edit_entry(
    state: State<'_, AppState>,
    service: Option<String>,
    patch: EntryPatch,
) -> AppResult<MediaListEntry> {
    sync::edit_entry(&state, service.as_deref(), patch).await
}

#[tauri::command]
pub async fn remove_entry(
    state: State<'_, AppState>,
    service: Option<String>,
    media_id: i64,
) -> AppResult<()> {
    sync::remove_entry(&state, service.as_deref(), media_id).await
}

#[tauri::command]
pub async fn search_anime(
    state: State<'_, AppState>,
    service: Option<String>,
    query: String,
) -> AppResult<Vec<Media>> {
    sync::search(&state, service.as_deref(), &query).await
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CurrentSeason {
    pub year: i32,
    pub season: String,
}

#[tauri::command]
pub fn current_season() -> CurrentSeason {
    let (year, season) = sync::current_season();
    CurrentSeason {
        year,
        season: season.as_str().to_string(),
    }
}

#[tauri::command]
pub async fn get_season(
    state: State<'_, AppState>,
    year: i32,
    season: String,
) -> AppResult<Vec<Media>> {
    sync::season(&state, year, &season).await
}

#[tauri::command]
pub async fn budget_snapshot(state: State<'_, AppState>) -> AppResult<BudgetSnapshot> {
    Ok(state.gateway().budget().await)
}

#[tauri::command]
pub async fn last_sync(
    state: State<'_, AppState>,
    service: Option<String>,
) -> AppResult<Option<String>> {
    sync::last_sync(&state, service.as_deref()).await
}

// --------------------------------------------------------------------------- //
// Local library (M3)
// --------------------------------------------------------------------------- //

#[tauri::command]
pub async fn library_folders(state: State<'_, AppState>) -> AppResult<Vec<LibraryFolder>> {
    sync::library_folders(&state).await
}

#[tauri::command]
pub async fn add_library_folder(
    state: State<'_, AppState>,
    path: String,
) -> AppResult<LibraryFolder> {
    sync::add_library_folder(&state, path.trim()).await
}

#[tauri::command]
pub async fn remove_library_folder(state: State<'_, AppState>, id: i64) -> AppResult<()> {
    sync::remove_library_folder(&state, id).await
}

#[tauri::command]
pub async fn set_library_folder_enabled(
    state: State<'_, AppState>,
    id: i64,
    enabled: bool,
) -> AppResult<()> {
    sync::set_library_folder_enabled(&state, id, enabled).await
}

#[tauri::command]
pub async fn scan_library(state: State<'_, AppState>) -> AppResult<ScanReport> {
    sync::scan_library(&state).await
}

#[tauri::command]
pub async fn library_files(state: State<'_, AppState>) -> AppResult<Vec<LibraryFile>> {
    sync::library_files(&state).await
}

#[tauri::command]
pub async fn library_owned(state: State<'_, AppState>) -> AppResult<Vec<OwnedMedia>> {
    sync::owned_media(&state).await
}

/// Open the local file for `episode` of `media_id` in the OS default player.
#[tauri::command]
pub async fn play_episode(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    media_id: i64,
    episode: i64,
    service: Option<String>,
) -> AppResult<()> {
    use tauri_plugin_opener::OpenerExt;
    let path = sync::episode_file_path(&state, service.as_deref(), media_id, episode).await?;
    app.opener()
        .open_path(path, None::<&str>)
        .map_err(|e| crate::error::AppError::other(format!("Couldn't open the file: {e}")))?;

    // M6a: track this episode for auto-progress detection. Best-effort — a
    // failure here shouldn't stop the file from having opened.
    match sync::prepare_watch_session(&state, service.as_deref(), media_id, episode).await {
        Ok(Some(session)) => state.playback.start(session),
        Ok(None) => {}
        Err(e) => tracing::warn!(?e, "couldn't prepare playback tracking"),
    }
    Ok(())
}

#[tauri::command]
pub async fn link_library_files(
    state: State<'_, AppState>,
    file_ids: Vec<i64>,
    service: Option<String>,
    media_id: i64,
    remember: bool,
) -> AppResult<()> {
    sync::link_library_files(&state, &file_ids, service.as_deref(), media_id, remember).await
}

#[tauri::command]
pub async fn unlink_library_file(state: State<'_, AppState>, file_id: i64) -> AppResult<()> {
    sync::unlink_library_file(&state, file_id).await
}

#[tauri::command]
pub async fn library_link_rules(state: State<'_, AppState>) -> AppResult<Vec<LinkRule>> {
    sync::library_link_rules(&state).await
}

#[tauri::command]
pub async fn delete_link_rule(state: State<'_, AppState>, id: i64) -> AppResult<()> {
    sync::delete_link_rule(&state, id).await
}

// --------------------------------------------------------------------------- //
// RSS auto-download (M5)
// --------------------------------------------------------------------------- //

#[tauri::command]
pub async fn rss_feeds(state: State<'_, AppState>) -> AppResult<Vec<Feed>> {
    repo::list_feeds(&state.db).await
}

#[tauri::command]
pub async fn add_rss_feed(
    state: State<'_, AppState>,
    name: String,
    url: String,
) -> AppResult<Feed> {
    let url = url.trim();
    if !(url.starts_with("http://") || url.starts_with("https://")) {
        return Err(crate::error::AppError::other(
            "Enter the feed's http(s) URL.",
        ));
    }
    let name = name.trim();
    let name = if name.is_empty() { url } else { name };
    repo::add_feed(&state.db, name, url).await
}

#[tauri::command]
pub async fn remove_rss_feed(state: State<'_, AppState>, id: i64) -> AppResult<()> {
    repo::remove_feed(&state.db, id).await
}

#[tauri::command]
pub async fn set_rss_feed_enabled(
    state: State<'_, AppState>,
    id: i64,
    enabled: bool,
) -> AppResult<()> {
    repo::set_feed_enabled(&state.db, id, enabled).await
}

#[tauri::command]
pub async fn rss_rules(state: State<'_, AppState>) -> AppResult<Vec<Rule>> {
    repo::list_rules(&state.db).await
}

/// Create (`id` omitted) or update (`id` set) a rule.
#[tauri::command]
pub async fn save_rss_rule(
    state: State<'_, AppState>,
    id: Option<i64>,
    rule: RuleInput,
) -> AppResult<Rule> {
    if rule.name.trim().is_empty() {
        return Err(crate::error::AppError::other("Give the rule a name."));
    }
    match id {
        Some(id) => repo::update_rule(&state.db, id, &rule).await,
        None => repo::insert_rule(&state.db, &rule).await,
    }
}

#[tauri::command]
pub async fn delete_rss_rule(state: State<'_, AppState>, id: i64) -> AppResult<()> {
    repo::delete_rule(&state.db, id).await
}

#[tauri::command]
pub async fn set_rss_rule_enabled(
    state: State<'_, AppState>,
    id: i64,
    enabled: bool,
) -> AppResult<()> {
    repo::set_rule_enabled(&state.db, id, enabled).await
}

#[tauri::command]
pub async fn rss_history(
    state: State<'_, AppState>,
    limit: Option<i64>,
) -> AppResult<Vec<HistoryEntry>> {
    repo::list_history(&state.db, limit.unwrap_or(100).clamp(1, 500)).await
}

/// Wipe the download history. The next check will re-evaluate current feed
/// items, so a still-matching release can be grabbed again.
#[tauri::command]
pub async fn clear_rss_history(state: State<'_, AppState>) -> AppResult<u64> {
    repo::clear_history(&state.db).await
}

#[tauri::command]
pub async fn check_feeds_now(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> AppResult<CheckReport> {
    scheduler::check_all_feeds(&state, &app).await
}

#[tauri::command]
pub async fn get_qb_config(state: State<'_, AppState>) -> AppResult<QbConfig> {
    repo::get_qb_config(&state.db).await
}

#[tauri::command]
pub async fn set_qb_config(state: State<'_, AppState>, config: QbConfig) -> AppResult<()> {
    repo::set_qb_config(&state.db, &config).await
}

/// Test a connection with the given (possibly unsaved) settings; returns the
/// qBittorrent version string on success.
#[tauri::command]
pub async fn test_qb_connection(config: QbConfig) -> AppResult<String> {
    QbClient::new(config)?.test_connection().await
}

#[tauri::command]
pub async fn rss_poll_enabled(state: State<'_, AppState>) -> AppResult<bool> {
    repo::get_bool_setting(&state.db, scheduler::RSS_POLL_KEY, true).await
}

#[tauri::command]
pub async fn set_rss_poll_enabled(
    state: State<'_, AppState>,
    enabled: bool,
) -> AppResult<()> {
    repo::set_bool_setting(&state.db, scheduler::RSS_POLL_KEY, enabled).await
}
