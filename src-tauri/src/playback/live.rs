//! Playback detection (M6c): for a self-launched VLC or mpv session, replace
//! the flat-delay wall-clock guess with the player's own reported position.
//!
//! AniTrax normally hands a file to the OS's default-app opener and has no
//! idea what happens inside the player process. Here we launch the player
//! directly instead, with flags that expose its real playback position with
//! nothing to pre-configure — no settings dialog to open inside the player,
//! no config file to edit. A background task then polls it every few seconds.
//!
//! This fixes 6a/6b's shared blind spot for free: pausing or seeking away
//! just means the reported position stops advancing, so there's nothing
//! extra to detect. It also means closing the player before the episode
//! actually finishes correctly results in *no* bump at all, rather than the
//! wall-clock heuristic's "walked away and it still counts."
//!
//! Scoped to VLC and mpv — both can be told at launch time (via command-line
//! flags) to expose their position, with nothing for the user to set up.
//! MPC-HC/BE has no CLI equivalent for its web interface, so it stays on the
//! wall-clock heuristic; an already-running player (6b's case, opened outside
//! AniTrax) can't be handed launch flags after the fact either.

use std::path::{Path, PathBuf};
use std::time::Duration;

use rand::distributions::Alphanumeric;
use rand::Rng;
use serde::Deserialize;
use tauri::AppHandle;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

use crate::error::{AppError, AppResult};
use crate::state::AppState;

use super::WatchSession;

/// Fraction of the episode's reported length at which we consider it
/// "watched" — leaves room for credits/a cold open without waiting for
/// literal 100%, which some releases never quite reach anyway.
const COMPLETE_FRACTION: f64 = 0.90;
/// How often to ask the player where it is.
const POLL_EVERY: Duration = Duration::from_secs(10);
/// How long to keep retrying the initial connection to mpv's IPC pipe/socket
/// after launch — it needs a moment to come up.
const MPV_CONNECT_TIMEOUT: Duration = Duration::from_secs(8);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerKind {
    Vlc,
    Mpv,
}

