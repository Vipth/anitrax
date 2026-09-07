//! Orchestration: combine the tracker service with the local cache. Commands
//! call into here; this module owns the "when do we hit the network" policy.

use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::Serialize;

use crate::auth;
use crate::db::repo;
use crate::error::{AppError, AppResult};
use crate::library::{matcher, scanner, LibraryFile, LibraryFolder, OwnedMedia, ScanReport};
use crate::state::AppState;
use crate::tracker::model::*;
use crate::tracker::TrackerService;

/// How long a cached media metadata row stays fresh before we'd refetch it.
pub const META_TTL: Duration = Duration::from_secs(14 * 24 * 3600);
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

/// Full-sync every connected service on launch. Skipped entirely when the user
/// turns off "Sync on startup".
pub async fn sync_on_launch(state: &AppState) -> AppResult<()> {
    if !repo::get_bool_setting(&state.db, SYNC_ON_STARTUP_KEY, true).await? {
        return Ok(());
    }
    for svc in [ServiceKind::AniList] {
        if auth::load_token(svc)?.is_none() {
            continue;
        }
        let _ = full_sync(state, Some(svc.as_str())).await?;
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

// --------------------------------------------------------------------------- //
// Local library (M3). All offline — the only network touch is an optional
// cache-miss media lookup when the user links a file by hand.
// --------------------------------------------------------------------------- //

pub async fn library_folders(state: &AppState) -> AppResult<Vec<LibraryFolder>> {
    repo::list_library_folders(&state.db).await
}

pub async fn library_files(state: &AppState) -> AppResult<Vec<LibraryFile>> {
    repo::library_files(&state.db).await
}

pub async fn owned_media(state: &AppState) -> AppResult<Vec<OwnedMedia>> {
    repo::owned_media(&state.db).await
}

/// Add a folder and immediately scan it. Errors if the path isn't a directory.
pub async fn add_library_folder(state: &AppState, path: &str) -> AppResult<LibraryFolder> {
    let p = Path::new(path);
    if !p.is_dir() {
        return Err(AppError::other("That path isn't a folder we can read."));
    }
    let folder = repo::add_library_folder(&state.db, path).await?;
    scan_folders(state, &[(folder.id, folder.path.clone())]).await?;
    state.watcher.refresh(state).await;
    repo::get_library_folder(&state.db, folder.id)
        .await?
        .ok_or_else(|| AppError::other("folder vanished after add"))
}

pub async fn remove_library_folder(state: &AppState, id: i64) -> AppResult<()> {
    repo::remove_library_folder(&state.db, id).await?;
    state.watcher.refresh(state).await;
    Ok(())
}

pub async fn set_library_folder_enabled(
    state: &AppState,
    id: i64,
    enabled: bool,
) -> AppResult<()> {
    repo::set_library_folder_enabled(&state.db, id, enabled).await?;
    state.watcher.refresh(state).await;
    if enabled {
        if let Some(f) = repo::get_library_folder(&state.db, id).await? {
            scan_folders(state, &[(f.id, f.path)]).await?;
        }
    }
    Ok(())
}

/// Manually bind a file to a media row. Ensures the media is cached so the
/// library join has a title to show (one cache-first lookup, no forced fetch
/// when it's already present).
pub async fn link_library_file(
    state: &AppState,
    file_id: i64,
    service: Option<&str>,
    media_id: i64,
) -> AppResult<()> {
    let svc = parse_service(service);
    if repo::get_library_file(&state.db, file_id).await?.is_none() {
        return Err(AppError::other("That library file no longer exists."));
    }
    let _ = ensure_media(state, svc, media_id).await?;
    repo::set_file_match(&state.db, file_id, svc, media_id, "manual", None).await
}

pub async fn unlink_library_file(state: &AppState, file_id: i64) -> AppResult<()> {
    repo::clear_file_match(&state.db, file_id).await
}

/// Rescan every enabled folder.
pub async fn scan_library(state: &AppState) -> AppResult<ScanReport> {
    let folders = repo::enabled_library_folders(&state.db).await?;
    scan_folders(state, &folders).await
}

/// Rescan the folders that contain `roots` (used by the filesystem watcher).
pub async fn rescan_paths(state: &AppState, roots: &[PathBuf]) -> AppResult<ScanReport> {
    let all = repo::enabled_library_folders(&state.db).await?;
    let hit: Vec<(i64, String)> = all
        .into_iter()
        .filter(|(_, p)| roots.iter().any(|r| r == Path::new(p)))
        .collect();
    scan_folders(state, &hit).await
}

async fn scan_folders(state: &AppState, folders: &[(i64, String)]) -> AppResult<ScanReport> {
    let mut report = ScanReport {
        folders: folders.len(),
        ..Default::default()
    };

    for (folder_id, path) in folders {
        let root = PathBuf::from(path);
        let paths = tokio::task::spawn_blocking(move || scanner::walk(&root))
            .await
            .unwrap_or_default();

        let mut present: Vec<String> = Vec::with_capacity(paths.len());
        for p in paths {
            present.push(p.to_string_lossy().into_owned());
            let scanned = tokio::task::spawn_blocking({
                let p = p.clone();
                move || scanner::scan_file(&p)
            })
            .await
            .ok()
            .flatten();
            if let Some(sf) = scanned {
                report.files_seen += 1;
                repo::upsert_library_file(&state.db, *folder_id, &sf).await?;
            }
        }

        report.files_removed +=
            repo::prune_missing_files(&state.db, *folder_id, &present).await? as usize;
        repo::mark_library_folder_scanned(&state.db, *folder_id).await?;
    }

    let (auto, unmatched) = run_matcher(state).await?;
    report.auto_matched = auto;
    report.unmatched = unmatched;
    report.finished_at = chrono::Utc::now().to_rfc3339();
    Ok(report)
}

/// Try to match every still-unmatched file against the media cache. Manual
/// links are never touched. Returns `(newly auto-matched, still unmatched)`.
async fn run_matcher(state: &AppState) -> AppResult<(usize, usize)> {
    let pending = repo::files_needing_match(&state.db).await?;
    if pending.is_empty() {
        return Ok((0, 0));
    }
    let index = repo::media_match_index(&state.db).await?;
    let mut auto = 0;
    for (id, title, season, year) in &pending {
        let Some(title) = title else { continue };
        if let Some(m) = matcher::best_match(title, *season, year.map(|y| y as i32), &index) {
            repo::set_file_match(&state.db, *id, m.service, m.media_id, "auto", Some(m.score))
                .await?;
            auto += 1;
        }
    }
    let still = repo::files_needing_match(&state.db).await?.len();
    Ok((auto, still))
}
