//! Watch statistics, computed in Rust from the local cache — no network.

use std::collections::HashMap;

use serde::Serialize;

use crate::tracker::model::{ListStatus, MediaListEntry};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Bucket {
    /// Raw key — a status/format enum name, a genre, or a score/month label.
    pub key: String,
    pub count: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StatsData {
    pub total: i64,
    /// Entries per list status, in reading order.
    pub by_status: Vec<Bucket>,
    /// Distinct episodes watched (progress), plus rewatches.
    pub episodes_watched: i64,
    pub minutes_watched: i64,
    /// Mean of the entries that carry a score, on the 0–100 scale. 0 when none.
    pub mean_score: f64,
    pub scored_count: i64,
    /// Score histogram, keys "1".."10".
    pub score_buckets: Vec<Bucket>,
    /// Most-tracked genres, biggest first (top 12).
    pub top_genres: Vec<Bucket>,
    pub by_format: Vec<Bucket>,
    /// Completed / (everything the user actually started).
    pub completion_rate: f64,
    /// Completions per month for the last 12 months, oldest first ("YYYY-MM").
    pub activity: Vec<Bucket>,
}

const STATUS_ORDER: [ListStatus; 6] = [
    ListStatus::Current,
    ListStatus::Repeating,
    ListStatus::Planning,
    ListStatus::Paused,
    ListStatus::Completed,
    ListStatus::Dropped,
];

pub fn compute(entries: &[MediaListEntry], now: chrono::DateTime<chrono::Utc>) -> StatsData {
    let total = entries.len() as i64;

    let mut status_counts: HashMap<&str, i64> = HashMap::new();
    let mut format_counts: HashMap<String, i64> = HashMap::new();
    let mut genre_counts: HashMap<String, i64> = HashMap::new();
    let mut score_buckets = [0i64; 10];
    let mut score_sum = 0i64;
    let mut scored_count = 0i64;
    let mut episodes_watched = 0i64;
    let mut minutes_watched = 0i64;

    for e in entries {
        *status_counts.entry(e.status.as_str()).or_default() += 1;
        *format_counts.entry(e.media.format.as_str().to_string()).or_default() += 1;
        for g in &e.media.genres {
            *genre_counts.entry(g.clone()).or_default() += 1;
        }

        if e.score_raw > 0 {
            score_sum += e.score_raw as i64;
            scored_count += 1;
            let b = ((e.score_raw as f64) / 10.0).round().clamp(1.0, 10.0) as usize;
            score_buckets[b - 1] += 1;
        }

        // progress + full rewatches (repeat * episode count, or progress if unknown)
        let per_watch = e.media.episodes.map(i64::from).unwrap_or(e.progress as i64);
        let watched = e.progress as i64 + e.repeat as i64 * per_watch;
        episodes_watched += watched;
        if let Some(dur) = e.media.duration {
            minutes_watched += watched * i64::from(dur);
        }
    }

    let by_status = STATUS_ORDER
        .iter()
        .map(|s| Bucket {
            key: s.as_str().to_string(),
            count: status_counts.get(s.as_str()).copied().unwrap_or(0),
        })
        .collect();

    let mut by_format: Vec<Bucket> = format_counts
        .into_iter()
        .filter(|(k, _)| k != "UNKNOWN")
        .map(|(key, count)| Bucket { key, count })
        .collect();
    by_format.sort_by_key(|b| std::cmp::Reverse(b.count));

    let mut top_genres: Vec<Bucket> = genre_counts
        .into_iter()
        .map(|(key, count)| Bucket { key, count })
        .collect();
    top_genres.sort_by(|a, b| b.count.cmp(&a.count).then(a.key.cmp(&b.key)));
    top_genres.truncate(12);

    let score_buckets = (0..10)
        .map(|i| Bucket {
            key: (i + 1).to_string(),
            count: score_buckets[i],
        })
        .collect();

    let mean_score = if scored_count > 0 {
        score_sum as f64 / scored_count as f64
    } else {
        0.0
    };

    // "Started" = anything past Planning.
    let started: i64 = STATUS_ORDER
        .iter()
        .filter(|s| !matches!(s, ListStatus::Planning))
        .map(|s| status_counts.get(s.as_str()).copied().unwrap_or(0))
        .sum();
    let completed = status_counts.get(ListStatus::Completed.as_str()).copied().unwrap_or(0);
    let completion_rate = if started > 0 {
        completed as f64 / started as f64
    } else {
        0.0
    };

    let activity = monthly_completions(entries, now);

    StatsData {
        total,
        by_status,
        episodes_watched,
        minutes_watched,
        mean_score,
        scored_count,
        score_buckets,
        top_genres,
        by_format,
        completion_rate,
        activity,
    }
}

/// Completions per month for the trailing 12 months (oldest first).
fn monthly_completions(
    entries: &[MediaListEntry],
    now: chrono::DateTime<chrono::Utc>,
) -> Vec<Bucket> {
    use chrono::Datelike;

    // Build the 12 month keys ending with the current month.
    let mut months: Vec<(String, i64)> = Vec::with_capacity(12);
    let (mut y, mut m) = (now.year(), now.month() as i32);
    for _ in 0..12 {
        months.push((format!("{y:04}-{m:02}"), 0));
        m -= 1;
        if m == 0 {
            m = 12;
            y -= 1;
        }
    }
    months.reverse();
    let index: HashMap<String, usize> = months
        .iter()
        .enumerate()
        .map(|(i, (k, _))| (k.clone(), i))
        .collect();

    for e in entries {
        if e.status != ListStatus::Completed {
            continue;
        }
        let Some(date) = e.completed_at.as_deref() else {
            continue;
        };
        // completed_at is "YYYY-MM-DD"; take the "YYYY-MM".
        if date.len() >= 7 {
            if let Some(&i) = index.get(&date[..7]) {
                months[i].1 += 1;
            }
        }
    }

    months
        .into_iter()
        .map(|(key, count)| Bucket { key, count })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tracker::model::*;

    fn entry(status: ListStatus, progress: i32, score: i32, genres: &[&str]) -> MediaListEntry {
        MediaListEntry {
            media: Media {
                id: MediaId { service: ServiceKind::AniList, id: 1 },
                title: MediaTitle::default(),
                format: MediaFormat::Tv,
                airing_status: AiringStatus::Finished,
                description: None,
                episodes: Some(12),
                duration: Some(24),
                season: None,
                season_year: None,
                cover_url: None,
                cover_color: None,
                banner_url: None,
                average_score: None,
                popularity: None,
                genres: genres.iter().map(|s| s.to_string()).collect(),
                synonyms: vec![],
                start_date: None,
                site_url: None,
                next_airing: None,
            },
            remote_id: None,
            status,
            progress,
            score_raw: score,
            repeat: 0,
            notes: None,
            started_at: None,
            completed_at: None,
            updated_at: None,
            dirty: false,
        }
    }

    #[test]
    fn totals_and_watch_time() {
        let entries = vec![
            entry(ListStatus::Completed, 12, 90, &["Action"]),
            entry(ListStatus::Current, 5, 0, &["Action", "Comedy"]),
        ];
        let s = compute(&entries, chrono::Utc::now());
        assert_eq!(s.total, 2);
        assert_eq!(s.episodes_watched, 17);
        assert_eq!(s.minutes_watched, 17 * 24);
        assert_eq!(s.scored_count, 1);
        assert_eq!(s.mean_score, 90.0);
    }

    #[test]
    fn score_bucket_rounds_to_nearest_ten() {
        let entries = vec![
            entry(ListStatus::Completed, 12, 84, &[]), // -> 8
            entry(ListStatus::Completed, 12, 85, &[]), // -> 9 (round half up)
            entry(ListStatus::Completed, 12, 100, &[]), // -> 10
        ];
        let s = compute(&entries, chrono::Utc::now());
        let get = |k: &str| s.score_buckets.iter().find(|b| b.key == k).unwrap().count;
        assert_eq!(get("8"), 1);
        assert_eq!(get("9"), 1);
        assert_eq!(get("10"), 1);
    }

    #[test]
    fn completion_rate_ignores_planning() {
        let entries = vec![
            entry(ListStatus::Completed, 12, 0, &[]),
            entry(ListStatus::Dropped, 3, 0, &[]),
            entry(ListStatus::Planning, 0, 0, &[]),
            entry(ListStatus::Planning, 0, 0, &[]),
        ];
        let s = compute(&entries, chrono::Utc::now());
        assert!((s.completion_rate - 0.5).abs() < 1e-9);
    }

    #[test]
    fn top_genres_are_ranked() {
        let entries = vec![
            entry(ListStatus::Completed, 1, 0, &["Action", "Comedy"]),
            entry(ListStatus::Completed, 1, 0, &["Action"]),
            entry(ListStatus::Completed, 1, 0, &["Drama"]),
        ];
        let s = compute(&entries, chrono::Utc::now());
        assert_eq!(s.top_genres[0].key, "Action");
        assert_eq!(s.top_genres[0].count, 2);
    }

    #[test]
    fn activity_has_twelve_months_oldest_first() {
        let s = compute(&[], chrono::Utc::now());
        assert_eq!(s.activity.len(), 12);
        assert!(s.activity[0].key < s.activity[11].key);
    }
}
