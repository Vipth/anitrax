use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use tauri::{AppHandle, Manager};
use tokio::sync::{mpsc, Notify};

use crate::db::{self, repo, Db};
use crate::error::AppResult;
use crate::library::watcher::LibraryWatcher;
use crate::tracker::anilist::{AniList, AniListGateway};

/// Shared application state, cloned into every command and background task.
#[derive(Clone)]
pub struct AppState {
    pub db: Db,
    pub anilist: Arc<AniList>,
    pub push: PushSignal,
    pub watcher: LibraryWatcherHandle,
}

impl AppState {
    pub async fn init(app: &AppHandle) -> AppResult<Self> {
        let dir = app
            .path()
            .app_data_dir()
            .map_err(|e| crate::error::AppError::other(format!("no app data dir: {e}")))?;
        let db_path = dir.join("anitrax.db");
        let db = db::connect(&db_path).await?;

        let gateway = AniListGateway::spawn();
        let anilist = Arc::new(AniList::new(gateway));

        Ok(Self {
            db,
            anilist,
            push: PushSignal::new(),
            watcher: LibraryWatcherHandle::new(),
        })
    }

    pub fn gateway(&self) -> &AniListGateway {
        self.anilist.gateway()
    }
}

/// Owns the filesystem watcher over the enabled watched folders and lets the
/// rest of the app rebuild it when that set changes. Change notifications are
/// delivered on a channel `lib.rs` drains into incremental rescans.
#[derive(Clone)]
pub struct LibraryWatcherHandle {
    inner: Arc<Mutex<Option<LibraryWatcher>>>,
    tx: mpsc::UnboundedSender<Vec<PathBuf>>,
    rx: Arc<Mutex<Option<mpsc::UnboundedReceiver<Vec<PathBuf>>>>>,
}

impl LibraryWatcherHandle {
    fn new() -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        Self {
            inner: Arc::new(Mutex::new(None)),
            tx,
            rx: Arc::new(Mutex::new(Some(rx))),
        }
    }

    /// Taken once, by the background task that turns change events into rescans.
    pub fn take_receiver(&self) -> Option<mpsc::UnboundedReceiver<Vec<PathBuf>>> {
        self.rx.lock().unwrap().take()
    }

    /// Rebuild the watcher from the current set of enabled folders.
    pub async fn refresh(&self, state: &AppState) {
        let roots: Vec<PathBuf> = match repo::enabled_library_folders(&state.db).await {
            Ok(v) => v.into_iter().map(|(_, p)| PathBuf::from(p)).collect(),
            Err(e) => {
                tracing::warn!(?e, "could not read watched folders");
                return;
            }
        };
        let mut guard = self.inner.lock().unwrap();
        if let Some(old) = guard.take() {
            old.stop();
        }
        if roots.is_empty() {
            return;
        }
        match LibraryWatcher::start(roots, self.tx.clone()) {
            Ok(w) => *guard = Some(w),
            Err(e) => tracing::warn!(?e, "library watcher failed to start"),
        }
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
