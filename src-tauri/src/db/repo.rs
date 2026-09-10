//! Typed helpers over the SQLite schema. Every query is plain runtime SQL so the
//! crate builds without a live database at compile time.

use chrono::Utc;
use serde_json::Value;
use sqlx::Row;

use crate::download::QbConfig;
use crate::error::AppResult;
use crate::library::matcher::IndexEntry;
use crate::library::scanner::ScannedFile;
use crate::library::{LibraryFile, LibraryFolder, LinkRule, OwnedMedia};
use crate::rss::{Feed, HistoryEntry, Rule, RuleInput};
use crate::tracker::model::*;

use super::Db;

fn now() -> String {
    Utc::now().to_rfc3339()
}

// --------------------------------------------------------------------------- //
// Settings
// --------------------------------------------------------------------------- //

pub async fn get_setting(db: &Db, key: &str) -> AppResult<Option<Value>> {
    let row = sqlx::query("SELECT value_json FROM app_setting WHERE key = ?1")
        .bind(key)
        .fetch_optional(db)
        .await?;
    Ok(match row {
        Some(r) => {
            let raw: String = r.get(0);
            serde_json::from_str(&raw).ok()
        }
        None => None,
    })
}

pub async fn set_setting(db: &Db, key: &str, value: &Value) -> AppResult<()> {
    sqlx::query(
        "INSERT INTO app_setting (key, value_json) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value_json = excluded.value_json",
    )
    .bind(key)
    .bind(serde_json::to_string(value)?)
    .execute(db)
    .await?;
    Ok(())
}

/// Read a boolean setting, falling back to `default` when unset or malformed.
pub async fn get_bool_setting(db: &Db, key: &str, default: bool) -> AppResult<bool> {
    Ok(get_setting(db, key)
        .await?
        .and_then(|v| v.as_bool())
        .unwrap_or(default))
}

pub async fn set_bool_setting(db: &Db, key: &str, value: bool) -> AppResult<()> {
    set_setting(db, key, &Value::Bool(value)).await
}

// --------------------------------------------------------------------------- //
// Accounts
// --------------------------------------------------------------------------- //

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Account {
    pub service: String,
    pub user_id: String,
    pub user_name: String,
    pub avatar_url: Option<String>,
    pub score_format: String,
    pub is_primary: bool,
    pub connected_at: String,
}

pub async fn upsert_account(
    db: &Db,
    service: ServiceKind,
    viewer: &Viewer,
    make_primary: bool,
) -> AppResult<()> {
    sqlx::query(
        "INSERT INTO account (service, user_id, user_name, avatar_url, score_format, is_primary, connected_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
         ON CONFLICT(service) DO UPDATE SET
            user_id = excluded.user_id,
            user_name = excluded.user_name,
            avatar_url = excluded.avatar_url,
            score_format = excluded.score_format",
    )
    .bind(service.as_str())
    .bind(viewer.id.to_string())
    .bind(&viewer.name)
    .bind(&viewer.avatar_url)
    .bind(&viewer.score_format)
    .bind(i64::from(make_primary))
    .bind(now())
    .execute(db)
    .await?;

    if make_primary {
        sqlx::query("UPDATE account SET is_primary = (service = ?1)")
            .bind(service.as_str())
            .execute(db)
            .await?;
    }
    Ok(())
}

pub async fn get_account(db: &Db, service: ServiceKind) -> AppResult<Option<Account>> {
    let row = sqlx::query(
        "SELECT service, user_id, user_name, avatar_url, score_format, is_primary, connected_at
         FROM account WHERE service = ?1",
    )
    .bind(service.as_str())
    .fetch_optional(db)
    .await?;
    Ok(row.map(|r| Account {
        service: r.get("service"),
        user_id: r.get("user_id"),
        user_name: r.get("user_name"),
        avatar_url: r.get("avatar_url"),
        score_format: r.get("score_format"),
        is_primary: r.get::<i64, _>("is_primary") != 0,
        connected_at: r.get("connected_at"),
    }))
}

pub async fn list_accounts(db: &Db) -> AppResult<Vec<Account>> {
    let rows = sqlx::query(
        "SELECT service, user_id, user_name, avatar_url, score_format, is_primary, connected_at
         FROM account ORDER BY connected_at",
    )
    .fetch_all(db)
    .await?;
    Ok(rows
        .into_iter()
        .map(|r| Account {
            service: r.get("service"),
            user_id: r.get("user_id"),
            user_name: r.get("user_name"),
            avatar_url: r.get("avatar_url"),
            score_format: r.get("score_format"),
            is_primary: r.get::<i64, _>("is_primary") != 0,
            connected_at: r.get("connected_at"),
        })
        .collect())
}

pub async fn delete_account(db: &Db, service: ServiceKind) -> AppResult<()> {
    sqlx::query("DELETE FROM account WHERE service = ?1")
        .bind(service.as_str())
        .execute(db)
        .await?;
    Ok(())
}

// --------------------------------------------------------------------------- //
// Media cache
// --------------------------------------------------------------------------- //

pub async fn upsert_media(db: &Db, m: &Media) -> AppResult<()> {
    let ts = now();
    let (next_ep, next_at) = match &m.next_airing {
        Some(a) => (Some(a.episode), Some(a.airing_at.clone())),
        None => (None, None),
    };
    sqlx::query(
        "INSERT INTO media_cache (
            service, id, title_romaji, title_english, title_native, format, airing_status,
            description, episodes, duration, season, season_year, cover_url, cover_color,
            banner_url, average_score, genres_json, synonyms_json, start_date, site_url,
            next_episode, next_airing_at, meta_fetched_at, airing_fetched_at, popularity)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,?20,?21,?22,?23,?24,?25)
         ON CONFLICT(service, id) DO UPDATE SET
            title_romaji = excluded.title_romaji,
            title_english = excluded.title_english,
            title_native = excluded.title_native,
            format = excluded.format,
            airing_status = excluded.airing_status,
            description = excluded.description,
            episodes = excluded.episodes,
            duration = excluded.duration,
            season = excluded.season,
            season_year = excluded.season_year,
            cover_url = excluded.cover_url,
            cover_color = excluded.cover_color,
            banner_url = excluded.banner_url,
            average_score = excluded.average_score,
            genres_json = excluded.genres_json,
            synonyms_json = excluded.synonyms_json,
            start_date = excluded.start_date,
            site_url = excluded.site_url,
            next_episode = excluded.next_episode,
            next_airing_at = excluded.next_airing_at,
            meta_fetched_at = excluded.meta_fetched_at,
            airing_fetched_at = excluded.airing_fetched_at,
            popularity = excluded.popularity",
    )
    .bind(m.id.service.as_str())
    .bind(m.id.id)
    .bind(&m.title.romaji)
    .bind(&m.title.english)
    .bind(&m.title.native)
    .bind(m.format.as_str())
    .bind(m.airing_status.as_str())
    .bind(&m.description)
    .bind(m.episodes)
    .bind(m.duration)
    .bind(m.season.map(|s| s.as_str()))
    .bind(m.season_year)
    .bind(&m.cover_url)
    .bind(&m.cover_color)
    .bind(&m.banner_url)
    .bind(m.average_score)
    .bind(serde_json::to_string(&m.genres)?)
    .bind(serde_json::to_string(&m.synonyms)?)
    .bind(&m.start_date)
    .bind(&m.site_url)
    .bind(next_ep)
    .bind(next_at)
    .bind(&ts)
    .bind(&ts)
    .bind(m.popularity)
    .execute(db)
    .await?;
    Ok(())
}

