//! AniList JSON -> domain model. Deliberately tolerant: missing/renamed fields
//! degrade to `Unknown`/`None` rather than failing the whole sync.

use chrono::{TimeZone, Utc};
use serde_json::Value;

use crate::tracker::model::*;

fn s(v: &Value, key: &str) -> Option<String> {
    v.get(key).and_then(|x| x.as_str()).map(str::to_owned)
}

fn i(v: &Value, key: &str) -> Option<i64> {
    v.get(key).and_then(|x| x.as_i64())
}

pub fn media_format(raw: Option<&str>) -> MediaFormat {
    match raw {
        Some("TV") => MediaFormat::Tv,
        Some("TV_SHORT") => MediaFormat::TvShort,
        Some("MOVIE") => MediaFormat::Movie,
        Some("SPECIAL") => MediaFormat::Special,
        Some("OVA") => MediaFormat::Ova,
        Some("ONA") => MediaFormat::Ona,
        Some("MUSIC") => MediaFormat::Music,
        _ => MediaFormat::Unknown,
    }
}

pub fn airing_status(raw: Option<&str>) -> AiringStatus {
    match raw {
        Some("FINISHED") => AiringStatus::Finished,
        Some("RELEASING") => AiringStatus::Releasing,
        Some("NOT_YET_RELEASED") => AiringStatus::NotYetReleased,
        Some("CANCELLED") => AiringStatus::Cancelled,
        Some("HIATUS") => AiringStatus::Hiatus,
        _ => AiringStatus::Unknown,
    }
}

pub fn season(raw: Option<&str>) -> Option<MediaSeason> {
    match raw {
        Some("WINTER") => Some(MediaSeason::Winter),
        Some("SPRING") => Some(MediaSeason::Spring),
        Some("SUMMER") => Some(MediaSeason::Summer),
        Some("FALL") => Some(MediaSeason::Fall),
        _ => None,
    }
}

fn fuzzy_date(v: &Value) -> Option<String> {
    let year = v.get("year")?.as_i64()?;
    let month = v.get("month").and_then(|x| x.as_i64()).unwrap_or(1).max(1);
    let day = v.get("day").and_then(|x| x.as_i64()).unwrap_or(1).max(1);
    Some(format!("{year:04}-{month:02}-{day:02}"))
}

pub fn media(v: &Value) -> Option<Media> {
    let id = i(v, "id")?;
    let title = v.get("title").cloned().unwrap_or(Value::Null);
    let cover = v.get("coverImage").cloned().unwrap_or(Value::Null);

    let next_airing = v.get("nextAiringEpisode").and_then(|n| {
        let episode = n.get("episode")?.as_i64()? as i32;
        let airing_at = n.get("airingAt")?.as_i64()?;
        let ts = Utc
            .timestamp_opt(airing_at, 0)
            .single()
            .map(|dt| dt.to_rfc3339())?;
        Some(AiringInfo {
            episode,
            airing_at: ts,
        })
    });

    Some(Media {
        id: MediaId {
            service: ServiceKind::AniList,
            id,
        },
        title: MediaTitle {
            romaji: s(&title, "romaji"),
            english: s(&title, "english"),
            native: s(&title, "native"),
        },
        format: media_format(v.get("format").and_then(|x| x.as_str())),
        airing_status: airing_status(v.get("status").and_then(|x| x.as_str())),
        description: s(v, "description"),
        episodes: i(v, "episodes").map(|x| x as i32),
        duration: i(v, "duration").map(|x| x as i32),
        season: season(v.get("season").and_then(|x| x.as_str())),
        season_year: i(v, "seasonYear").map(|x| x as i32),
        cover_url: s(&cover, "large"),
        cover_color: s(&cover, "color"),
        banner_url: s(v, "bannerImage"),
        average_score: i(v, "averageScore").map(|x| x as i32),
        popularity: i(v, "popularity").map(|x| x as i32),
        genres: v
            .get("genres")
            .and_then(|g| g.as_array())
            .map(|a| a.iter().filter_map(|x| x.as_str().map(str::to_owned)).collect())
            .unwrap_or_default(),
        synonyms: v
            .get("synonyms")
            .and_then(|g| g.as_array())
            .map(|a| a.iter().filter_map(|x| x.as_str().map(str::to_owned)).collect())
            .unwrap_or_default(),
        start_date: v.get("startDate").and_then(fuzzy_date),
        site_url: s(v, "siteUrl"),
        next_airing,
    })
}

pub fn list_status(raw: &str) -> ListStatus {
    raw.parse().unwrap_or(ListStatus::Planning)
}

pub fn entry(v: &Value) -> Option<MediaListEntry> {
    let media_v = v.get("media")?;
    let media = media(media_v)?;
    let status = list_status(v.get("status").and_then(|x| x.as_str()).unwrap_or("PLANNING"));
    let updated_at = i(v, "updatedAt").and_then(|ts| {
        Utc.timestamp_opt(ts, 0).single().map(|dt| dt.to_rfc3339())
    });

    Some(MediaListEntry {
        media,
        remote_id: i(v, "id"),
        status,
        progress: i(v, "progress").unwrap_or(0) as i32,
        score_raw: v
            .get("score")
            .and_then(|x| x.as_f64())
            .map(|x| x.round() as i32)
            .unwrap_or(0)
            .clamp(0, 100),
        repeat: i(v, "repeat").unwrap_or(0) as i32,
        notes: s(v, "notes"),
        started_at: v.get("startedAt").and_then(fuzzy_date),
        completed_at: v.get("completedAt").and_then(fuzzy_date),
        updated_at,
        dirty: false,
    })
}
