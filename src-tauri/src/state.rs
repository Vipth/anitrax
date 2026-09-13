use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
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
    /// General-purpose HTTP client for non-AniList traffic (RSS feed fetches).
    /// AniList never uses this — it goes through the paced gateway.
    pub http: reqwest::Client,
    /// Serialises RSS feed checks so the timer and a manual "check now" can't
    /// run concurrently and double-add a torrent.
    pub rss_lock: Arc<tokio::sync::Mutex<()>>,
    /// Mirrors the `close_to_tray` setting in memory so the (synchronous)
    /// window close-request handler doesn't need to hit the DB. Kept in sync
    /// by `set_close_to_tray`.
    pub close_to_tray: Arc<AtomicBool>,
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

        let http = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(20))
            .build()
            .unwrap_or_default();

        let close_to_tray = repo::get_bool_setting(&db, crate::sync::CLOSE_TO_TRAY_KEY, true)
            .await
            .unwrap_or(true);

        Ok(Self {
            db,
            anilist,
            push: PushSignal::new(),
            watcher: LibraryWatcherHandle::new(),
            http,
            rss_lock: Arc::new(tokio::sync::Mutex::new(())),
            close_to_tray: Arc::new(AtomicBool::new(close_to_tray)),
        })
    }

    pub fn gateway(&self) -> &AniListGateway {
        self.anilist.gateway()
    }
}

/// Convenience wrappers over the atomics above.
impl AppState {
    pub fn is_close_to_tray(&self) -> bool {
        self.close_to_tray.load(Ordering::Relaxed)
    }

    pub fn set_close_to_tray_flag(&self, v: bool) {
        self.close_to_tray.store(v, Ordering::Relaxed);
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