pub async fn upsert_media_bulk(db: &Db, media: &[Media]) -> AppResult<()> {
    let mut tx = db.begin().await?;
    for m in media {
        upsert_media_tx(&mut tx, m).await?;
    }
    tx.commit().await?;
    Ok(())
}

async fn upsert_media_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    m: &Media,
) -> AppResult<()> {
    let ts = now();
    let (next_ep, next_at) = match &m.next_airing {
        Some(a) => (Some(a.episode), Some(a.airing_at.clone())),
        None => (None, None),
    };
    sqlx::query(
        "INSERT INTO media_cache (
            service, id, title_romaji, title_english, title_native, format, airing_status,
            description, episodes, duration, season, season_year, cover_url, cover_color,
            banner_url, average_score, genres_json, synonyms_json, start_date, site_url,
            next_episode, next_airing_at, meta_fetched_at, airing_fetched_at, popularity)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,?20,?21,?22,?23,?24,?25)
         ON CONFLICT(service, id) DO UPDATE SET
            title_romaji = excluded.title_romaji,
            title_english = excluded.title_english,
            title_native = excluded.title_native,
            format = excluded.format,
            airing_status = excluded.airing_status,
            description = excluded.description,
            episodes = excluded.episodes,
            duration = excluded.duration,
            season = excluded.season,
            season_year = excluded.season_year,
            cover_url = excluded.cover_url,
            cover_color = excluded.cover_color,
            banner_url = excluded.banner_url,
            average_score = excluded.average_score,
            genres_json = excluded.genres_json,
            synonyms_json = excluded.synonyms_json,
            start_date = excluded.start_date,
            site_url = excluded.site_url,
            next_episode = excluded.next_episode,
            next_airing_at = excluded.next_airing_at,
            meta_fetched_at = excluded.meta_fetched_at,
            airing_fetched_at = excluded.airing_fetched_at,
            popularity = excluded.popularity",
    )
    .bind(m.id.service.as_str())
    .bind(m.id.id)
    .bind(&m.title.romaji)
    .bind(&m.title.english)
    .bind(&m.title.native)
    .bind(m.format.as_str())
    .bind(m.airing_status.as_str())
    .bind(&m.description)
    .bind(m.episodes)
    .bind(m.duration)
    .bind(m.season.map(|s| s.as_str()))
    .bind(m.season_year)
    .bind(&m.cover_url)
    .bind(&m.cover_color)
    .bind(&m.banner_url)
    .bind(m.average_score)
    .bind(serde_json::to_string(&m.genres)?)
    .bind(serde_json::to_string(&m.synonyms)?)
    .bind(&m.start_date)
    .bind(&m.site_url)
    .bind(next_ep)
    .bind(next_at)
    .bind(&ts)
    .bind(&ts)
    .bind(m.popularity)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

fn media_from_row(r: &sqlx::sqlite::SqliteRow) -> Media {
    let genres: Vec<String> = r
        .get::<Option<String>, _>("genres_json")
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();
    let synonyms: Vec<String> = r
        .get::<Option<String>, _>("synonyms_json")
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();
    let next_airing = match (
        r.get::<Option<i64>, _>("next_episode"),
        r.get::<Option<String>, _>("next_airing_at"),
    ) {
        (Some(ep), Some(at)) => Some(AiringInfo {
            episode: ep as i32,
            airing_at: at,
        }),
        _ => None,
    };
    let service: String = r.get("service");
    Media {
        id: MediaId {
            service: service.parse().unwrap_or(ServiceKind::AniList),
            id: r.get("id"),
        },
        title: MediaTitle {
            romaji: r.get("title_romaji"),
            english: r.get("title_english"),
            native: r.get("title_native"),
        },
        format: MediaFormat::from_opt_str(r.get::<Option<String>, _>("format").as_deref()),
        airing_status: AiringStatus::from_opt_str(
            r.get::<Option<String>, _>("airing_status").as_deref(),
        ),
        description: r.get("description"),
        episodes: r.get::<Option<i64>, _>("episodes").map(|x| x as i32),
        duration: r.get::<Option<i64>, _>("duration").map(|x| x as i32),
        season: MediaSeason::from_opt_str(r.get::<Option<String>, _>("season").as_deref()),
        season_year: r.get::<Option<i64>, _>("season_year").map(|x| x as i32),
        cover_url: r.get("cover_url"),
        cover_color: r.get("cover_color"),
        banner_url: r.get("banner_url"),
        average_score: r.get::<Option<i64>, _>("average_score").map(|x| x as i32),
        popularity: r.get::<Option<i64>, _>("popularity").map(|x| x as i32),
        genres,
        synonyms,
        start_date: r.get("start_date"),
        site_url: r.get("site_url"),
        next_airing,
    }
}

/// One media row from the cache, plus when its metadata was last fetched
/// (RFC3339). Works whether or not the media is on the user's list.
pub async fn cached_media(
    db: &Db,
    service: ServiceKind,
    id: i64,
) -> AppResult<Option<(Media, String)>> {
    let row = sqlx::query(
        "SELECT service, id, title_romaji, title_english, title_native, format,
            airing_status, description, episodes, duration, season, season_year,
            cover_url, cover_color, banner_url, average_score, popularity, genres_json,
            synonyms_json, start_date, site_url, next_episode, next_airing_at,
            meta_fetched_at
         FROM media_cache WHERE service = ?1 AND id = ?2",
    )
    .bind(service.as_str())
    .bind(id)
    .fetch_optional(db)
    .await?;
    Ok(row.map(|r| (media_from_row(&r), r.get::<String, _>("meta_fetched_at"))))
}

// --------------------------------------------------------------------------- //
// List entries
// --------------------------------------------------------------------------- //

const ENTRY_SELECT_BASE: &str = "SELECT
    e.remote_id, e.status, e.progress, e.score_raw, e.repeat, e.notes,
    e.started_at, e.completed_at, e.updated_at, e.dirty,
    m.service, m.id, m.title_romaji, m.title_english, m.title_native, m.format,
    m.airing_status, m.description, m.episodes, m.duration, m.season, m.season_year,
    m.cover_url, m.cover_color, m.banner_url, m.average_score, m.popularity, m.genres_json,
    m.synonyms_json, m.start_date, m.site_url, m.next_episode, m.next_airing_at
 FROM list_entry e
 JOIN media_cache m ON m.service = e.service AND m.id = e.media_id";

