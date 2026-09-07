//! Typed helpers over the SQLite schema. Every query is plain runtime SQL so the
//! crate builds without a live database at compile time.

use chrono::Utc;
use serde_json::Value;
use sqlx::Row;

use crate::error::AppResult;
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
            next_episode, next_airing_at, meta_fetched_at, airing_fetched_at)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,?20,?21,?22,?23,?24)
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
            airing_fetched_at = excluded.airing_fetched_at",
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
            next_episode, next_airing_at, meta_fetched_at, airing_fetched_at)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,?20,?21,?22,?23,?24)
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
            airing_fetched_at = excluded.airing_fetched_at",
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
        genres,
        synonyms,
        start_date: r.get("start_date"),
        site_url: r.get("site_url"),
        next_airing,
    }
}

// --------------------------------------------------------------------------- //
// List entries
// --------------------------------------------------------------------------- //

const ENTRY_SELECT_BASE: &str = "SELECT
    e.remote_id, e.status, e.progress, e.score_raw, e.repeat, e.notes,
    e.started_at, e.completed_at, e.updated_at, e.dirty,
    m.service, m.id, m.title_romaji, m.title_english, m.title_native, m.format,
    m.airing_status, m.description, m.episodes, m.duration, m.season, m.season_year,
    m.cover_url, m.cover_color, m.banner_url, m.average_score, m.genres_json,
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
    .bind(&base.started_at)
    .bind(&base.completed_at)
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
