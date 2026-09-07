//! Orchestration: combine the tracker service with the local cache. Commands
//! call into here; this module owns the "when do we hit the network" policy.

use std::time::Duration;

use serde::Serialize;

use crate::auth;
use crate::db::repo;
use crate::error::{AppError, AppResult};
use crate::state::AppState;
use crate::tracker::model::*;
use crate::tracker::TrackerService;

/// How long a cached media metadata row stays fresh before we'd refetch it.
pub const META_TTL: Duration = Duration::from_secs(14 * 24 * 3600);
/// How stale the list can get on launch before we auto-sync.
pub const LAUNCH_SYNC_AFTER: Duration = Duration::from_secs(30 * 60);
/// Background refresh cadence (only while the window is focused).
pub const BACKGROUND_SYNC_EVERY: Duration = Duration::from_secs(30 * 60);
/// Debounce before the push worker flushes dirty rows.
pub const PUSH_DEBOUNCE: Duration = Duration::from_secs(3);
/// If a push leaves rows still dirty (offline / rate-limited / API down), retry
/// on this cadence until the queue clears.
pub const PUSH_RETRY_EVERY: Duration = Duration::from_secs(90);

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncReport {
    pub service: String,
    pub entries: usize,
    pub finished_at: String,
}

fn parse_service(s: Option<&str>) -> ServiceKind {
    s.and_then(|s| s.parse().ok()).unwrap_or(ServiceKind::AniList)
}

/// Connect an AniList account from an access token we already hold.
pub async fn connect_anilist(state: &AppState, token: &str) -> AppResult<repo::Account> {
    let viewer = state.anilist.viewer(token).await?;
    auth::store_token(ServiceKind::AniList, token)?;
    repo::upsert_account(&state.db, ServiceKind::AniList, &viewer, true).await?;
    // First pull so the UI has something immediately.
    let _ = full_sync(state, Some("anilist")).await?;
    repo::get_account(&state.db, ServiceKind::AniList)
        .await?
        .ok_or_else(|| AppError::other("account missing after connect"))
}

pub async fn disconnect(state: &AppState, service: Option<&str>) -> AppResult<()> {
    let svc = parse_service(service);
    auth::delete_token(svc)?;
    repo::delete_account(&state.db, svc).await?;
    Ok(())
}

/// Pull the entire list from the service and reconcile it into the cache.
pub async fn full_sync(state: &AppState, service: Option<&str>) -> AppResult<SyncReport> {
    let svc = parse_service(service);
    let token = auth::require_token(svc)?;
    let account = repo::get_account(&state.db, svc)
        .await?
        .ok_or_else(|| AppError::NotAuthenticated(svc.as_str().into()))?;
    let user_id: i64 = account
        .user_id
        .parse()
        .map_err(|_| AppError::other("stored user id is not numeric"))?;

    let service_impl: &dyn TrackerService = match svc {
        ServiceKind::AniList => state.anilist.as_ref(),
        ServiceKind::Kitsu => return Err(AppError::other("Kitsu sync arrives in a later milestone")),
    };

    // Send local edits before pulling, so the server state we reconcile against
    // already reflects them. Best-effort — a failed push just leaves rows dirty.
    if repo::dirty_count(&state.db, svc).await? > 0 {
        let _ = push_dirty(state, svc).await;
    }

    let entries = service_impl.full_list(&token, user_id).await?;
    repo::replace_list_from_remote(&state.db, svc, &entries).await?;

    Ok(SyncReport {
        service: svc.as_str().into(),
        entries: entries.len(),
        finished_at: chrono::Utc::now().to_rfc3339(),
    })
}

pub const SYNC_ON_STARTUP_KEY: &str = "sync_on_startup";

/// Auto-sync on launch only if the list is stale (keeps startup at zero requests
/// when the cache is warm). Skipped entirely when the user turns it off.
pub async fn sync_on_launch_if_stale(state: &AppState) -> AppResult<()> {
    if !repo::get_bool_setting(&state.db, SYNC_ON_STARTUP_KEY, true).await? {
        return Ok(());
    }
    for svc in [ServiceKind::AniList] {
        if auth::load_token(svc)?.is_none() {
            continue;
        }
        let stale = match repo::last_full_sync(&state.db, svc).await? {
            None => true,
            Some(ts) => chrono::DateTime::parse_from_rfc3339(&ts)
                .map(|t| {
                    (chrono::Utc::now() - t.with_timezone(&chrono::Utc)).to_std().unwrap_or(LAUNCH_SYNC_AFTER)
                        >= LAUNCH_SYNC_AFTER
                })
                .unwrap_or(true),
        };
        if stale {
            let _ = full_sync(state, Some(svc.as_str())).await?;
        }
    }
    Ok(())
}

/// Read the list straight from cache — no network.
pub async fn library(state: &AppState, service: Option<&str>) -> AppResult<Vec<MediaListEntry>> {
    repo::all_entries(&state.db, parse_service(service)).await
}

