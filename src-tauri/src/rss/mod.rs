//! RSS auto-download (M5). Poll torrent feeds, match new releases to a tracked
//! show with a user-tuned rule, and hand the magnet / `.torrent` URL to a
//! [`DownloadClient`](crate::download::DownloadClient). Progress stays manual —
//! this only fetches files, it never touches your AniList list.

pub mod feeds;
pub mod rules;
pub mod scheduler;

use serde::{Deserialize, Serialize};

use crate::tracker::model::MediaTitle;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Feed {
    pub id: i64,
    pub name: String,
    pub url: String,
    pub enabled: bool,
    pub added_at: String,
    pub last_fetched_at: Option<String>,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Rule {
    pub id: i64,
    pub name: String,
    pub enabled: bool,
    pub feed_id: Option<i64>,
    pub service: Option<String>,
    pub media_id: Option<i64>,
    pub title_contains: Option<String>,
    pub release_group: Option<String>,
    pub min_resolution: Option<i64>,
    pub episode_from: Option<i64>,
    pub episode_to: Option<i64>,
    pub dest_path: Option<String>,
    pub category: Option<String>,
    pub paused: bool,
    pub created_at: String,
    /// Joined from `media_cache` for display; `None` when the target show isn't
    /// cached (or the rule has no target).
    pub media_title: Option<MediaTitle>,
}

/// Create / update payload from the frontend. `id` is ignored on create.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuleInput {
    #[serde(default)]
    pub name: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
    pub feed_id: Option<i64>,
    pub service: Option<String>,
    pub media_id: Option<i64>,
    pub title_contains: Option<String>,
    pub release_group: Option<String>,
    pub min_resolution: Option<i64>,
    pub episode_from: Option<i64>,
    pub episode_to: Option<i64>,
    pub dest_path: Option<String>,
    pub category: Option<String>,
    #[serde(default)]
    pub paused: bool,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryEntry {
    pub guid: String,
    pub rule_id: Option<i64>,
    pub rule_name: Option<String>,
    pub feed_id: Option<i64>,
    pub title: String,
    pub link: String,
    pub episode: Option<i64>,
    pub downloaded_at: String,
}

/// Outcome of one "check the feeds" pass, surfaced to the UI.
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CheckReport {
    pub feeds_checked: usize,
    pub items_seen: usize,
    pub added: usize,
    pub errors: Vec<String>,
    pub finished_at: String,
}
