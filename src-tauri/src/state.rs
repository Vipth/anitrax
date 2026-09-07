use std::sync::Arc;

use tauri::{AppHandle, Manager};
use tokio::sync::Notify;

use crate::db::{self, Db};
use crate::error::AppResult;
use crate::tracker::anilist::{AniList, AniListGateway};

/// Shared application state, cloned into every command and background task.
#[derive(Clone)]
pub struct AppState {
    pub db: Db,
    pub anilist: Arc<AniList>,
    pub push: PushSignal,
}

impl AppState {
    pub async fn init(app: &AppHandle) -> AppResult<Self> {
        let dir = app
            .path()
            .app_data_dir()
            .map_err(|e| crate::error::AppError::other(format!("no app data dir: {e}")))?;
        let db_path = dir.join("anime-tracker.db");
        let db = db::connect(&db_path).await?;

        let gateway = AniListGateway::spawn();
        let anilist = Arc::new(AniList::new(gateway));

        Ok(Self {
            db,
            anilist,
            push: PushSignal::new(),
        })
    }

    pub fn gateway(&self) -> &AniListGateway {
        self.anilist.gateway()
    }
}

/// A debounce trigger for the background push worker. Callers `nudge()` after a
/// local edit; the worker waits out a quiet period, then flushes all dirty rows
/// in one pass so rapid `+1` clicks collapse into a single mutation.
#[derive(Clone)]
pub struct PushSignal {
    notify: Arc<Notify>,
}

impl PushSignal {
    fn new() -> Self {
        Self {
            notify: Arc::new(Notify::new()),
        }
    }

    pub fn nudge(&self) {
        self.notify.notify_one();
    }

    pub async fn wait(&self) {
        self.notify.notified().await;
    }
}
