//! Service-agnostic domain model. Both AniList and Kitsu map into these types
//! so the frontend never needs to know which service an entry came from.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ServiceKind {
    AniList,
    Kitsu,
}

impl ServiceKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            ServiceKind::AniList => "anilist",
            ServiceKind::Kitsu => "kitsu",
        }
    }
}

impl std::fmt::Display for ServiceKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for ServiceKind {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "anilist" => Ok(ServiceKind::AniList),
            "kitsu" => Ok(ServiceKind::Kitsu),
            other => Err(format!("unknown service '{other}'")),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MediaId {
    pub service: ServiceKind,
    pub id: i64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaTitle {
    pub romaji: Option<String>,
    pub english: Option<String>,
    pub native: Option<String>,
}

impl MediaTitle {
    #[allow(dead_code)] // used by Rust-side ordering helpers in later milestones
    pub fn preferred(&self) -> String {
        self.english
            .clone()
            .or_else(|| self.romaji.clone())
            .or_else(|| self.native.clone())
            .unwrap_or_else(|| "Unknown".into())
    }
}

/// Where a show is in its broadcast lifecycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AiringStatus {
    Finished,
    Releasing,
    NotYetReleased,
    Cancelled,
    Hiatus,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum MediaFormat {
    Tv,
    TvShort,
    Movie,
    Special,
    Ova,
    Ona,
    Music,
    Unknown,
}

impl MediaFormat {
    pub fn as_str(&self) -> &'static str {
        match self {
            MediaFormat::Tv => "TV",
            MediaFormat::TvShort => "TV_SHORT",
            MediaFormat::Movie => "MOVIE",
            MediaFormat::Special => "SPECIAL",
            MediaFormat::Ova => "OVA",
            MediaFormat::Ona => "ONA",
            MediaFormat::Music => "MUSIC",
            MediaFormat::Unknown => "UNKNOWN",
        }
    }
    pub fn from_opt_str(s: Option<&str>) -> Self {
        match s {
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
}

impl AiringStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            AiringStatus::Finished => "FINISHED",
            AiringStatus::Releasing => "RELEASING",
            AiringStatus::NotYetReleased => "NOT_YET_RELEASED",
            AiringStatus::Cancelled => "CANCELLED",
            AiringStatus::Hiatus => "HIATUS",
            AiringStatus::Unknown => "UNKNOWN",
        }
    }
    pub fn from_opt_str(s: Option<&str>) -> Self {
        match s {
            Some("FINISHED") => AiringStatus::Finished,
            Some("RELEASING") => AiringStatus::Releasing,
            Some("NOT_YET_RELEASED") => AiringStatus::NotYetReleased,
            Some("CANCELLED") => AiringStatus::Cancelled,
            Some("HIATUS") => AiringStatus::Hiatus,
            _ => AiringStatus::Unknown,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum MediaSeason {
    Winter,
    Spring,
    Summer,
    Fall,
}

impl MediaSeason {
    pub fn as_str(&self) -> &'static str {
        match self {
            MediaSeason::Winter => "WINTER",
            MediaSeason::Spring => "SPRING",
            MediaSeason::Summer => "SUMMER",
            MediaSeason::Fall => "FALL",
        }
    }
    pub fn from_opt_str(s: Option<&str>) -> Option<Self> {
        match s {
            Some("WINTER") => Some(MediaSeason::Winter),
            Some("SPRING") => Some(MediaSeason::Spring),
            Some("SUMMER") => Some(MediaSeason::Summer),
            Some("FALL") => Some(MediaSeason::Fall),
            _ => None,
        }
    }
}

/// The user's watch status for an entry. Names follow AniList; Kitsu maps onto these.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ListStatus {
    Current,
    Planning,
    Completed,
    Dropped,
    Paused,
    Repeating,
}

impl ListStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            ListStatus::Current => "CURRENT",
            ListStatus::Planning => "PLANNING",
            ListStatus::Completed => "COMPLETED",
            ListStatus::Dropped => "DROPPED",
            ListStatus::Paused => "PAUSED",
            ListStatus::Repeating => "REPEATING",
        }
    }
}

impl std::str::FromStr for ListStatus {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_uppercase().as_str() {
            "CURRENT" => Ok(ListStatus::Current),
            "PLANNING" => Ok(ListStatus::Planning),
            "COMPLETED" => Ok(ListStatus::Completed),
            "DROPPED" => Ok(ListStatus::Dropped),
            "PAUSED" => Ok(ListStatus::Paused),
            "REPEATING" => Ok(ListStatus::Repeating),
            other => Err(format!("unknown list status '{other}'")),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiringInfo {
    pub episode: i32,
    /// RFC3339 timestamp.
    pub airing_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Media {
    pub id: MediaId,
    pub title: MediaTitle,
    pub format: MediaFormat,
    pub airing_status: AiringStatus,
    pub description: Option<String>,
    pub episodes: Option<i32>,
    pub duration: Option<i32>,
    pub season: Option<MediaSeason>,
    pub season_year: Option<i32>,
    pub cover_url: Option<String>,
    pub cover_color: Option<String>,
    pub banner_url: Option<String>,
    pub average_score: Option<i32>,
    /// AniList `popularity` — how many users have this on a list.
    pub popularity: Option<i32>,
    pub genres: Vec<String>,
    pub synonyms: Vec<String>,
    /// RFC3339 date (may be date-only precision).
    pub start_date: Option<String>,
    pub site_url: Option<String>,
    pub next_airing: Option<AiringInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaListEntry {
    pub media: Media,
    pub remote_id: Option<i64>,
    pub status: ListStatus,
    pub progress: i32,
    /// 0..=100 regardless of the user's display format.
    pub score_raw: i32,
    pub repeat: i32,
    pub notes: Option<String>,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
    pub updated_at: Option<String>,
    #[serde(default)]
    pub dirty: bool,
}

/// A partial update to an entry. `None` fields are left unchanged.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EntryPatch {
    pub media_id: i64,
    pub remote_id: Option<i64>,
    pub status: Option<ListStatus>,
    pub progress: Option<i32>,
    pub score_raw: Option<i32>,
    pub repeat: Option<i32>,
    pub notes: Option<String>,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Viewer {
    pub service: ServiceKind,
    pub id: i64,
    pub name: String,
    pub avatar_url: Option<String>,
    pub score_format: String,
}
