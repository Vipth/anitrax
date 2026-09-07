//! Thin `#[tauri::command]` wrappers. All real logic lives in `sync` / `db` /
//! `tracker`; these just adapt arguments and surface `AppError` to the frontend.

use serde::Serialize;
use tauri::State;

use crate::auth;
use crate::db::repo::{self, Account};
use crate::error::AppResult;
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
    })
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
