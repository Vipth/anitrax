//! Playback detection (M6a): "you hit Play, so we already know the show,
//! episode and file — a fixed amount of time after we notice it, offer to
//! bump progress." No window-scraping, no player IPC (that's 6b/6c); this
//! only tracks episodes opened through AniTrax's own Play button, and only
//! ever the next unwatched one (enforced by
//! [`crate::sync::prepare_watch_session`], which won't track a replay of an
//! already-seen episode).

pub mod detect;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde::Serialize;

use crate::tracker::model::ServiceKind;

/// A session is dropped once it's this old, whether or not it crossed the
/// threshold — a player left open overnight (or a sleeping PC) shouldn't
/// linger forever or fire a stale bump on wake.
const MAX_SESSION_AGE: Duration = Duration::from_secs(6 * 3600);

/// How long after a session starts (or is first detected) we consider the
/// episode "probably watched" — a flat delay, not tied to the episode's
/// actual runtime.
pub const CONFIRM_AFTER: Duration = Duration::from_secs(2 * 60);

#[derive(Debug, Clone)]
pub struct WatchSession {
    pub service: ServiceKind,
    pub media_id: i64,
    pub episode: i64,
    pub title: String,
    pub episodes_total: Option<i64>,
    pub started_at: Instant,
    /// Elapsed time at which we consider the episode "probably watched".
    pub threshold: Duration,
    notified: bool,
}

impl WatchSession {
    pub fn new(
        service: ServiceKind,
        media_id: i64,
        episode: i64,
        title: String,
        episodes_total: Option<i64>,
    ) -> Self {
        Self {
            service,
            media_id,
            episode,
            title,
            episodes_total,
            started_at: Instant::now(),
            threshold: CONFIRM_AFTER,
            notified: false,
        }
    }
}

pub type SessionKey = (ServiceKind, i64, i64);

fn key_of(s: &WatchSession) -> SessionKey {
    (s.service, s.media_id, s.episode)
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WatchSessionView {
    pub service: String,
    pub media_id: i64,
    pub episode: i64,
    pub title: String,
    pub episodes_total: Option<i64>,
    pub elapsed_secs: u64,
    pub threshold_secs: u64,
}

fn view_of(s: &WatchSession) -> WatchSessionView {
    WatchSessionView {
        service: s.service.as_str().into(),
        media_id: s.media_id,
        episode: s.episode,
        title: s.title.clone(),
        episodes_total: s.episodes_total,
        elapsed_secs: s.started_at.elapsed().as_secs(),
        threshold_secs: s.threshold.as_secs(),
    }
}

/// In-memory registry of episodes currently "being watched" (i.e. opened via
/// Play and not yet resolved). Cleared on restart — nothing here is persisted,
/// it's a live signal only.
#[derive(Clone, Default)]
pub struct PlaybackTracker {
    sessions: Arc<Mutex<HashMap<SessionKey, WatchSession>>>,
}

impl PlaybackTracker {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn start(&self, session: WatchSession) {
        self.sessions.lock().unwrap().insert(key_of(&session), session);
    }

    /// Like `start`, but leaves an existing session's clock alone — used by
    /// M6b's repeated foreground-window polling, where re-detecting the same
    /// still-playing episode every poll must not keep resetting the timer.
    pub fn touch_or_start(&self, session: WatchSession) {
        self.sessions.lock().unwrap().entry(key_of(&session)).or_insert(session);
    }

    /// Drop every tracked episode for a show at or below `progress` — called
    /// after any progress edit (manual or auto-bumped) so a stale session
    /// doesn't fire a redundant prompt later.
    pub fn stop_up_to(&self, service: ServiceKind, media_id: i64, progress: i64) {
        self.sessions
            .lock()
            .unwrap()
            .retain(|k, _| !(k.0 == service && k.1 == media_id && k.2 <= progress));
    }

    /// For the "Now watching" strip.
    pub fn list(&self) -> Vec<WatchSessionView> {
        self.sessions.lock().unwrap().values().map(view_of).collect()
    }

    /// Sessions that just crossed their threshold since the last tick (each is
    /// returned at most once), plus housekeeping: prune stale sessions and drop
    /// ones that were already reported.
    pub fn tick(&self) -> Vec<WatchSessionView> {
        let mut sessions = self.sessions.lock().unwrap();
        sessions.retain(|_, s| !s.notified && s.started_at.elapsed() < MAX_SESSION_AGE);
        let mut ready = Vec::new();
        for s in sessions.values_mut() {
            if s.started_at.elapsed() >= s.threshold {
                s.notified = true;
                ready.push(view_of(s));
            }
        }
        ready
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn session() -> WatchSession {
        WatchSession::new(ServiceKind::AniList, 1, 5, "Show".into(), Some(12))
    }

    fn backdated(mut s: WatchSession, ago: Duration) -> WatchSession {
        s.started_at = Instant::now() - ago;
        s
    }

    #[test]
    fn threshold_is_a_flat_delay_regardless_of_runtime() {
        assert_eq!(session().threshold, CONFIRM_AFTER);
    }

    #[test]
    fn tick_reports_a_crossed_session_exactly_once() {
        let tracker = PlaybackTracker::new();
        tracker.start(backdated(session(), CONFIRM_AFTER + Duration::from_secs(1)));

        let first = tracker.tick();
        assert_eq!(first.len(), 1);
        assert_eq!(first[0].episode, 5);

        // Already reported — gone on the next tick, not re-sent.
        assert!(tracker.tick().is_empty());
        assert!(tracker.list().is_empty());
    }

    #[test]
    fn tick_ignores_a_session_still_under_threshold() {
        let tracker = PlaybackTracker::new();
        tracker.start(session()); // just started, threshold is CONFIRM_AFTER out
        assert!(tracker.tick().is_empty());
        assert_eq!(tracker.list().len(), 1);
    }

    #[test]
    fn stop_up_to_clears_at_or_below_but_keeps_later_episodes() {
        let tracker = PlaybackTracker::new();
        tracker.start(WatchSession::new(ServiceKind::AniList, 1, 5, "A".into(), None));
        tracker.start(WatchSession::new(ServiceKind::AniList, 1, 6, "A".into(), None));
        tracker.stop_up_to(ServiceKind::AniList, 1, 5);
        let remaining = tracker.list();
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0].episode, 6);
    }

    #[test]
    fn stale_sessions_are_pruned_without_firing() {
        let tracker = PlaybackTracker::new();
        tracker.start(backdated(session(), MAX_SESSION_AGE + Duration::from_secs(1)));
        assert!(tracker.tick().is_empty());
        assert!(tracker.list().is_empty());
    }
}
