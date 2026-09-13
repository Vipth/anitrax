//! Orchestration: combine the tracker service with the local cache. Commands
//! call into here; this module owns the "when do we hit the network" policy.

use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::Serialize;

use crate::auth;
use crate::db::repo;
use crate::error::{AppError, AppResult};
use crate::library::{
    matcher, scanner, LibraryFile, LibraryFolder, LinkRule, OwnedMedia, ScanReport,
};
use crate::state::AppState;
use crate::tracker::model::*;
use crate::tracker::TrackerService;

/// How long a cached media metadata row stays fresh before we'd refetch it.
pub const META_TTL: Duration = Duration::from_secs(14 * 24 * 3600);
/// The current / upcoming season keeps changing (scores, new additions); past
/// seasons are effectively frozen.
const SEASON_TTL_CURRENT: Duration = Duration::from_secs(12 * 3600);
const SEASON_TTL_PAST: Duration = Duration::from_secs(30 * 24 * 3600);
/// How many 50-title pages of a season to pull (popularity-sorted).
const SEASON_MAX_PAGES: i32 = 3;
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
/// M7 — tray + background running.
pub const CLOSE_TO_TRAY_KEY: &str = "close_to_tray";
pub const START_ON_LOGIN_KEY: &str = "start_on_login";
pub const START_MINIMIZED_KEY: &str = "start_minimized";
/// M6a — playback detection. Confirm-by-default (a dismissible toast +
/// notification); "silent" auto-bumps with no prompt.
pub const PLAYBACK_ENABLED_KEY: &str = "playback_enabled";
pub const PLAYBACK_MODE_KEY: &str = "playback_mode";
/// How often the playback tracker is checked for episodes that have crossed
/// their "probably watched" threshold.
pub const PLAYBACK_POLL_EVERY: Duration = Duration::from_secs(15);
/// M6b — foreground-window detection. Off by default: reading the title of
/// whatever window is focused is a step up in what the app looks at, so it's
/// an explicit opt-in on top of the base playback-detection toggle.
pub const PLAYBACK_WINDOW_DETECT_KEY: &str = "playback_window_detect";
pub const PLAYBACK_MONITORED_PLAYERS_KEY: &str = "playback_monitored_players";
/// How often the foreground window is checked.
pub const WINDOW_DETECT_POLL_EVERY: Duration = Duration::from_secs(10);

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

