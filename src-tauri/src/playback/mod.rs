//! Playback detection (M6a): "you hit Play, so we already know the show,
//! episode and file — a fixed amount of time after we notice it, offer to
//! bump progress." M6b adds any-player window-title detection on top of the
//! same tracker. M6c (`live`) adds a third way to fill in a `WatchSession`,
//! for self-launched VLC: a dedicated poller asks the player its real
//! position instead of guessing from wall-clock time.

pub mod detect;
pub mod live;

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

/// If a fired session is never resolved (no progress edit — the prompt was
/// dismissed, ignored, or auto-closed), ask again after this much longer
/// instead of nagging on the same short interval. Matters most for M6b: a
/// still-playing episode gets re-detected every poll, and without this a
/// dismissed prompt would just come right back in another `CONFIRM_AFTER`.
pub const RENOTIFY_AFTER: Duration = Duration::from_secs(12 * 60);

/// How a `WatchSession` decides "the episode is probably done."
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProgressSource {
    /// 6a/6b: a fixed delay since the session was started/first detected.
    /// `tick()` owns firing for these.
    WallClock,
    /// 6c: a dedicated poller (`playback::live`) owns firing and cleanup for
    /// this session entirely — `tick()` only prunes it by `MAX_SESSION_AGE`,
    /// it never fires it via the wall clock.
    Live,
}

/// Real position/duration as last reported by a 6c live poller, for the "Now
/// watching" strip. `None` fields for a `WallClock` session.
#[derive(Debug, Clone, Copy, Default)]
pub struct LivePosition {
    pub position_secs: u64,
    pub duration_secs: u64,
}