fn entry_from_row(r: &sqlx::sqlite::SqliteRow) -> MediaListEntry {
    let status: String = r.get("status");
    MediaListEntry {
        media: media_from_row(r),
        remote_id: r.get("remote_id"),
        status: status.parse().unwrap_or(ListStatus::Planning),
        progress: r.get::<i64, _>("progress") as i32,
        score_raw: r.get::<i64, _>("score_raw") as i32,
        repeat: r.get::<i64, _>("repeat") as i32,
        notes: r.get("notes"),
        started_at: r.get("started_at"),
        completed_at: r.get("completed_at"),
        updated_at: r.get("updated_at"),
        dirty: r.get::<i64, _>("dirty") != 0,
    }
}

pub async fn all_entries(db: &Db, service: ServiceKind) -> AppResult<Vec<MediaListEntry>> {
    let sql = format!(
        "{ENTRY_SELECT_BASE} WHERE e.deleted = 0 AND e.service = ?1
         ORDER BY m.title_romaji COLLATE NOCASE"
    );
    let rows = sqlx::query(&sql).bind(service.as_str()).fetch_all(db).await?;
    Ok(rows.iter().map(entry_from_row).collect())
}

pub async fn get_entry(
    db: &Db,
    service: ServiceKind,
    media_id: i64,
) -> AppResult<Option<MediaListEntry>> {
    let sql = format!(
        "{ENTRY_SELECT_BASE} WHERE e.deleted = 0 AND e.service = ?1 AND e.media_id = ?2"
    );
    let row = sqlx::query(&sql)
        .bind(service.as_str())
        .bind(media_id)
        .fetch_optional(db)
        .await?;
    Ok(row.as_ref().map(entry_from_row))
}

/// Replace the whole list for a service from a fresh server pull. Entries missing
/// from `entries` that were clean locally are removed; dirty local entries are kept.
pub async fn replace_list_from_remote(
    db: &Db,
    service: ServiceKind,
    entries: &[MediaListEntry],
) -> AppResult<()> {
    let mut tx = db.begin().await?;
    let ts = now();

    for e in entries {
        upsert_media_tx(&mut tx, &e.media).await?;
    }

    for e in entries {
        // Don't clobber a pending local edit with server state.
        let is_dirty: Option<i64> = sqlx::query_scalar(
            "SELECT dirty FROM list_entry WHERE service = ?1 AND media_id = ?2",
        )
        .bind(service.as_str())
        .bind(e.media.id.id)
        .fetch_optional(&mut *tx)
        .await?;
        if is_dirty == Some(1) {
            // still refresh the server-side bookkeeping columns
            sqlx::query(
                "UPDATE list_entry SET remote_id = ?3, remote_updated_at = ?4
                 WHERE service = ?1 AND media_id = ?2",
            )
            .bind(service.as_str())
            .bind(e.media.id.id)
            .bind(e.remote_id)
            .bind(&e.updated_at)
            .execute(&mut *tx)
            .await?;
            continue;
        }

        sqlx::query(
            "INSERT INTO list_entry (
                service, media_id, remote_id, status, progress, score_raw, repeat, notes,
                started_at, completed_at, updated_at, remote_updated_at, dirty, deleted)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,0,0)
             ON CONFLICT(service, media_id) DO UPDATE SET
                remote_id = excluded.remote_id,
                status = excluded.status,
                progress = excluded.progress,
                score_raw = excluded.score_raw,
                repeat = excluded.repeat,
                notes = excluded.notes,
                started_at = excluded.started_at,
                completed_at = excluded.completed_at,
                updated_at = excluded.updated_at,
                remote_updated_at = excluded.remote_updated_at,
                dirty = 0,
                deleted = 0",
        )
        .bind(service.as_str())
        .bind(e.media.id.id)
        .bind(e.remote_id)
        .bind(e.status.as_str())
        .bind(e.progress)
        .bind(e.score_raw)
        .bind(e.repeat)
        .bind(&e.notes)
        .bind(&e.started_at)
        .bind(&e.completed_at)
        .bind(e.updated_at.clone().unwrap_or_else(|| ts.clone()))
        .bind(&e.updated_at)
        .execute(&mut *tx)
        .await?;
    }

    // Prune clean local entries the server no longer has.
    let keep: Vec<i64> = entries.iter().map(|e| e.media.id.id).collect();
    let placeholders = if keep.is_empty() {
        "SELECT -1".to_string()
    } else {
        format!(
            "SELECT value FROM json_each('{}')",
            serde_json::to_string(&keep)?
        )
    };
    let prune_sql = format!(
        "DELETE FROM list_entry WHERE service = ?1 AND dirty = 0 AND media_id NOT IN ({placeholders})"
    );
    sqlx::query(&prune_sql)
        .bind(service.as_str())
        .execute(&mut *tx)
        .await?;

    sqlx::query(
        "INSERT INTO sync_state (service, last_full_sync) VALUES (?1, ?2)
         ON CONFLICT(service) DO UPDATE SET last_full_sync = excluded.last_full_sync",
    )
    .bind(service.as_str())
    .bind(&ts)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok(())
}

/// Fill in a "started watching" date on the transition into `Current` (or the
/// first episode of an already-current entry) — but never overwrite one that's
/// already set, so an explicit clear survives a re-save.
fn auto_started_date(
    resolved: Option<String>,
    base_status: ListStatus,
    status: ListStatus,
    base_progress: i32,
    progress: i32,
    today: &str,
) -> Option<String> {
    if resolved.is_some() {
        return resolved;
    }
    let began = status == ListStatus::Current
        && (base_status != ListStatus::Current || (base_progress == 0 && progress > 0));
    began.then(|| today.to_string())
}

/// Fill in a "finished" date on the transition into `Completed`, same rules.
fn auto_completed_date(
    resolved: Option<String>,
    base_status: ListStatus,
    status: ListStatus,
    today: &str,
) -> Option<String> {
    if resolved.is_some() {
        return resolved;
    }
    (status == ListStatus::Completed && base_status != ListStatus::Completed)
        .then(|| today.to_string())
}

