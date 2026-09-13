//! Foreground-window playback detection (M6b): read the title of whichever
//! player window is focused, so a show played *without* AniTrax's own Play
//! button — a different file location, streamed, whatever — still gets
//! noticed. Feeds into the exact same tracker/threshold pipeline 6a built
//! (see [`crate::sync::prepare_watch_session`]).
//!
//! Cross-platform via `active-win-pos-rs` (Win32 on Windows, Accessibility on
//! macOS, X11 on Linux) — one dependency covers all three targets named in
//! the milestone doc.

use std::path::Path;

/// (process exe filename, friendly label) for the players we know how to
/// recognise and (imperfectly) strip chrome from. Not exhaustive — anitomy
/// tolerates a player's own title decoration reasonably well even unstripped.
pub const KNOWN_PLAYERS: &[(&str, &str)] = &[
    ("vlc.exe", "VLC"),
    ("mpc-hc64.exe", "MPC-HC"),
    ("mpc-hc.exe", "MPC-HC (32-bit)"),
    ("mpc-be64.exe", "MPC-BE"),
    ("mpc-be.exe", "MPC-BE (32-bit)"),
    ("mpv.exe", "mpv"),
    ("potplayermini64.exe", "PotPlayer"),
    ("potplayermini.exe", "PotPlayer (32-bit)"),
    ("wmplayer.exe", "Windows Media Player"),
    ("smplayer.exe", "SMPlayer"),
];

pub struct ForegroundWindow {
    /// Lowercased exe filename (e.g. `"vlc.exe"`), no path.
    pub process: String,
    pub title: String,
}

/// The focused window's process and title, or `None` if it can't be read
/// (unsupported platform, permissions, nothing focused, empty title).
pub fn foreground_window() -> Option<ForegroundWindow> {
    let win = active_win_pos_rs::get_active_window().ok()?;
    if win.title.trim().is_empty() {
        return None;
    }
    let process = Path::new(&win.process_path)
        .file_name()?
        .to_string_lossy()
        .to_lowercase();
    Some(ForegroundWindow {
        process,
        title: win.title,
    })
}

/// Strip a known player's own trailing title chrome ("<file> - VLC media
/// player") before handing the rest to anitomy.
pub fn strip_player_chrome(title: &str, process: &str) -> String {
    let markers: &[&str] = match process {
        "vlc.exe" => &["vlc media player"],
        "mpc-hc64.exe" | "mpc-hc.exe" => &["media player classic - home cinema", "mpc-hc"],
        "mpc-be64.exe" | "mpc-be.exe" => &["media player classic - be", "mpc-be"],
        "potplayermini64.exe" | "potplayermini.exe" => &["potplayer"],
        "wmplayer.exe" => &["windows media player"],
        "smplayer.exe" => &["smplayer"],
        _ => &[],
    };
    let lower = title.to_lowercase();
    let mut end = title.len();
    for m in markers {
        if let Some(idx) = lower.rfind(m) {
            if idx > 0 && idx < end {
                end = idx;
            }
        }
    }
    title[..end]
        .trim()
        .trim_end_matches(['-', '–', '—'])
        .trim()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_known_player_chrome() {
        assert_eq!(
            strip_player_chrome("Sousou no Frieren - 05.mkv - VLC media player", "vlc.exe"),
            "Sousou no Frieren - 05.mkv"
        );
        assert_eq!(
            strip_player_chrome(
                "Sousou no Frieren - 05.mkv - Media Player Classic - Home Cinema",
                "mpc-hc64.exe"
            ),
            "Sousou no Frieren - 05.mkv"
        );
    }

    #[test]
    fn leaves_unrecognised_players_untouched() {
        assert_eq!(
            strip_player_chrome("Sousou no Frieren - 05.mkv", "mpv.exe"),
            "Sousou no Frieren - 05.mkv"
        );
    }

    #[test]
    fn never_empties_a_title_that_is_just_chrome() {
        assert_eq!(strip_player_chrome("VLC media player", "vlc.exe"), "VLC media player");
    }
}