impl PlayerKind {
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "vlc" => Some(Self::Vlc),
            "mpv" => Some(Self::Mpv),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Vlc => "vlc",
            Self::Mpv => "mpv",
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
        PlayerKind::Mpv => spawn_mpv(app, state, exe, file, session),
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

/// A fresh, unique IPC address for one mpv launch — a named pipe on Windows,
/// a unix socket path elsewhere. Never reused, so two AniTrax-launched mpv
/// instances can't collide.
fn mpv_ipc_address() -> String {
    let suffix = random_token(12);
    if cfg!(windows) {
        format!(r"\\.\pipe\anitrax-mpv-{suffix}")
    } else {
        std::env::temp_dir()
            .join(format!("anitrax-mpv-{suffix}.sock"))
            .to_string_lossy()
            .into_owned()
    }
}

fn spawn_mpv(
    app: AppHandle,
    state: AppState,
    exe: &Path,
    file: &Path,
    session: WatchSession,
) -> AppResult<()> {
    let address = mpv_ipc_address();

    let child = tokio::process::Command::new(exe)
        .arg(file)
        .arg(format!("--input-ipc-server={address}"))
        .spawn()
        .map_err(|e| AppError::other(format!("couldn't launch mpv: {e}")))?;

    state.playback.start(session.clone());
    tauri::async_runtime::spawn(poll_mpv(app, state, child, address, session));
    Ok(())
}

/// One line of mpv's JSON IPC protocol — either a reply to a request we sent
/// (`data`/`error` present) or an unsolicited event (`event` present, which
/// we just skip since we never `observe_property`).
#[derive(Debug, Deserialize)]
struct MpvLine {
    #[serde(default)]
    data: Option<serde_json::Value>,
    #[serde(default)]
    event: Option<String>,
}

/// Ask mpv for one numeric property over its IPC pipe: write the request,
/// then read lines until a reply (as opposed to an unsolicited event) comes
/// back. There's at most one outstanding request on this connection at a
/// time, so no need to match on `request_id`. `None` on a closed pipe, a
/// malformed line, or a property mpv doesn't have a number for yet.
async fn mpv_query_number<R, W>(reader: &mut R, writer: &mut W, property: &str) -> Option<f64>
where
    R: AsyncBufReadExt + Unpin,
    W: AsyncWriteExt + Unpin,
{
    let request = serde_json::json!({ "command": ["get_property", property] });
    writer.write_all(format!("{request}\n").as_bytes()).await.ok()?;

    loop {
        let mut line = String::new();
        let n = reader.read_line(&mut line).await.ok()?;
        if n == 0 {
            return None; // EOF — mpv closed the pipe (process exiting)
        }
        let Ok(reply) = serde_json::from_str::<MpvLine>(&line) else {
            continue;
        };
        if reply.event.is_some() {
            continue;
        }
        return reply.data.and_then(|v| v.as_f64());
    }
}

#[cfg(windows)]
async fn connect_mpv(address: &str) -> std::io::Result<tokio::net::windows::named_pipe::NamedPipeClient> {
    use tokio::net::windows::named_pipe::ClientOptions;
    let deadline = tokio::time::Instant::now() + MPV_CONNECT_TIMEOUT;
    loop {
        match ClientOptions::new().open(address) {
            Ok(client) => return Ok(client),
            // ERROR_FILE_NOT_FOUND / ERROR_PIPE_BUSY — mpv hasn't created the
            // pipe yet (or is briefly busy accepting another connection).
            Err(e)
                if matches!(e.raw_os_error(), Some(2) | Some(231)) && tokio::time::Instant::now() < deadline =>
            {
                tokio::time::sleep(Duration::from_millis(200)).await;
            }
            Err(e) => return Err(e),
        }
    }
}

#[cfg(not(windows))]
async fn connect_mpv(address: &str) -> std::io::Result<tokio::net::UnixStream> {
    let deadline = tokio::time::Instant::now() + MPV_CONNECT_TIMEOUT;
    loop {
        match tokio::net::UnixStream::connect(address).await {
            Ok(client) => return Ok(client),
            Err(e) if tokio::time::Instant::now() < deadline => {
                tokio::time::sleep(Duration::from_millis(200)).await;
            }
            Err(e) => return Err(e),
        }
    }
}

async fn poll_mpv(
    app: AppHandle,
    state: AppState,
    mut child: tokio::process::Child,
    address: String,
    session: WatchSession,
) {
    let key = (session.service, session.media_id, session.episode);

    let client = tokio::select! {
        c = connect_mpv(&address) => c,
        status = child.wait() => {
            tracing::debug!(?status, "mpv exited before its IPC pipe came up");
            state.playback.remove(key.0, key.1, key.2);
            return;
        }
    };
    let client = match client {
        Ok(c) => c,
        Err(e) => {
            tracing::warn!(?e, "couldn't connect to mpv's IPC pipe, falling back to no tracking");
            state.playback.remove(key.0, key.1, key.2);
            return;
        }
    };
    let (read_half, mut writer) = tokio::io::split(client);
    let mut reader = BufReader::new(read_half);

    loop {
        tokio::select! {
            _ = tokio::time::sleep(POLL_EVERY) => {}
            status = child.wait() => {
                tracing::debug!(?status, episode = session.episode, "mpv closed before finishing — not counted as watched");
                state.playback.remove(key.0, key.1, key.2);
                return;
            }
        }

        let duration = mpv_query_number(&mut reader, &mut writer, "duration").await;
        let position = mpv_query_number(&mut reader, &mut writer, "time-pos").await;
        let (Some(duration), Some(position)) = (duration, position) else {
            // A closed pipe here just means the next loop iteration's
            // `child.wait()` will catch the exit and clean up; nothing to do.
            tracing::debug!("mpv didn't answer a position query, will retry");
            continue;
        };

        state
            .playback
            .update_live(key.0, key.1, key.2, position.max(0.0) as u64, duration.max(0.0) as u64);

        if duration > 0.0 && position / duration >= COMPLETE_FRACTION {
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