/// Apply a local edit optimistically and mark the row dirty for the push worker.
pub async fn apply_local_patch(
    db: &Db,
    service: ServiceKind,
    media: &Media,
    patch: &EntryPatch,
) -> AppResult<MediaListEntry> {
    let ts = now();
    upsert_media(db, media).await?;

    let existing = get_entry(db, service, patch.media_id).await?;
    let base = existing.unwrap_or(MediaListEntry {
        media: media.clone(),
        remote_id: patch.remote_id,
        status: ListStatus::Planning,
        progress: 0,
        score_raw: 0,
        repeat: 0,
        notes: None,
        started_at: None,
        completed_at: None,
        updated_at: Some(ts.clone()),
        dirty: true,
    });

    let status = patch.status.unwrap_or(base.status);
    let progress = patch.progress.unwrap_or(base.progress).max(0);
    let score_raw = patch.score_raw.unwrap_or(base.score_raw).clamp(0, 100);
    let repeat = patch.repeat.unwrap_or(base.repeat).max(0);
    let notes = patch.notes.clone().or(base.notes.clone());

    // `None` = leave as-is; `Some("")` = clear; `Some(date)` = set.
    let resolve_date = |p: &Option<String>, b: &Option<String>| match p {
        None => b.clone(),
        Some(s) if s.trim().is_empty() => None,
        Some(s) => Some(s.trim().to_string()),
    };
    let today = Utc::now().format("%Y-%m-%d").to_string();
    let started_at = auto_started_date(
        resolve_date(&patch.started_at, &base.started_at),
        base.status,
        status,
        base.progress,
        progress,
        &today,
    );
    let completed_at = auto_completed_date(
        resolve_date(&patch.completed_at, &base.completed_at),
        base.status,
        status,
        &today,
    );

    sqlx::query(
        "INSERT INTO list_entry (
            service, media_id, remote_id, status, progress, score_raw, repeat, notes,
            started_at, completed_at, updated_at, dirty, deleted)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,1,0)
         ON CONFLICT(service, media_id) DO UPDATE SET
            remote_id = COALESCE(excluded.remote_id, list_entry.remote_id),
            status = excluded.status,
            progress = excluded.progress,
            score_raw = excluded.score_raw,
            repeat = excluded.repeat,
            notes = excluded.notes,
            started_at = excluded.started_at,
            completed_at = excluded.completed_at,
            updated_at = excluded.updated_at,
            dirty = 1,
            deleted = 0",
    )
    .bind(service.as_str())
    .bind(patch.media_id)
    .bind(patch.remote_id.or(base.remote_id))
    .bind(status.as_str())
    .bind(progress)
    .bind(score_raw)
    .bind(repeat)
    .bind(&notes)
    .bind(&started_at)
    .bind(&completed_at)
    .bind(&ts)
    .execute(db)
    .await?;

    get_entry(db, service, patch.media_id)
        .await?
        .ok_or_else(|| crate::error::AppError::other("entry vanished after patch"))
}

pub async fn mark_entry_deleted(db: &Db, service: ServiceKind, media_id: i64) -> AppResult<()> {
    sqlx::query(
        "UPDATE list_entry SET deleted = 1, dirty = 1, updated_at = ?3
         WHERE service = ?1 AND media_id = ?2",
    )
    .bind(service.as_str())
    .bind(media_id)
    .bind(now())
    .execute(db)
    .await?;
    Ok(())
}

pub async fn dirty_entries(db: &Db, service: ServiceKind) -> AppResult<Vec<MediaListEntry>> {
    let sql = format!(
        "{ENTRY_SELECT_BASE} WHERE e.service = ?1 AND e.dirty = 1 AND e.deleted = 0"
    );
    let rows = sqlx::query(&sql).bind(service.as_str()).fetch_all(db).await?;
    Ok(rows.iter().map(entry_from_row).collect())
}

/// How many local edits (including pending deletions) are waiting to be pushed.
pub async fn dirty_count(db: &Db, service: ServiceKind) -> AppResult<i64> {
    let n: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM list_entry WHERE service = ?1 AND dirty = 1")
            .bind(service.as_str())
            .fetch_one(db)
            .await?;
    Ok(n)
}

pub async fn dirty_deleted(db: &Db, service: ServiceKind) -> AppResult<Vec<(i64, Option<i64>)>> {
    let rows = sqlx::query(
        "SELECT media_id, remote_id FROM list_entry
         WHERE service = ?1 AND dirty = 1 AND deleted = 1",
    )
    .bind(service.as_str())
    .fetch_all(db)
    .await?;
    Ok(rows
        .iter()
        .map(|r| (r.get::<i64, _>("media_id"), r.get::<Option<i64>, _>("remote_id")))
        .collect())
}

pub async fn mark_entry_clean(
    db: &Db,
    service: ServiceKind,
    saved: &MediaListEntry,
) -> AppResult<()> {
    sqlx::query(
        "UPDATE list_entry SET
            remote_id = COALESCE(?3, remote_id),
            status = ?4, progress = ?5, score_raw = ?6, repeat = ?7, notes = ?8,
            started_at = ?10, completed_at = ?11,
            remote_updated_at = ?9, updated_at = ?9, dirty = 0
         WHERE service = ?1 AND media_id = ?2",
    )
    .bind(service.as_str())
    .bind(saved.media.id.id)
    .bind(saved.remote_id)
    .bind(saved.status.as_str())
    .bind(saved.progress)
    .bind(saved.score_raw)
    .bind(saved.repeat)
    .bind(&saved.notes)
    .bind(saved.updated_at.clone().unwrap_or_else(now))
    .bind(&saved.started_at)
    .bind(&saved.completed_at)
    .execute(db)
    .await?;
    Ok(())
}

pub async fn remove_entry_row(db: &Db, service: ServiceKind, media_id: i64) -> AppResult<()> {
    sqlx::query("DELETE FROM list_entry WHERE service = ?1 AND media_id = ?2")
        .bind(service.as_str())
        .bind(media_id)
        .execute(db)
        .await?;
    Ok(())
}

pub async fn last_full_sync(db: &Db, service: ServiceKind) -> AppResult<Option<String>> {
    let row = sqlx::query("SELECT last_full_sync FROM sync_state WHERE service = ?1")
        .bind(service.as_str())
        .fetch_optional(db)
        .await?;
    Ok(row.and_then(|r| r.get::<Option<String>, _>("last_full_sync")))
}

// --------------------------------------------------------------------------- //
// Local library — watched folders
// --------------------------------------------------------------------------- //

fn folder_from_row(r: &sqlx::sqlite::SqliteRow) -> LibraryFolder {
    LibraryFolder {
        id: r.get("id"),
        path: r.get("path"),
        enabled: r.get::<i64, _>("enabled") != 0,
        added_at: r.get("added_at"),
        scanned_at: r.get("scanned_at"),
        file_count: r.get("file_count"),
    }
}

const FOLDER_SELECT: &str = "SELECT f.id, f.path, f.enabled, f.added_at, f.scanned_at,
    (SELECT COUNT(*) FROM library_file lf WHERE lf.folder_id = f.id) AS file_count
 FROM library_folder f";

pub async fn list_library_folders(db: &Db) -> AppResult<Vec<LibraryFolder>> {
    let sql = format!("{FOLDER_SELECT} ORDER BY f.added_at");
    let rows = sqlx::query(&sql).fetch_all(db).await?;
    Ok(rows.iter().map(folder_from_row).collect())
}

pub async fn get_library_folder(db: &Db, id: i64) -> AppResult<Option<LibraryFolder>> {
    let sql = format!("{FOLDER_SELECT} WHERE f.id = ?1");
    let row = sqlx::query(&sql).bind(id).fetch_optional(db).await?;
    Ok(row.as_ref().map(folder_from_row))
}

/// Insert a folder (or return the existing row for that path).
pub async fn add_library_folder(db: &Db, path: &str) -> AppResult<LibraryFolder> {
    sqlx::query(
        "INSERT INTO library_folder (path, enabled, added_at) VALUES (?1, 1, ?2)
         ON CONFLICT(path) DO NOTHING",
    )
    .bind(path)
    .bind(now())
    .execute(db)
    .await?;
    let sql = format!("{FOLDER_SELECT} WHERE f.path = ?1");
    let row = sqlx::query(&sql).bind(path).fetch_one(db).await?;
    Ok(folder_from_row(&row))
}

