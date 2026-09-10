//! Torrent download clients. The RSS scheduler only ever talks to a
//! `dyn DownloadClient`, so a second client (Transmission, Deluge) is a new
//! file here — not a change to the scheduler.

pub mod qbittorrent;

use serde::{Deserialize, Serialize};

use crate::error::AppResult;

/// Everything a client needs to add one torrent.
#[derive(Debug, Clone)]
pub struct AddTorrent {
    /// A magnet URI or an `http(s)` `.torrent` URL. qBittorrent accepts either.
    pub link: String,
    /// Absolute save path, or `None` to use the client's default / category path.
    pub save_path: Option<String>,
    /// Client-side category, or `None`.
    pub category: Option<String>,
    /// Add in a stopped state.
    pub paused: bool,
}

/// Connection settings for a client. Stored as one JSON blob in `app_setting`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct QbConfig {
    /// e.g. `http://localhost:8080` — no trailing slash, no `/api` suffix.
    #[serde(default)]
    pub base_url: String,
    #[serde(default)]
    pub username: String,
    #[serde(default)]
    pub password: String,
}

impl QbConfig {
    pub fn is_configured(&self) -> bool {
        !self.base_url.trim().is_empty()
    }
}

#[async_trait::async_trait]
pub trait DownloadClient: Send + Sync {
    /// Human name for logs / the UI ("qBittorrent"). Used once a second client
    /// exists and the scheduler has to say which one it handed a torrent to.
    #[allow(dead_code)]
    fn name(&self) -> &'static str;

    /// Authenticate (if needed) and confirm the client is reachable. Returns the
    /// client's version string on success.
    async fn test_connection(&self) -> AppResult<String>;

    /// Add one torrent. Implementations authenticate lazily.
    async fn add(&self, torrent: &AddTorrent) -> AppResult<()>;
}