/// Watch statistics, computed from the local cache.
pub async fn stats(state: &AppState, service: Option<&str>) -> AppResult<crate::stats::StatsData> {
    let entries = repo::all_entries(&state.db, parse_service(service)).await?;
    Ok(crate::stats::compute(&entries, chrono::Utc::now()))
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
    mut patch: EntryPatch,
) -> AppResult<MediaListEntry> {
    let svc = parse_service(service);
    let media = ensure_media(state, svc, patch.media_id).await?;

    // Reaching the final episode while Watching completes the show — the one
    // rule every progress-changing path (the +1 button, a playback bump, or a
    // manual edit that leaves status untouched) should agree on, so it lives
    // here instead of being recomputed by each caller.
    if let Some(progress) = patch.progress {
        if media.episodes == Some(progress) {
            let effective_status = match patch.status {
                Some(s) => s,
                None => repo::get_entry(&state.db, svc, patch.media_id)
                    .await?
                    .map(|e| e.status)
                    .unwrap_or(ListStatus::Planning),
            };
            if effective_status == ListStatus::Current {
                patch.status = Some(ListStatus::Completed);
            }
        }
    }

    let entry = repo::apply_local_patch(&state.db, svc, &media, &patch).await?;
    state.push.nudge();
    // Any progress change (manual or auto-bumped) retires tracked playback
    // sessions at or below the new progress, so they don't fire a stale prompt.
    if let Some(progress) = patch.progress {
        state
            .playback
            .stop_up_to(svc, patch.media_id, progress as i64);
    }
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
            // Push the local dates as authoritative (last-write-wins). `""` sent
            // for an unset date clears it on the service.
            started_at: Some(entry.started_at.clone().unwrap_or_default()),
            completed_at: Some(entry.completed_at.clone().unwrap_or_default()),
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
// Season browser (M4). One paced, cached `Page` query set per (year, season).
// --------------------------------------------------------------------------- //

/// (year, season) for right now, AniList's calendar convention.
pub fn current_season() -> (i32, MediaSeason) {
    let now = chrono::Utc::now();
    let year = now.format("%Y").to_string().parse().unwrap_or(2025);
    let month: u32 = now.format("%m").to_string().parse().unwrap_or(1);
    let season = match month {
        1..=3 => MediaSeason::Winter,
        4..=6 => MediaSeason::Spring,
        7..=9 => MediaSeason::Summer,
        _ => MediaSeason::Fall,
    };
    (year, season)
}

fn season_is_stale(year: i32, season: MediaSeason, fetched_at: &str) -> bool {
    let (cy, cs) = current_season();
    let ttl = if year > cy || (year == cy && season as u8 >= cs as u8) {
        SEASON_TTL_CURRENT
    } else {
        SEASON_TTL_PAST
    };
    chrono::DateTime::parse_from_rfc3339(fetched_at)
        .ok()
        .and_then(|t| (chrono::Utc::now() - t.with_timezone(&chrono::Utc)).to_std().ok())
        .map(|age| age >= ttl)
        .unwrap_or(true)
}

/// The season's anime, popularity-first. Cache-first with a TTL; a stale copy
/// beats an error when AniList is unreachable.
pub async fn season(state: &AppState, year: i32, season: &str) -> AppResult<Vec<Media>> {
    let season = MediaSeason::from_opt_str(Some(&season.to_ascii_uppercase()))
        .ok_or_else(|| AppError::other("Unknown season."))?;

    if let Some((media, fetched_at)) = repo::season_cached(&state.db, year, season).await? {
        if !season_is_stale(year, season, &fetched_at) {
            return Ok(media);
        }
        return match refetch_season(state, year, season).await {
            Ok(m) => Ok(m),
            Err(_) => Ok(media),
        };
    }
    refetch_season(state, year, season).await
}

async fn refetch_season(
    state: &AppState,
    year: i32,
    season: MediaSeason,
) -> AppResult<Vec<Media>> {
    let token = auth::load_token(ServiceKind::AniList)?;
    let mut all: Vec<Media> = Vec::new();
    for page in 1..=SEASON_MAX_PAGES {
        let result = state
            .anilist
            .season(token.as_deref(), year, season, page)
            .await?;
        let last = !result.has_next_page || result.media.is_empty();
        all.extend(result.media);
        if last {
            break;
        }
    }
    repo::replace_season(&state.db, year, season, &all).await?;
    Ok(all)
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

/// If playback detection is on and `episode` is genuinely the next unwatched
/// one, build the watch session for it. Never tracks a rewatch or a batch
/// jump-ahead — only ever `progress + 1`, matching what the Play button
/// itself is allowed to open.
pub async fn prepare_watch_session(
    state: &AppState,
    service: Option<&str>,
    media_id: i64,
    episode: i64,
) -> AppResult<Option<crate::playback::WatchSession>> {
    if !repo::get_bool_setting(&state.db, PLAYBACK_ENABLED_KEY, true).await? {
        return Ok(None);
    }
    let svc = parse_service(service);
    let Some(entry) = repo::get_entry(&state.db, svc, media_id).await? else {
        return Ok(None);
    };
    if i64::from(entry.progress) + 1 != episode {
        return Ok(None);
    }
    Ok(Some(crate::playback::WatchSession::new(
        svc,
        media_id,
        episode,
        entry.media.title.preferred(),
        entry.media.episodes.map(i64::from),
    )))
}

/// Apply a playback-detected bump: sets progress to `episode`. `edit_entry`
/// itself decides whether that also completes the show.
pub async fn bump_from_playback(
    state: &AppState,
    service: &str,
    media_id: i64,
    episode: i64,
) -> AppResult<MediaListEntry> {
    let svc = parse_service(Some(service));
    let entry = repo::get_entry(&state.db, svc, media_id)
        .await?
        .ok_or_else(|| AppError::other("that entry no longer exists"))?;
    let patch = EntryPatch {
        media_id,
        remote_id: entry.remote_id,
        progress: Some(episode as i32),
        ..Default::default()
    };
    edit_entry(state, Some(service), patch).await
}

/// M6b: check whatever window currently has focus. If it's a monitored player
/// showing what looks like an anime episode that's on the user's list, feed it
/// into the same tracker `prepare_watch_session` uses for self-launched
/// playback — same threshold, same confirm/silent handling.
pub async fn detect_foreground_playback(
    state: &AppState,
) -> AppResult<Option<crate::playback::WatchSession>> {
    if !repo::get_bool_setting(&state.db, PLAYBACK_WINDOW_DETECT_KEY, false).await? {
        return Ok(None);
    }
    let Some(win) = crate::playback::detect::foreground_window() else {
        return Ok(None);
    };
    let enabled = repo::enabled_players(&state.db).await?;
    if !enabled.iter().any(|p| p.eq_ignore_ascii_case(&win.process)) {
        return Ok(None);
    }

    let cleaned = crate::playback::detect::strip_player_chrome(&win.title, &win.process);
    let parsed = scanner::parse_name(&cleaned);
    let (Some(title), Some(episode)) = (parsed.title, parsed.episode) else {
        return Ok(None);
    };

    let index = repo::media_match_index(&state.db).await?;
    let Some(m) = matcher::best_match(
        &title,
        None,
        parsed.season,
        parsed.year.map(|y| y as i32),
        &index,
    ) else {
        return Ok(None);
    };

    prepare_watch_session(state, Some(m.service.as_str()), m.media_id, episode).await
}

/// Absolute path of the local file for one episode. Errors if it isn't on disk.
pub async fn episode_file_path(
    state: &AppState,
    service: Option<&str>,
    media_id: i64,
    episode: i64,
) -> AppResult<String> {
    repo::episode_file(&state.db, parse_service(service), media_id, episode)
        .await?
        .ok_or_else(|| AppError::other("That episode isn't in your local library."))
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

/// Manually bind one or more files to a media row. When `remember` is set, a
/// link rule is stored (keyed on the first file's folder/file title + season)
/// so future episodes of the same show/season link on their own.
///
/// Ensures the media is cached first — one cache-first lookup, no forced fetch
/// when it's already present.
pub async fn link_library_files(
    state: &AppState,
    file_ids: &[i64],
    service: Option<&str>,
    media_id: i64,
    remember: bool,
) -> AppResult<()> {
    let svc = parse_service(service);
    if file_ids.is_empty() {
        return Ok(());
    }
    let first = repo::get_library_file(&state.db, file_ids[0])
        .await?
        .ok_or_else(|| AppError::other("That library file no longer exists."))?;

    let _ = ensure_media(state, svc, media_id).await?;
    repo::set_files_match(&state.db, file_ids, svc, media_id, "manual").await?;

    if remember {
        if let Some(key) =
            rule_key(first.folder_title.as_deref().or(first.parsed_title.as_deref()))
        {
            repo::upsert_link_rule(&state.db, &key, first.parsed_season, svc, media_id).await?;
            // Sweep up any other pending files the new rule now covers.
            let _ = run_matcher(state).await?;
        }
    }
    Ok(())
}

pub async fn unlink_library_file(state: &AppState, file_id: i64) -> AppResult<()> {
    repo::clear_file_match(&state.db, file_id).await
}

pub async fn library_link_rules(state: &AppState) -> AppResult<Vec<LinkRule>> {
    repo::list_link_rules(&state.db).await
}

pub async fn delete_link_rule(state: &AppState, id: i64) -> AppResult<()> {
    repo::delete_link_rule(&state.db, id).await
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
        let paths = tokio::task::spawn_blocking({
            let root = root.clone();
            move || scanner::walk(&root)
        })
        .await
        .unwrap_or_default();

        let mut present: Vec<String> = Vec::with_capacity(paths.len());
        for p in paths {
            present.push(p.to_string_lossy().into_owned());
            let scanned = tokio::task::spawn_blocking({
                let p = p.clone();
                let root = root.clone();
                move || scanner::scan_file(&p, &root)
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

    let counts = run_matcher(state).await?;
    report.rule_matched = counts.rule;
    report.auto_matched = counts.auto;
    report.unmatched = counts.unmatched;
    report.finished_at = chrono::Utc::now().to_rfc3339();
    Ok(report)
}

#[derive(Default)]
struct MatchCounts {
    rule: usize,
    auto: usize,
    unmatched: usize,
}

/// Resolve still-unmatched files: first apply remembered link rules (keyed on
/// the folder/file title + season), then fall back to fuzzy matching against
/// the media cache. Manual links are never touched.
async fn run_matcher(state: &AppState) -> AppResult<MatchCounts> {
    let mut counts = MatchCounts::default();
    let pending = repo::files_needing_match(&state.db).await?;
    if pending.is_empty() {
        return Ok(counts);
    }

    // Pass 1 — remembered rules.
    let rules = repo::list_link_rules(&state.db).await?;
    if !rules.is_empty() {
        for f in &pending {
            let key = rule_key(f.folder_title.as_deref().or(f.parsed_title.as_deref()));
            let Some(key) = key else { continue };
            let hit = rules.iter().find(|r| {
                r.title_key == key && (r.season == f.season || r.season.is_none())
            });
            if let Some(r) = hit {
                let svc = r.service.parse().unwrap_or(ServiceKind::AniList);
                repo::set_file_match(&state.db, f.id, svc, r.media_id, "rule", None).await?;
                counts.rule += 1;
            }
        }
    }

    // Pass 2 — fuzzy match whatever's left.
    let pending = repo::files_needing_match(&state.db).await?;
    let index = repo::media_match_index(&state.db).await?;
    for f in &pending {
        let title = f
            .parsed_title
            .as_deref()
            .or(f.folder_title.as_deref())
            .unwrap_or("");
        if let Some(m) = matcher::best_match(
            title,
            f.folder_title.as_deref(),
            f.season,
            f.year.map(|y| y as i32),
            &index,
        ) {
            repo::set_file_match(&state.db, f.id, m.service, m.media_id, "auto", Some(m.score))
                .await?;
            counts.auto += 1;
        }
    }

    counts.unmatched = repo::files_needing_match(&state.db).await?.len();
    Ok(counts)
}

/// The key a link rule is stored under — normalised title, or `None` if there's
/// nothing usable to key on.
fn rule_key(title: Option<&str>) -> Option<String> {
    let k = matcher::normalize(title?);
    (!k.is_empty()).then_some(k)
}