pub async fn remove_library_folder(db: &Db, id: i64) -> AppResult<()> {
    sqlx::query("DELETE FROM library_folder WHERE id = ?1")
        .bind(id)
        .execute(db)
        .await?;
    Ok(())
}

pub async fn set_library_folder_enabled(db: &Db, id: i64, enabled: bool) -> AppResult<()> {
    sqlx::query("UPDATE library_folder SET enabled = ?2 WHERE id = ?1")
        .bind(id)
        .bind(i64::from(enabled))
        .execute(db)
        .await?;
    Ok(())
}

pub async fn mark_library_folder_scanned(db: &Db, id: i64) -> AppResult<()> {
    sqlx::query("UPDATE library_folder SET scanned_at = ?2 WHERE id = ?1")
        .bind(id)
        .bind(now())
        .execute(db)
        .await?;
    Ok(())
}

/// `(id, path)` for every enabled folder.
pub async fn enabled_library_folders(db: &Db) -> AppResult<Vec<(i64, String)>> {
    let rows = sqlx::query("SELECT id, path FROM library_folder WHERE enabled = 1 ORDER BY added_at")
        .fetch_all(db)
        .await?;
    Ok(rows
        .iter()
        .map(|r| (r.get::<i64, _>("id"), r.get::<String, _>("path")))
        .collect())
}

// --------------------------------------------------------------------------- //
// Local library — files
// --------------------------------------------------------------------------- //

const FILE_SELECT: &str = "SELECT lf.id, lf.folder_id, lf.path, lf.file_name, lf.size_bytes,
    lf.modified_at, lf.parsed_title, lf.folder_title, lf.parsed_episode, lf.parsed_season,
    lf.parsed_year, lf.resolution, lf.release_group, lf.service, lf.media_id, lf.match_kind,
    lf.match_score, lf.scanned_at,
    m.title_romaji, m.title_english, m.title_native
 FROM library_file lf
 LEFT JOIN media_cache m ON m.service = lf.service AND m.id = lf.media_id";

fn file_from_row(r: &sqlx::sqlite::SqliteRow) -> LibraryFile {
    let media_title = match (
        r.get::<Option<String>, _>("title_romaji"),
        r.get::<Option<String>, _>("title_english"),
        r.get::<Option<String>, _>("title_native"),
    ) {
        (None, None, None) => None,
        (romaji, english, native) => Some(MediaTitle {
            romaji,
            english,
            native,
        }),
    };
    LibraryFile {
        id: r.get("id"),
        folder_id: r.get("folder_id"),
        path: r.get("path"),
        file_name: r.get("file_name"),
        size_bytes: r.get("size_bytes"),
        modified_at: r.get("modified_at"),
        parsed_title: r.get("parsed_title"),
        folder_title: r.get("folder_title"),
        parsed_episode: r.get("parsed_episode"),
        parsed_season: r.get("parsed_season"),
        resolution: r.get("resolution"),
        release_group: r.get("release_group"),
        service: r.get("service"),
        media_id: r.get("media_id"),
        match_kind: r.get("match_kind"),
        match_score: r.get("match_score"),
        scanned_at: r.get("scanned_at"),
        media_title,
    }
}

pub async fn library_files(db: &Db) -> AppResult<Vec<LibraryFile>> {
    let sql = format!(
        "{FILE_SELECT} ORDER BY COALESCE(m.title_romaji, lf.parsed_title, lf.file_name) \
         COLLATE NOCASE, lf.parsed_episode"
    );
    let rows = sqlx::query(&sql).fetch_all(db).await?;
    Ok(rows.iter().map(file_from_row).collect())
}

pub async fn get_library_file(db: &Db, id: i64) -> AppResult<Option<LibraryFile>> {
    let sql = format!("{FILE_SELECT} WHERE lf.id = ?1");
    let row = sqlx::query(&sql).bind(id).fetch_optional(db).await?;
    Ok(row.as_ref().map(file_from_row))
}

/// Insert or refresh a scanned file. A manual match (`match_kind = 'manual'`) is
/// never disturbed; other columns are always refreshed from the parse.
pub async fn upsert_library_file(
    db: &Db,
    folder_id: i64,
    f: &ScannedFile,
) -> AppResult<()> {
    let path = f.path.to_string_lossy();
    sqlx::query(
        "INSERT INTO library_file (
            folder_id, path, file_name, size_bytes, modified_at,
            parsed_title, folder_title, parsed_episode, parsed_season, parsed_year,
            resolution, release_group, scanned_at)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13)
         ON CONFLICT(path) DO UPDATE SET
            folder_id = excluded.folder_id,
            file_name = excluded.file_name,
            size_bytes = excluded.size_bytes,
            modified_at = excluded.modified_at,
            parsed_title = excluded.parsed_title,
            folder_title = excluded.folder_title,
            parsed_episode = excluded.parsed_episode,
            parsed_season = excluded.parsed_season,
            parsed_year = excluded.parsed_year,
            resolution = excluded.resolution,
            release_group = excluded.release_group,
            scanned_at = excluded.scanned_at",
    )
    .bind(folder_id)
    .bind(path.as_ref())
    .bind(&f.file_name)
    .bind(f.size_bytes)
    .bind(&f.modified_at)
    .bind(&f.title)
    .bind(&f.folder_title)
    .bind(f.episode)
    .bind(f.season)
    .bind(f.year)
    .bind(&f.resolution)
    .bind(&f.release_group)
    .bind(now())
    .execute(db)
    .await?;
    Ok(())
}

/// Drop rows for `folder_id` whose path is no longer on disk. Returns how many.
pub async fn prune_missing_files(
    db: &Db,
    folder_id: i64,
    present: &[String],
) -> AppResult<u64> {
    let keep = serde_json::to_string(present)?;
    let res = sqlx::query(
        "DELETE FROM library_file
         WHERE folder_id = ?1 AND path NOT IN (SELECT value FROM json_each(?2))",
    )
    .bind(folder_id)
    .bind(keep)
    .execute(db)
    .await?;
    Ok(res.rows_affected())
}

/// A file that still needs matching.
pub struct PendingFile {
    pub id: i64,
    pub parsed_title: Option<String>,
    pub folder_title: Option<String>,
    pub season: Option<i64>,
    pub year: Option<i64>,
}

/// Files awaiting a match — never those the user linked by hand (`manual`).
pub async fn files_needing_match(db: &Db) -> AppResult<Vec<PendingFile>> {
    let rows = sqlx::query(
        "SELECT id, parsed_title, folder_title, parsed_season, parsed_year FROM library_file
         WHERE media_id IS NULL AND match_kind IS NOT 'manual'",
    )
    .fetch_all(db)
    .await?;
    Ok(rows
        .iter()
        .map(|r| PendingFile {
            id: r.get("id"),
            parsed_title: r.get("parsed_title"),
            folder_title: r.get("folder_title"),
            season: r.get("parsed_season"),
            year: r.get("parsed_year"),
        })
        .collect())
}

