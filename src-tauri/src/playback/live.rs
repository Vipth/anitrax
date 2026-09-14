//! Playback detection (M6c): for a self-launched VLC session, replace the
//! flat-delay wall-clock guess with the player's own reported position.
//!
//! AniTrax normally hands a file to the OS's default-app opener and has no
//! idea what happens inside the player process. Here we launch VLC directly
//! instead, passing its built-in HTTP interface a random per-session
//! port + password so nothing needs pre-configuring — no settings dialog to
//! open inside VLC itself, no config file to edit. A background task then
//! polls that interface every few seconds for the real playback position.
//!
//! This fixes 6a/6b's shared blind spot for free: pausing or seeking away
//! just means the reported position stops advancing, so there's nothing
//! extra to detect. It also means closing the player before the episode
//! actually finishes correctly results in *no* bump at all, rather than the
//! wall-clock heuristic's "walked away and it still counts."
//!
//! Scoped to VLC only for now — it's the most commonly used cross-platform
//! player and its HTTP interface can be fully configured via command-line
//! flags at launch time, so there's nothing for a user to set up. mpv would
//! need its own IPC transport (a named pipe on Windows, a socket elsewhere)
//! and is a reasonable follow-up, not included here.

use std::path::{Path, PathBuf};
use std::time::Duration;

use rand::distributions::Alphanumeric;
use rand::Rng;
use serde::Deserialize;
use tauri::AppHandle;

use crate::error::{AppError, AppResult};
use crate::state::AppState;

use super::WatchSession;

/// Fraction of the episode's reported length at which we consider it
/// "watched" — leaves room for credits/a cold open without waiting for
/// literal 100%, which some releases never quite reach anyway.
const COMPLETE_FRACTION: f64 = 0.90;
/// How often to ask the player where it is.
const POLL_EVERY: Duration = Duration::from_secs(10);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerKind {
    Vlc,
}

impl PlayerKind {
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "vlc" => Some(Self::Vlc),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Vlc => "vlc",
        }
    }
}

/// Launch `exe` directly on `file` (bypassing the OS opener) with flags that
/// expose real playback position, and spawn a background poller that fires
/// the usual confirm/silent playback flow the moment the episode crosses
/// `COMPLETE_FRACTION` — or drops the session, unfired, if the player closes
/// first. `session` should already be tagged `ProgressSource::Live`
/// ([`WatchSession::as_live`]).
pub fn spawn_and_track(
    app: AppHandle,
    state: AppState,
    kind: PlayerKind,
    exe: &Path,
    file: &Path,
    session: WatchSession,
) -> AppResult<()> {
    match kind {
        PlayerKind::Vlc => spawn_vlc(app, state, exe, file, session),
    }
}

fn random_token(len: usize) -> String {
    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(len)
        .map(char::from)
        .collect()
}

fn spawn_vlc(
    app: AppHandle,
    state: AppState,
    exe: &Path,
    file: &Path,
    session: WatchSession,
) -> AppResult<()> {
    let port: u16 = rand::thread_rng().gen_range(23_000..24_000);
    let token = random_token(24);

    let child = tokio::process::Command::new(exe)
        .arg(file)
        .arg("--extraintf")
        .arg("http")
        .arg("--http-host")
        .arg("127.0.0.1")
        .arg("--http-port")
        .arg(port.to_string())
        .arg("--http-password")
        .arg(&token)
        .spawn()
        .map_err(|e| AppError::other(format!("couldn't launch VLC: {e}")))?;

    state.playback.start(session.clone());
    tauri::async_runtime::spawn(poll_vlc(app, state, child, port, token, session));
    Ok(())
}

#[derive(Debug, Deserialize)]
struct VlcStatus {
    time: Option<u64>,
    length: Option<u64>,
}

async fn poll_vlc(
    app: AppHandle,
    state: AppState,
    mut child: tokio::process::Child,
    port: u16,
    token: String,
    session: WatchSession,
) {
    let client = reqwest::Client::new();
    let url = format!("http://127.0.0.1:{port}/requests/status.json");
    let key = (session.service, session.media_id, session.episode);

    loop {
        tokio::select! {
            _ = tokio::time::sleep(POLL_EVERY) => {}
            status = child.wait() => {
                tracing::debug!(?status, episode = session.episode, "vlc closed before finishing — not counted as watched");
                state.playback.remove(key.0, key.1, key.2);
                return;
            }
        }

        let resp = client
            .get(&url)
            .basic_auth("", Some(&token))
            .send()
            .await
            .and_then(|r| r.error_for_status());
        let status: VlcStatus = match resp {
            Ok(r) => match r.json().await {
                Ok(s) => s,
                Err(e) => {
                    tracing::debug!(?e, "vlc status response wasn't the expected shape");
                    continue;
                }
            },
            Err(e) => {
                // VLC's HTTP interface can take a beat to come up after
                // launch, or briefly hiccup — only treat an actual process
                // exit (caught above) as "the episode is done".
                tracing::debug!(?e, "vlc status poll failed, will retry");
                continue;
            }
        };

        let (position, duration) = (status.time.unwrap_or(0), status.length.unwrap_or(0));
        state.playback.update_live(key.0, key.1, key.2, position, duration);

        if duration > 0 && position as f64 / duration as f64 >= COMPLETE_FRACTION {
            crate::sync::fire_ready(
                &app,
                &state,
                session.service.as_str(),
                session.media_id,
                session.episode,
                &session.title,
                session.episodes_total,
            )
            .await;
            return;
        }
    }
}

/// The user's configured "launch directly + track real position" player
/// (see `sync::player_integration`, which reads this out of settings).
pub struct PlayerIntegration {
    pub kind: PlayerKind,
    pub exe: PathBuf,
}
