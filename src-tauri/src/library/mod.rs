//! Local library: scan watched folders, parse file names, match them to cached
//! media. Everything here is offline — the only network cost is an optional
//! title search when the user links an unmatched file by hand.

pub mod matcher;
pub mod scanner;
pub mod watcher;

use serde::Serialize;

use crate::tracker::model::MediaTitle;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryFolder {
    pub id: i64,
    pub path: String,
    pub enabled: bool,
    pub added_at: String,
    pub scanned_at: Option<String>,
    pub file_count: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryFile {
    pub id: i64,
    pub folder_id: i64,
    pub path: String,
    pub file_name: String,
    pub size_bytes: Option<i64>,
    pub modified_at: Option<String>,
    pub parsed_title: Option<String>,
    pub folder_title: Option<String>,
    pub parsed_episode: Option<i64>,
    pub parsed_season: Option<i64>,
    pub resolution: Option<String>,
    pub release_group: Option<String>,
    pub service: Option<String>,
    pub media_id: Option<i64>,
    pub match_kind: Option<String>,
    pub match_score: Option<f64>,
    pub scanned_at: String,
    /// Joined from `media_cache` for display; `None` when unmatched.
    pub media_title: Option<MediaTitle>,
}

/// Which episodes of a given media are present on disk.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OwnedMedia {
    pub media_id: i64,
    pub episodes: Vec<i64>,
}

/// A remembered manual link: "files whose folder/title normalises to `titleKey`
/// (and season `season`, if set) belong to this media".
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LinkRule {
    pub id: i64,
    pub title_key: String,
    pub season: Option<i64>,
    pub service: String,
    pub media_id: i64,
    pub created_at: String,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanReport {
    pub folders: usize,
    pub files_seen: usize,
    pub files_removed: usize,
    pub auto_matched: usize,
    pub rule_matched: usize,
    pub unmatched: usize,
    pub finished_at: String,
}