#[derive(Debug, Clone)]
pub struct WatchSession {
    pub service: ServiceKind,
    pub media_id: i64,
    pub episode: i64,
    pub title: String,
    pub episodes_total: Option<i64>,
    pub started_at: Instant,
    /// Elapsed time at which we consider the episode "probably watched" (or,
    /// after the first fire, "worth asking about again"). Meaningless for a
    /// `Live` session — `tick()` never reads it for one.
    pub threshold: Duration,
    pub source: ProgressSource,
    /// `Some` only for a `Live` session; updated by its poller on every poll.
    pub live: Option<Arc<Mutex<LivePosition>>>,
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
            source: ProgressSource::WallClock,
            live: None,
        }
    }

    /// A copy of this session tagged for 6c live-position tracking instead of
    /// the wall-clock heuristic. Takes `&self` rather than consuming — the
    /// caller may still need the original if launching the live player fails
    /// and it falls back to the OS opener + wall-clock tracking.
    pub fn as_live(&self) -> Self {
        Self {
            source: ProgressSource::Live,
            live: Some(Arc::new(Mutex::new(LivePosition::default()))),
            ..self.clone()
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
    /// Real player position/duration for a `Live` session; `None` for a
    /// `WallClock` one (or before its first poll has reported in).
    pub position_secs: Option<u64>,
    pub duration_secs: Option<u64>,
}

fn view_of(s: &WatchSession) -> WatchSessionView {
    let live = s.live.as_ref().map(|l| *l.lock().unwrap());
    WatchSessionView {
        service: s.service.as_str().into(),
        media_id: s.media_id,
        episode: s.episode,
        title: s.title.clone(),
        episodes_total: s.episodes_total,
        elapsed_secs: s.started_at.elapsed().as_secs(),
        threshold_secs: s.threshold.as_secs(),
        position_secs: live.map(|l| l.position_secs),
        duration_secs: live.map(|l| l.duration_secs),
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

    /// Drop one exact session outright — used by a 6c live poller when the
    /// player closes before the episode actually finished, so it doesn't
    /// linger as "now watching" (and never fires a prompt for it).
    pub fn remove(&self, service: ServiceKind, media_id: i64, episode: i64) {
        self.sessions.lock().unwrap().remove(&(service, media_id, episode));
    }

    /// Record a 6c poller's latest read of a session's real position, for the
    /// "Now watching" strip. A no-op if the session isn't tracked (already
    /// resolved/removed) or isn't `Live`.
    pub fn update_live(
        &self,
        service: ServiceKind,
        media_id: i64,
        episode: i64,
        position_secs: u64,
        duration_secs: u64,
    ) {
        if let Some(s) = self.sessions.lock().unwrap().get(&(service, media_id, episode)) {
            if let Some(live) = &s.live {
                *live.lock().unwrap() = LivePosition { position_secs, duration_secs };
            }
        }
    }

    /// For the "Now watching" strip.
    pub fn list(&self) -> Vec<WatchSessionView> {
        self.sessions.lock().unwrap().values().map(view_of).collect()
    }

    /// Sessions that just crossed their threshold since the last tick, plus
    /// housekeeping: prune stale sessions. A session that fires is **not**
    /// removed — it's re-armed with `RENOTIFY_AFTER` so an unresolved prompt
    /// (dismissed, ignored, or just re-detected by M6b while still playing)
    /// comes back on a longer, less naggy interval instead of immediately.
    /// Only a progress edit (`stop_up_to`) or old age actually clears one.
    /// `Live` sessions are never fired from here — their own poller (6c)
    /// owns that — this just applies the same `MAX_SESSION_AGE` safety net to
    /// them in case a poller task dies without cleaning up after itself.
    pub fn tick(&self) -> Vec<WatchSessionView> {
        let mut sessions = self.sessions.lock().unwrap();
        sessions.retain(|_, s| s.started_at.elapsed() < MAX_SESSION_AGE);
        let mut ready = Vec::new();
        for s in sessions.values_mut() {
            if s.source == ProgressSource::Live {
                continue;
            }
            if s.started_at.elapsed() >= s.threshold {
                ready.push(view_of(s));
                s.started_at = Instant::now();
                s.threshold = RENOTIFY_AFTER;
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
    fn firing_re_arms_with_the_longer_renotify_interval_instead_of_clearing() {
        let tracker = PlaybackTracker::new();
        tracker.start(backdated(session(), CONFIRM_AFTER + Duration::from_secs(1)));

        let first = tracker.tick();
        assert_eq!(first.len(), 1);
        assert_eq!(first[0].episode, 5);

        // Not resolved — still tracked, but on the longer follow-up interval,
        // not immediately re-sent.
        assert!(tracker.tick().is_empty());
        let remaining = tracker.list();
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0].threshold_secs, RENOTIFY_AFTER.as_secs());
    }

    #[test]
    fn unresolved_session_fires_again_after_the_renotify_interval() {
        let tracker = PlaybackTracker::new();
        tracker.start(backdated(session(), CONFIRM_AFTER + Duration::from_secs(1)));
        assert_eq!(tracker.tick().len(), 1);

        // Still short of the 12-minute follow-up — quiet.
        assert!(tracker.tick().is_empty());

        // Fast-forward past it: fires again.
        {
            let mut sessions = tracker.sessions.lock().unwrap();
            for s in sessions.values_mut() {
                s.started_at = Instant::now() - RENOTIFY_AFTER - Duration::from_secs(1);
            }
        }
        assert_eq!(tracker.tick().len(), 1);
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

    #[test]
    fn tick_never_fires_a_live_session_even_past_threshold() {
        let tracker = PlaybackTracker::new();
        tracker.start(backdated(session().as_live(), CONFIRM_AFTER + Duration::from_secs(1)));
        assert!(tracker.tick().is_empty());
        assert_eq!(tracker.list().len(), 1);
    }

    #[test]
    fn live_session_still_expires_at_max_age_as_a_safety_net() {
        let tracker = PlaybackTracker::new();
        tracker.start(backdated(session().as_live(), MAX_SESSION_AGE + Duration::from_secs(1)));
        assert!(tracker.tick().is_empty());
        assert!(tracker.list().is_empty());
    }

    #[test]
    fn update_live_is_reflected_in_the_view() {
        let tracker = PlaybackTracker::new();
        tracker.start(session().as_live());
        tracker.update_live(ServiceKind::AniList, 1, 5, 300, 1400);
        let view = &tracker.list()[0];
        assert_eq!(view.position_secs, Some(300));
        assert_eq!(view.duration_secs, Some(1400));
    }

    #[test]
    fn remove_drops_the_exact_session() {
        let tracker = PlaybackTracker::new();
        tracker.start(session().as_live());
        tracker.remove(ServiceKind::AniList, 1, 5);
        assert!(tracker.list().is_empty());
    }
}