pub async fn set_file_match(
    db: &Db,
    file_id: i64,
    service: ServiceKind,
    media_id: i64,
    kind: &str,
    score: Option<f64>,
) -> AppResult<()> {
    sqlx::query(
        "UPDATE library_file
         SET service = ?2, media_id = ?3, match_kind = ?4, match_score = ?5
         WHERE id = ?1",
    )
    .bind(file_id)
    .bind(service.as_str())
    .bind(media_id)
    .bind(kind)
    .bind(score)
    .execute(db)
    .await?;
    Ok(())
}

pub async fn clear_file_match(db: &Db, file_id: i64) -> AppResult<()> {
    sqlx::query(
        "UPDATE library_file
         SET service = NULL, media_id = NULL, match_kind = NULL, match_score = NULL
         WHERE id = ?1",
    )
    .bind(file_id)
    .execute(db)
    .await?;
    Ok(())
}

/// Point a set of files at one media in a single statement.
pub async fn set_files_match(
    db: &Db,
    file_ids: &[i64],
    service: ServiceKind,
    media_id: i64,
    kind: &str,
) -> AppResult<()> {
    if file_ids.is_empty() {
        return Ok(());
    }
    let ids = serde_json::to_string(file_ids)?;
    sqlx::query(
        "UPDATE library_file
         SET service = ?2, media_id = ?3, match_kind = ?4, match_score = NULL
         WHERE id IN (SELECT value FROM json_each(?1))",
    )
    .bind(ids)
    .bind(service.as_str())
    .bind(media_id)
    .bind(kind)
    .execute(db)
    .await?;
    Ok(())
}

// --------------------------------------------------------------------------- //
// Local library — remembered link rules
// --------------------------------------------------------------------------- //

pub async fn list_link_rules(db: &Db) -> AppResult<Vec<LinkRule>> {
    let rows = sqlx::query(
        "SELECT id, title_key, season, service, media_id, created_at
         FROM library_link_rule ORDER BY title_key, season",
    )
    .fetch_all(db)
    .await?;
    Ok(rows
        .iter()
        .map(|r| LinkRule {
            id: r.get("id"),
            title_key: r.get("title_key"),
            season: r.get("season"),
            service: r.get("service"),
            media_id: r.get("media_id"),
            created_at: r.get("created_at"),
        })
        .collect())
}

pub async fn upsert_link_rule(
    db: &Db,
    title_key: &str,
    season: Option<i64>,
    service: ServiceKind,
    media_id: i64,
) -> AppResult<()> {
    sqlx::query(
        "INSERT INTO library_link_rule (title_key, season, service, media_id, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5)
         ON CONFLICT(title_key, season) DO UPDATE SET
            service = excluded.service, media_id = excluded.media_id",
    )
    .bind(title_key)
    .bind(season)
    .bind(service.as_str())
    .bind(media_id)
    .bind(now())
    .execute(db)
    .await?;
    Ok(())
}

pub async fn delete_link_rule(db: &Db, id: i64) -> AppResult<()> {
    sqlx::query("DELETE FROM library_link_rule WHERE id = ?1")
        .bind(id)
        .execute(db)
        .await?;
    Ok(())
}

/// Build the matcher index from everything in the media cache.
pub async fn media_match_index(db: &Db) -> AppResult<Vec<IndexEntry>> {
    let rows = sqlx::query(
        "SELECT service, id, title_romaji, title_english, title_native, synonyms_json,
            season_year, format, popularity
         FROM media_cache",
    )
    .fetch_all(db)
    .await?;
    Ok(rows
        .iter()
        .map(|r| {
            let service: String = r.get("service");
            let primary: Vec<String> = ["title_romaji", "title_english", "title_native"]
                .iter()
                .filter_map(|c| r.get::<Option<String>, _>(*c))
                .collect();
            let synonyms: Vec<String> = r
                .get::<Option<String>, _>("synonyms_json")
                .and_then(|s| serde_json::from_str::<Vec<String>>(&s).ok())
                .unwrap_or_default();
            IndexEntry::new(crate::library::matcher::IndexInput {
                service: service.parse().unwrap_or(ServiceKind::AniList),
                media_id: r.get("id"),
                season_year: r.get::<Option<i64>, _>("season_year").map(|y| y as i32),
                popularity: r.get::<Option<i64>, _>("popularity"),
                format: r.get::<Option<String>, _>("format"),
                primary,
                synonyms,
            })
        })
        .collect())
}

/// The on-disk file for one episode of a matched show, if any. Prefers the
/// largest file when a folder has duplicates (e.g. a 1080p and a 720p rip).
pub async fn episode_file(
    db: &Db,
    service: ServiceKind,
    media_id: i64,
    episode: i64,
) -> AppResult<Option<String>> {
    let path: Option<String> = sqlx::query_scalar(
        "SELECT path FROM library_file
         WHERE service = ?1 AND media_id = ?2 AND parsed_episode = ?3
         ORDER BY size_bytes DESC
         LIMIT 1",
    )
    .bind(service.as_str())
    .bind(media_id)
    .bind(episode)
    .fetch_optional(db)
    .await?;
    Ok(path)
}

/// Episodes present on disk per matched media.
pub async fn owned_media(db: &Db) -> AppResult<Vec<OwnedMedia>> {
    let rows = sqlx::query(
        "SELECT media_id, parsed_episode FROM library_file
         WHERE media_id IS NOT NULL AND parsed_episode IS NOT NULL
         ORDER BY media_id, parsed_episode",
    )
    .fetch_all(db)
    .await?;

    let mut out: Vec<OwnedMedia> = Vec::new();
    for r in &rows {
        let media_id: i64 = r.get("media_id");
        let ep: i64 = r.get("parsed_episode");
        match out.last_mut() {
            Some(last) if last.media_id == media_id => {
                if !last.episodes.contains(&ep) {
                    last.episodes.push(ep);
                }
            }
            _ => out.push(OwnedMedia {
                media_id,
                episodes: vec![ep],
            }),
        }
    }
    Ok(out)
}

// --------------------------------------------------------------------------- //
// Season browser (M4)
// --------------------------------------------------------------------------- //

/// Cached season listing (media ordered by popularity) plus when it was fetched.
pub async fn season_cached(
    db: &Db,
    year: i32,
    season: MediaSeason,
) -> AppResult<Option<(Vec<Media>, String)>> {
    let fetched_at: Option<String> = sqlx::query_scalar(
        "SELECT fetched_at FROM season_state WHERE year = ?1 AND season = ?2",
    )
    .bind(year)
    .bind(season.as_str())
    .fetch_optional(db)
    .await?;
    let Some(fetched_at) = fetched_at else {
        return Ok(None);
    };

    let rows = sqlx::query(
        "SELECT m.service, m.id, m.title_romaji, m.title_english, m.title_native, m.format,
            m.airing_status, m.description, m.episodes, m.duration, m.season, m.season_year,
            m.cover_url, m.cover_color, m.banner_url, m.average_score, m.popularity,
            m.genres_json, m.synonyms_json, m.start_date, m.site_url, m.next_episode, m.next_airing_at
         FROM season_media s
         JOIN media_cache m ON m.service = 'anilist' AND m.id = s.media_id
         WHERE s.year = ?1 AND s.season = ?2
         ORDER BY s.sort_order",
    )
    .bind(year)
    .bind(season.as_str())
    .fetch_all(db)
    .await?;
    Ok(Some((rows.iter().map(media_from_row).collect(), fetched_at)))
}

