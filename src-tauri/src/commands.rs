//! Thin `#[tauri::command]` wrappers. All real logic lives in `sync` / `db` /
//! `tracker`; these just adapt arguments and surface `AppError` to the frontend.

use serde::Serialize;
use tauri::State;

use crate::auth;
use crate::db::repo::{self, Account};
use crate::error::AppResult;
use crate::library::{LibraryFile, LibraryFolder, LinkRule, OwnedMedia, ScanReport};
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
}

#[tauri::command]
pub async fn get_settings(state: State<'_, AppState>) -> AppResult<AppSettings> {
    let anilist_client_id = repo::get_setting(&state.db, "anilist_client_id")
        .await?
        .and_then(|v| v.as_str().map(str::to_owned));
    Ok(AppSettings {
        anilist_client_id,
        anilist_redirect: auth::ANILIST_REDIRECT.to_string(),
        accounts: repo::list_accounts(&state.db).await?,
        sync_on_startup: repo::get_bool_setting(&state.db, sync::SYNC_ON_STARTUP_KEY, true).await?,
    })
}

#[tauri::command]
pub async fn set_sync_on_startup(state: State<'_, AppState>, enabled: bool) -> AppResult<()> {
    repo::set_bool_setting(&state.db, sync::SYNC_ON_STARTUP_KEY, enabled).await
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
        .map_err(|e| crate::error::AppError::other(format!("Couldn't open the file: {e}")))
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
