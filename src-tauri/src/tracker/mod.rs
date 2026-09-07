pub mod anilist;
pub mod model;

use crate::error::AppResult;
use model::*;

/// A remote tracking service (AniList today, Kitsu in milestone 3).
///
/// The rest of the app only ever talks to a `dyn TrackerService`, so adding a
/// second service is additive — no call site changes.
#[async_trait::async_trait]
pub trait TrackerService: Send + Sync {
    /// Which service this is. Used for routing once Kitsu lands.
    #[allow(dead_code)]
    fn kind(&self) -> ServiceKind;

    /// The authenticated user for `token`.
    async fn viewer(&self, token: &str) -> AppResult<Viewer>;

    /// The user's entire anime list in as few requests as possible.
    async fn full_list(&self, token: &str, user_id: i64) -> AppResult<Vec<MediaListEntry>>;

    /// Create or update one entry; returns the server's canonical version.
    async fn save_entry(&self, token: &str, patch: &EntryPatch) -> AppResult<MediaListEntry>;

    /// Remove an entry from the list.
    async fn delete_entry(&self, token: &str, remote_id: i64) -> AppResult<()>;

    /// Title search for the discover screen (unauthenticated).
    async fn search(&self, query: &str) -> AppResult<Vec<Media>>;

    /// Fetch many media objects by id in batched requests (unauthenticated).
    async fn media_batch(&self, ids: &[i64]) -> AppResult<Vec<Media>>;
}