/// Replace a season's membership from a fresh pull. Media details go into
/// `media_cache`; this records order and the fetch time.
pub async fn replace_season(
    db: &Db,
    year: i32,
    season: MediaSeason,
    media: &[Media],
) -> AppResult<()> {
    let mut tx = db.begin().await?;
    for m in media {
        upsert_media_tx(&mut tx, m).await?;
    }
    sqlx::query("DELETE FROM season_media WHERE year = ?1 AND season = ?2")
        .bind(year)
        .bind(season.as_str())
        .execute(&mut *tx)
        .await?;
    for (i, m) in media.iter().enumerate() {
        sqlx::query(
            "INSERT INTO season_media (year, season, media_id, sort_order) VALUES (?1,?2,?3,?4)",
        )
        .bind(year)
        .bind(season.as_str())
        .bind(m.id.id)
        .bind(i as i64)
        .execute(&mut *tx)
        .await?;
    }
    sqlx::query(
        "INSERT INTO season_state (year, season, fetched_at) VALUES (?1,?2,?3)
         ON CONFLICT(year, season) DO UPDATE SET fetched_at = excluded.fetched_at",
    )
    .bind(year)
    .bind(season.as_str())
    .bind(now())
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(())
}

// --------------------------------------------------------------------------- //
// RSS auto-download (M5) — feeds
// --------------------------------------------------------------------------- //

fn feed_from_row(r: &sqlx::sqlite::SqliteRow) -> Feed {
    Feed {
        id: r.get("id"),
        name: r.get("name"),
        url: r.get("url"),
        enabled: r.get::<i64, _>("enabled") != 0,
        added_at: r.get("added_at"),
        last_fetched_at: r.get("last_fetched_at"),
        last_error: r.get("last_error"),
    }
}

const FEED_SELECT: &str =
    "SELECT id, name, url, enabled, added_at, last_fetched_at, last_error FROM rss_feed";

pub async fn list_feeds(db: &Db) -> AppResult<Vec<Feed>> {
    let rows = sqlx::query(&format!("{FEED_SELECT} ORDER BY added_at"))
        .fetch_all(db)
        .await?;
    Ok(rows.iter().map(feed_from_row).collect())
}

pub async fn enabled_feeds(db: &Db) -> AppResult<Vec<Feed>> {
    let rows = sqlx::query(&format!("{FEED_SELECT} WHERE enabled = 1 ORDER BY added_at"))
        .fetch_all(db)
        .await?;
    Ok(rows.iter().map(feed_from_row).collect())
}

pub async fn add_feed(db: &Db, name: &str, url: &str) -> AppResult<Feed> {
    sqlx::query(
        "INSERT INTO rss_feed (name, url, enabled, added_at) VALUES (?1, ?2, 1, ?3)
         ON CONFLICT(url) DO UPDATE SET name = excluded.name",
    )
    .bind(name)
    .bind(url)
    .bind(now())
    .execute(db)
    .await?;
    let row = sqlx::query(&format!("{FEED_SELECT} WHERE url = ?1"))
        .bind(url)
        .fetch_one(db)
        .await?;
    Ok(feed_from_row(&row))
}

pub async fn remove_feed(db: &Db, id: i64) -> AppResult<()> {
    sqlx::query("DELETE FROM rss_feed WHERE id = ?1")
        .bind(id)
        .execute(db)
        .await?;
    Ok(())
}

pub async fn set_feed_enabled(db: &Db, id: i64, enabled: bool) -> AppResult<()> {
    sqlx::query("UPDATE rss_feed SET enabled = ?2 WHERE id = ?1")
        .bind(id)
        .bind(i64::from(enabled))
        .execute(db)
        .await?;
    Ok(())
}

/// Stamp a fetch attempt: `error = None` clears the last error, `Some` records it.
pub async fn mark_feed_fetched(db: &Db, id: i64, error: Option<&str>) -> AppResult<()> {
    sqlx::query("UPDATE rss_feed SET last_fetched_at = ?2, last_error = ?3 WHERE id = ?1")
        .bind(id)
        .bind(now())
        .bind(error)
        .execute(db)
        .await?;
    Ok(())
}

// --------------------------------------------------------------------------- //
// RSS auto-download — rules
// --------------------------------------------------------------------------- //

const RULE_SELECT: &str = "SELECT r.id, r.name, r.enabled, r.feed_id, r.service, r.media_id,
    r.title_contains, r.release_group, r.min_resolution, r.episode_from, r.episode_to,
    r.dest_path, r.category, r.paused, r.created_at,
    m.title_romaji, m.title_english, m.title_native
 FROM rss_rule r
 LEFT JOIN media_cache m ON m.service = r.service AND m.id = r.media_id";

fn rule_from_row(r: &sqlx::sqlite::SqliteRow) -> Rule {
    let media_title = match (
        r.get::<Option<String>, _>("title_romaji"),
        r.get::<Option<String>, _>("title_english"),
        r.get::<Option<String>, _>("title_native"),
    ) {
        (None, None, None) => None,
        (romaji, english, native) => Some(MediaTitle {
            romaji,
            english,
            native,
        }),
    };
    Rule {
        id: r.get("id"),
        name: r.get("name"),
        enabled: r.get::<i64, _>("enabled") != 0,
        feed_id: r.get("feed_id"),
        service: r.get("service"),
        media_id: r.get("media_id"),
        title_contains: r.get("title_contains"),
        release_group: r.get("release_group"),
        min_resolution: r.get("min_resolution"),
        episode_from: r.get("episode_from"),
        episode_to: r.get("episode_to"),
        dest_path: r.get("dest_path"),
        category: r.get("category"),
        paused: r.get::<i64, _>("paused") != 0,
        created_at: r.get("created_at"),
        media_title,
    }
}

fn clean_opt(s: &Option<String>) -> Option<String> {
    s.as_ref()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}

pub async fn list_rules(db: &Db) -> AppResult<Vec<Rule>> {
    let rows = sqlx::query(&format!("{RULE_SELECT} ORDER BY r.created_at"))
        .fetch_all(db)
        .await?;
    Ok(rows.iter().map(rule_from_row).collect())
}

pub async fn get_rule(db: &Db, id: i64) -> AppResult<Option<Rule>> {
    let row = sqlx::query(&format!("{RULE_SELECT} WHERE r.id = ?1"))
        .bind(id)
        .fetch_optional(db)
        .await?;
    Ok(row.as_ref().map(rule_from_row))
}

/// Rules that apply to `feed_id` — those bound to it plus the unbound (all-feed) ones.
pub async fn rules_for_feed(db: &Db, feed_id: i64) -> AppResult<Vec<Rule>> {
    let rows = sqlx::query(&format!(
        "{RULE_SELECT} WHERE r.feed_id IS NULL OR r.feed_id = ?1"
    ))
    .bind(feed_id)
    .fetch_all(db)
    .await?;
    Ok(rows.iter().map(rule_from_row).collect())
}

