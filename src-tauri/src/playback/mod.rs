//! Playback detection (M6a): "you hit Play, so we already know the show,
//! episode and file — once enough wall-clock time has passed for a typical
//! viewer to have finished it, offer to bump progress." No window-scraping,
//! no player IPC (that's 6b/6c); this only tracks episodes opened through
//! AniTrax's own Play button, and only ever the next unwatched one (enforced
//! by [`crate::sync::prepare_watch_session`], which won't track a replay of an
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

#[derive(Debug, Clone)]
pub struct WatchSession {
    pub service: ServiceKind,
    pub media_id: i64,
    pub episode: i64,
    pub title: String,
    pub episodes_total: Option<i64>,
    pub started_at: Instant,
    /// Elapsed time at which we consider the episode "probably watched" —
    /// the earlier of ~80% of the runtime or the last few minutes of it.
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
        duration_minutes: Option<i32>,
    ) -> Self {
        let duration_secs = duration_minutes.filter(|m| *m > 0).unwrap_or(20) as u64 * 60;
        let eighty_pct = duration_secs * 80 / 100;
        let minus_three_min = duration_secs.saturating_sub(180);
        let threshold = Duration::from_secs(eighty_pct.min(minus_three_min).max(60));
        Self {
            service,
            media_id,
            episode,
            title,
            episodes_total,
            started_at: Instant::now(),
            threshold,
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

    fn session(minutes: Option<i32>) -> WatchSession {
        WatchSession::new(ServiceKind::AniList, 1, 5, "Show".into(), Some(12), minutes)
    }

    fn backdated(mut s: WatchSession, ago: Duration) -> WatchSession {
        s.started_at = Instant::now() - ago;
        s
    }

    #[test]
    fn threshold_is_the_earlier_of_80pct_or_minus_3min() {
        // 24 min: 80% = 19.2min, dur-3min = 21min -> 80% wins.
        assert_eq!(session(Some(24)).threshold, Duration::from_secs(1152));
        // 5 min: 80% = 4min, dur-3min = 2min -> dur-3min wins.
        assert_eq!(session(Some(5)).threshold, Duration::from_secs(120));
        // Missing duration falls back to 20 minutes, same rule.
        assert_eq!(session(None).threshold, Duration::from_secs(960));
        // Pathologically short duration floors at 60s, never negative/zero.
        assert_eq!(session(Some(1)).threshold, Duration::from_secs(60));
    }

    #[test]
    fn tick_reports_a_crossed_session_exactly_once() {
        let tracker = PlaybackTracker::new();
        tracker.start(backdated(session(Some(1)), Duration::from_secs(120)));

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
        tracker.start(session(Some(24))); // just started, threshold is ~19min out
        assert!(tracker.tick().is_empty());
        assert_eq!(tracker.list().len(), 1);
    }

    #[test]
    fn stop_up_to_clears_at_or_below_but_keeps_later_episodes() {
        let tracker = PlaybackTracker::new();
        tracker.start(WatchSession::new(ServiceKind::AniList, 1, 5, "A".into(), None, None));
        tracker.start(WatchSession::new(ServiceKind::AniList, 1, 6, "A".into(), None, None));
        tracker.stop_up_to(ServiceKind::AniList, 1, 5);
        let remaining = tracker.list();
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0].episode, 6);
    }

    #[test]
    fn stale_sessions_are_pruned_without_firing() {
        let tracker = PlaybackTracker::new();
        tracker.start(backdated(session(Some(24)), MAX_SESSION_AGE + Duration::from_secs(1)));
        assert!(tracker.tick().is_empty());
        assert!(tracker.list().is_empty());
    }
}