/// Ensure we have a media row. Serves it from the cache when the cached copy is
/// still within [`META_TTL`]; refetches (and re-caches) when it's stale or
/// missing, falling back to the stale copy if the network is unavailable.
pub async fn ensure_media(state: &AppState, svc: ServiceKind, media_id: i64) -> AppResult<Media> {
    if let Some((media, fetched_at)) = repo::cached_media(&state.db, svc, media_id).await? {
        let fresh = chrono::DateTime::parse_from_rfc3339(&fetched_at)
            .ok()
            .and_then(|t| {
                (chrono::Utc::now() - t.with_timezone(&chrono::Utc))
                    .to_std()
                    .ok()
            })
            .map(|age| age < META_TTL)
            .unwrap_or(false);
        if fresh {
            return Ok(media);
        }
        // Stale — try to refresh, but a stale copy beats an error.
        return match refetch_media(state, svc, media_id).await {
            Ok(m) => Ok(m),
            Err(_) => Ok(media),
        };
    }
    refetch_media(state, svc, media_id).await
}

async fn refetch_media(state: &AppState, svc: ServiceKind, media_id: i64) -> AppResult<Media> {
    let token = auth::load_token(svc)?;
    let fetched = match svc {
        ServiceKind::AniList => {
            state.anilist.media_batch(token.as_deref(), &[media_id]).await?
        }
        ServiceKind::Kitsu => return Err(AppError::other("Kitsu arrives in a later milestone")),
    };
    let media = fetched
        .into_iter()
        .next()
        .ok_or_else(|| AppError::other("media not found"))?;
    repo::upsert_media(&state.db, &media).await?;
    Ok(media)
}

pub async fn get_media(state: &AppState, service: Option<&str>, media_id: i64) -> AppResult<Media> {
    ensure_media(state, parse_service(service), media_id).await
}

/// Apply an edit optimistically to the cache and schedule a background push.
pub async fn edit_entry(
    state: &AppState,
    service: Option<&str>,
    patch: EntryPatch,
) -> AppResult<MediaListEntry> {
    let svc = parse_service(service);
    let media = ensure_media(state, svc, patch.media_id).await?;
    let entry = repo::apply_local_patch(&state.db, svc, &media, &patch).await?;
    state.push.nudge();
    Ok(entry)
}

pub async fn remove_entry(
    state: &AppState,
    service: Option<&str>,
    media_id: i64,
) -> AppResult<()> {
    let svc = parse_service(service);
    repo::mark_entry_deleted(&state.db, svc, media_id).await?;
    state.push.nudge();
    Ok(())
}

/// Whether any local edits are still waiting to reach the service.
pub async fn has_pending_pushes(state: &AppState, svc: ServiceKind) -> bool {
    repo::dirty_count(&state.db, svc).await.unwrap_or(0) > 0
}

/// Flush every dirty row for a service. Naturally paced by the gateway.
pub async fn push_dirty(state: &AppState, svc: ServiceKind) -> AppResult<()> {
    let token = match auth::load_token(svc)? {
        Some(t) => t,
        None => return Ok(()),
    };
    let service_impl: &dyn TrackerService = match svc {
        ServiceKind::AniList => state.anilist.as_ref(),
        ServiceKind::Kitsu => return Ok(()),
    };

    // Deletions first.
    for (media_id, remote_id) in repo::dirty_deleted(&state.db, svc).await? {
        if let Some(rid) = remote_id {
            match service_impl.delete_entry(&token, rid).await {
                Ok(()) => repo::remove_entry_row(&state.db, svc, media_id).await?,
                // Transient: keep the row dirty and retry on the next nudge/sync.
                Err(AppError::RateLimited { .. }) | Err(AppError::ServiceUnavailable { .. }) => {
                    return Ok(())
                }
                Err(e) => tracing::warn!(?e, media_id, "delete push failed"),
            }
        } else {
            repo::remove_entry_row(&state.db, svc, media_id).await?;
        }
    }

    // Upserts.
    for entry in repo::dirty_entries(&state.db, svc).await? {
        let patch = EntryPatch {
            media_id: entry.media.id.id,
            remote_id: entry.remote_id,
            status: Some(entry.status),
            progress: Some(entry.progress),
            score_raw: Some(entry.score_raw),
            repeat: Some(entry.repeat),
            notes: entry.notes.clone(),
            started_at: None,
            completed_at: None,
        };
        match service_impl.save_entry(&token, &patch).await {
            Ok(saved) => repo::mark_entry_clean(&state.db, svc, &saved).await?,
            Err(AppError::RateLimited { .. }) | Err(AppError::ServiceUnavailable { .. }) => {
                return Ok(())
            }
            Err(e) => tracing::warn!(?e, media_id = entry.media.id.id, "entry push failed"),
        }
    }
    Ok(())
}

/// Title search for the discover screen. Results are also written to the media
/// cache so opening a result is instant and costs no request.
pub async fn search(state: &AppState, service: Option<&str>, query: &str) -> AppResult<Vec<Media>> {
    let svc = parse_service(service);
    if query.trim().len() < 2 {
        return Ok(vec![]);
    }
    let token = auth::load_token(svc)?;
    let results = match svc {
        ServiceKind::AniList => state.anilist.search(token.as_deref(), query).await?,
        ServiceKind::Kitsu => return Err(AppError::other("Kitsu arrives in a later milestone")),
    };
    repo::upsert_media_bulk(&state.db, &results).await?;
    Ok(results)
}

pub async fn last_sync(state: &AppState, service: Option<&str>) -> AppResult<Option<String>> {
    repo::last_full_sync(&state.db, parse_service(service)).await
}