pub async fn insert_rule(db: &Db, input: &RuleInput) -> AppResult<Rule> {
    let id: i64 = sqlx::query_scalar(
        "INSERT INTO rss_rule
         (name, enabled, feed_id, service, media_id, title_contains, release_group,
          min_resolution, episode_from, episode_to, dest_path, category, paused, created_at)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14)
         RETURNING id",
    )
    .bind(input.name.trim())
    .bind(i64::from(input.enabled))
    .bind(input.feed_id)
    .bind(clean_opt(&input.service))
    .bind(input.media_id)
    .bind(clean_opt(&input.title_contains))
    .bind(clean_opt(&input.release_group))
    .bind(input.min_resolution)
    .bind(input.episode_from)
    .bind(input.episode_to)
    .bind(clean_opt(&input.dest_path))
    .bind(clean_opt(&input.category))
    .bind(i64::from(input.paused))
    .bind(now())
    .fetch_one(db)
    .await?;
    get_rule(db, id)
        .await?
        .ok_or_else(|| crate::error::AppError::other("rule vanished after insert"))
}

pub async fn update_rule(db: &Db, id: i64, input: &RuleInput) -> AppResult<Rule> {
    sqlx::query(
        "UPDATE rss_rule SET
           name = ?2, enabled = ?3, feed_id = ?4, service = ?5, media_id = ?6,
           title_contains = ?7, release_group = ?8, min_resolution = ?9,
           episode_from = ?10, episode_to = ?11, dest_path = ?12, category = ?13, paused = ?14
         WHERE id = ?1",
    )
    .bind(id)
    .bind(input.name.trim())
    .bind(i64::from(input.enabled))
    .bind(input.feed_id)
    .bind(clean_opt(&input.service))
    .bind(input.media_id)
    .bind(clean_opt(&input.title_contains))
    .bind(clean_opt(&input.release_group))
    .bind(input.min_resolution)
    .bind(input.episode_from)
    .bind(input.episode_to)
    .bind(clean_opt(&input.dest_path))
    .bind(clean_opt(&input.category))
    .bind(i64::from(input.paused))
    .execute(db)
    .await?;
    get_rule(db, id)
        .await?
        .ok_or_else(|| crate::error::AppError::other("no rule with that id"))
}

pub async fn delete_rule(db: &Db, id: i64) -> AppResult<()> {
    sqlx::query("DELETE FROM rss_rule WHERE id = ?1")
        .bind(id)
        .execute(db)
        .await?;
    Ok(())
}

pub async fn set_rule_enabled(db: &Db, id: i64, enabled: bool) -> AppResult<()> {
    sqlx::query("UPDATE rss_rule SET enabled = ?2 WHERE id = ?1")
        .bind(id)
        .bind(i64::from(enabled))
        .execute(db)
        .await?;
    Ok(())
}

// --------------------------------------------------------------------------- //
// RSS auto-download — download history / dedupe
// --------------------------------------------------------------------------- //

/// Has this feed-item guid already been grabbed (by any rule)?
pub async fn history_has(db: &Db, guid: &str) -> AppResult<bool> {
    let hit: Option<i64> = sqlx::query_scalar("SELECT 1 FROM rss_history WHERE guid = ?1")
        .bind(guid)
        .fetch_optional(db)
        .await?;
    Ok(hit.is_some())
}

#[allow(clippy::too_many_arguments)]
pub async fn record_history(
    db: &Db,
    guid: &str,
    rule_id: i64,
    feed_id: i64,
    title: &str,
    link: &str,
    episode: Option<i64>,
) -> AppResult<()> {
    sqlx::query(
        "INSERT INTO rss_history (guid, rule_id, feed_id, title, link, episode, downloaded_at)
         VALUES (?1,?2,?3,?4,?5,?6,?7)
         ON CONFLICT(guid) DO NOTHING",
    )
    .bind(guid)
    .bind(rule_id)
    .bind(feed_id)
    .bind(title)
    .bind(link)
    .bind(episode)
    .bind(now())
    .execute(db)
    .await?;
    Ok(())
}

pub async fn list_history(db: &Db, limit: i64) -> AppResult<Vec<HistoryEntry>> {
    let rows = sqlx::query(
        "SELECT h.guid, h.rule_id, h.feed_id, h.title, h.link, h.episode, h.downloaded_at,
            r.name AS rule_name
         FROM rss_history h
         LEFT JOIN rss_rule r ON r.id = h.rule_id
         ORDER BY h.downloaded_at DESC
         LIMIT ?1",
    )
    .bind(limit)
    .fetch_all(db)
    .await?;
    Ok(rows
        .iter()
        .map(|r| HistoryEntry {
            guid: r.get("guid"),
            rule_id: r.get("rule_id"),
            rule_name: r.get("rule_name"),
            feed_id: r.get("feed_id"),
            title: r.get("title"),
            link: r.get("link"),
            episode: r.get("episode"),
            downloaded_at: r.get("downloaded_at"),
        })
        .collect())
}

// --------------------------------------------------------------------------- //
// RSS auto-download — download client config
// --------------------------------------------------------------------------- //

const QB_CONFIG_KEY: &str = "qbittorrent_config";

pub async fn get_qb_config(db: &Db) -> AppResult<QbConfig> {
    Ok(get_setting(db, QB_CONFIG_KEY)
        .await?
        .and_then(|v| serde_json::from_value(v).ok())
        .unwrap_or_default())
}

pub async fn set_qb_config(db: &Db, cfg: &QbConfig) -> AppResult<()> {
    set_setting(db, QB_CONFIG_KEY, &serde_json::to_value(cfg)?).await
}

#[cfg(test)]
mod tests {
    use super::*;

    const TODAY: &str = "2026-09-10";

    #[test]
    fn started_date_fills_on_first_watch_only_when_blank() {
        // Planning -> Current: auto-fill.
        assert_eq!(
            auto_started_date(None, ListStatus::Planning, ListStatus::Current, 0, 1, TODAY),
            Some(TODAY.into())
        );
        // Already Current, bumping past the first episode: no change.
        assert_eq!(
            auto_started_date(None, ListStatus::Current, ListStatus::Current, 3, 4, TODAY),
            None
        );
        // Current, first episode: auto-fill.
        assert_eq!(
            auto_started_date(None, ListStatus::Current, ListStatus::Current, 0, 1, TODAY),
            Some(TODAY.into())
        );
        // A date the user set is never overwritten.
        assert_eq!(
            auto_started_date(
                Some("2020-01-01".into()),
                ListStatus::Planning,
                ListStatus::Current,
                0,
                1,
                TODAY
            ),
            Some("2020-01-01".into())
        );
    }

    #[test]
    fn completed_date_fills_on_finishing_only_when_blank() {
        assert_eq!(
            auto_completed_date(None, ListStatus::Current, ListStatus::Completed, TODAY),
            Some(TODAY.into())
        );
        // Re-saving an already-completed entry with the date cleared keeps it clear.
        assert_eq!(
            auto_completed_date(None, ListStatus::Completed, ListStatus::Completed, TODAY),
            None
        );
        // Dropped, not completed: no date.
        assert_eq!(
            auto_completed_date(None, ListStatus::Current, ListStatus::Dropped, TODAY),
            None
        );
    }
}
