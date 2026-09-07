//! Filesystem watcher over the enabled watched folders. Coalesces bursts of
//! events (a torrent finishing writes many) into a single debounced "something
//! under here changed" signal, which the caller turns into an incremental
//! rescan.

use std::path::PathBuf;
use std::time::Duration;

use notify::{RecommendedWatcher, RecursiveMode};
use notify_debouncer_full::{new_debouncer, DebounceEventResult, Debouncer, RecommendedCache};
use tokio::sync::mpsc;

/// How long to wait for the filesystem to settle before emitting.
const DEBOUNCE: Duration = Duration::from_secs(3);

pub struct LibraryWatcher {
    debouncer: Debouncer<RecommendedWatcher, RecommendedCache>,
    roots: Vec<PathBuf>,
}

impl LibraryWatcher {
    /// Start watching `roots`. Debounced change notifications (the set of
    /// affected roots) are sent on `tx`.
    pub fn start(
        roots: Vec<PathBuf>,
        tx: mpsc::UnboundedSender<Vec<PathBuf>>,
    ) -> notify::Result<Self> {
        let watched = roots.clone();
        let mut debouncer = new_debouncer(
            DEBOUNCE,
            None,
            move |result: DebounceEventResult| {
                let Ok(events) = result else { return };
                let mut hit: Vec<PathBuf> = Vec::new();
                for event in events {
                    for path in &event.paths {
                        if let Some(root) = watched.iter().find(|r| path.starts_with(r)) {
                            if !hit.contains(root) {
                                hit.push(root.clone());
                            }
                        }
                    }
                }
                if !hit.is_empty() {
                    let _ = tx.send(hit);
                }
            },
        )?;

        for root in &roots {
            // A folder on an offline drive just won't watch — that's fine.
            let _ = debouncer.watch(root, RecursiveMode::Recursive);
        }

        Ok(Self { debouncer, roots })
    }

    pub fn stop(mut self) {
        for root in std::mem::take(&mut self.roots) {
            let _ = self.debouncer.unwatch(&root);
        }
    }
}
