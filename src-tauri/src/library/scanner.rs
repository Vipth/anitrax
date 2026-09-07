//! Walk watched folders and turn each video file into a [`ScannedFile`] via
//! anitomy. Pure filesystem + string work — no network, no database.

use std::path::{Path, PathBuf};

use anitomy::{ElementKind, OwnedElementObject};
use walkdir::WalkDir;

/// Extensions we treat as watchable episodes. Lowercase, no dot.
const VIDEO_EXTS: &[&str] = &[
    "mkv", "mp4", "avi", "mov", "wmv", "flv", "webm", "m4v", "mpg", "mpeg", "ts", "m2ts", "ogm",
];

/// The minimum size (bytes) for a file to count as a real episode — filters out
/// samples and stray clips.
const MIN_SIZE: u64 = 20 * 1024 * 1024;

#[derive(Debug, Clone)]
pub struct ScannedFile {
    pub path: PathBuf,
    pub file_name: String,
    pub size_bytes: Option<i64>,
    pub modified_at: Option<String>,
    pub title: Option<String>,
    /// Title guessed from the containing folder(s) — usually the show-level name
    /// when the file itself only says something generic.
    pub folder_title: Option<String>,
    pub episode: Option<i64>,
    pub season: Option<i64>,
    pub year: Option<i64>,
    pub resolution: Option<String>,
    pub release_group: Option<String>,
}

pub fn is_video(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| VIDEO_EXTS.contains(&e.to_ascii_lowercase().as_str()))
        .unwrap_or(false)
}

/// Every video file under `root`, recursively. Missing/unreadable roots yield an
/// empty list rather than an error — a folder can be on a drive that's offline.
pub fn walk(root: &Path) -> Vec<PathBuf> {
    WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_file())
        .map(|e| e.into_path())
        .filter(|p| is_video(p))
        .collect()
}

/// Parse one already-known-to-exist video file. `root` is the watched folder it
/// was found under, used to read title/season hints from the folder chain.
/// `None` if it's below [`MIN_SIZE`] or its metadata can't be read.
pub fn scan_file(path: &Path, root: &Path) -> Option<ScannedFile> {
    let meta = std::fs::metadata(path).ok()?;
    let size = meta.len();
    if size < MIN_SIZE {
        return None;
    }
    let file_name = path.file_name()?.to_string_lossy().into_owned();
    let modified_at = meta
        .modified()
        .ok()
        .map(chrono::DateTime::<chrono::Utc>::from)
        .map(|t| t.to_rfc3339());

    let parsed = parse_name(&file_name);
    let folder = folder_context(path, root);
    Some(ScannedFile {
        path: path.to_path_buf(),
        file_name,
        size_bytes: Some(size as i64),
        modified_at,
        title: parsed.title,
        folder_title: folder.title,
        episode: parsed.episode,
        // A season in the file name wins; otherwise take the folder's ("Season 3").
        season: parsed.season.or(folder.season),
        year: parsed.year,
        resolution: parsed.resolution,
        release_group: parsed.release_group,
    })
}

#[derive(Debug, Default, PartialEq)]
pub struct FolderContext {
    pub title: Option<String>,
    pub season: Option<i64>,
}

/// Read hints from the directories between `root` (exclusive) and the file. The
/// deepest folder that isn't a "Season 2" / "Specials" style bucket is taken as
/// the show title; a season number from any bucket folder is picked up too.
pub fn folder_context(file: &Path, root: &Path) -> FolderContext {
    let rel = match file.strip_prefix(root) {
        Ok(r) => r,
        Err(_) => return FolderContext::default(),
    };
    // Components above the file itself, deepest first.
    let mut dirs: Vec<String> = rel
        .parent()
        .into_iter()
        .flat_map(|p| p.components())
        .filter_map(|c| match c {
            std::path::Component::Normal(s) => Some(s.to_string_lossy().into_owned()),
            _ => None,
        })
        .collect();
    dirs.reverse();

    let mut ctx = FolderContext::default();
    for dir in &dirs {
        if let Some(n) = bucket_season(dir) {
            ctx.season.get_or_insert(n);
            continue;
        }
        if is_generic_folder(dir) {
            continue;
        }
        if ctx.title.is_none() {
            let parsed = parse_name(dir);
            ctx.title = parsed.title.filter(|s| !s.is_empty());
            if ctx.season.is_none() {
                ctx.season = parsed.season;
            }
        }
    }
    ctx
}

/// `"Season 3"`, `"S3"`, `"Series 2"`, `"Cour 2"`, `"Part 2"` -> the number.
fn bucket_season(name: &str) -> Option<i64> {
    let l = name.trim().to_lowercase();
    for prefix in ["season ", "series ", "cour ", "part ", "s"] {
        if let Some(rest) = l.strip_prefix(prefix) {
            let rest = rest.trim();
            if !rest.is_empty() && rest.bytes().all(|b| b.is_ascii_digit()) {
                return rest.parse().ok();
            }
        }
    }
    None
}

fn is_generic_folder(name: &str) -> bool {
    let l = name.trim().to_lowercase();
    matches!(
        l.as_str(),
        "specials"
            | "special"
            | "extras"
            | "extra"
            | "ova"
            | "ovas"
            | "oad"
            | "ncop"
            | "nced"
            | "nc"
            | "bd"
            | "bdrip"
            | "bluray"
            | "subs"
            | "subtitles"
            | "sp"
            | "movies"
            | "movie"
            | "anime"
            | "downloads"
            | "complete"
            | "batch"
    )
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ParsedName {
    pub title: Option<String>,
    pub episode: Option<i64>,
    pub season: Option<i64>,
    pub year: Option<i64>,
    pub resolution: Option<String>,
    pub release_group: Option<String>,
}

/// Run anitomy over a file name (or a bare release title) and pull out the bits
/// the matcher and the UI care about.
pub fn parse_name(name: &str) -> ParsedName {
    let elements = anitomy::parse(name);

    // A batch/range release ("01-12", "01~24") shows up as more than one
    // Episode element — there's no single episode we can track, so drop it.
    let episode_count = elements
        .iter()
        .filter(|e| e.kind() == ElementKind::Episode)
        .count();

    let obj: OwnedElementObject = elements
        .iter()
        .filter(|e| e.kind() != ElementKind::FileExtension)
        .collect();

    ParsedName {
        title: obj.title.map(|t| clean(&t)).filter(|s| !s.is_empty()),
        episode: if episode_count == 1 {
            obj.episode.as_deref().and_then(first_int)
        } else {
            None
        },
        season: obj.season.as_deref().and_then(first_int),
        year: obj
            .year
            .as_deref()
            .and_then(first_int)
            .filter(|y| (1950..=2100).contains(y)),
        resolution: obj.video_resolution.map(|r| normalize_resolution(&r)),
        release_group: obj.release_group.filter(|s| !s.is_empty()),
    }
}

/// anitomy hands back the raw token; a range like `01-12` or a decimal `13.5`
/// isn't a single episode we can track, so only accept a clean integer.
fn first_int(raw: &str) -> Option<i64> {
    let t = raw.trim();
    if t.is_empty() || !t.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    t.parse().ok()
}

fn clean(title: &str) -> String {
    title
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .trim_matches(|c: char| c == '-' || c == '_' || c == '.' || c.is_whitespace())
        .to_string()
}

/// `1920x1080` / `1080` / `1080P` -> `1080p`; anything else passes through.
fn normalize_resolution(raw: &str) -> String {
    let lower = raw.to_ascii_lowercase();
    if let Some((_, h)) = lower.split_once('x') {
        return format!("{}p", h.trim());
    }
    if let Some(h) = lower.strip_suffix('p') {
        return format!("{}p", h.trim());
    }
    if lower.bytes().all(|b| b.is_ascii_digit()) {
        return format!("{lower}p");
    }
    lower
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(name: &str) -> ParsedName {
        parse_name(name)
    }

    #[test]
    fn typical_fansub_release() {
        let r = p("[SubsPlease] Frieren - 12 (1080p) [9C63B2A4].mkv");
        assert_eq!(r.title.as_deref(), Some("Frieren"));
        assert_eq!(r.episode, Some(12));
        assert_eq!(r.resolution.as_deref(), Some("1080p"));
        assert_eq!(r.release_group.as_deref(), Some("SubsPlease"));
    }

    #[test]
    fn dotted_scene_release() {
        let r = p("Spy.x.Family.S02E05.1080p.WEB.H264-SENPAI.mkv");
        assert_eq!(r.episode, Some(5));
        assert_eq!(r.season, Some(2));
        assert!(r.title.as_deref().unwrap().to_lowercase().contains("spy"));
    }

    #[test]
    fn resolution_from_dimensions() {
        let r = p("[Taiga] Toradora! - 01 [1280x720].mkv");
        assert_eq!(r.resolution.as_deref(), Some("720p"));
        assert_eq!(r.episode, Some(1));
    }

    #[test]
    fn episode_range_is_not_a_single_episode() {
        let r = p("[Group] Some Show (01-12) [Batch].mkv");
        assert_eq!(r.episode, None);
    }

    #[test]
    fn movie_with_no_episode() {
        let r = p("[Group] A Silent Voice (2016) [BD 1080p].mkv");
        assert_eq!(r.episode, None);
        assert!(r.title.is_some());
    }

    #[test]
    fn folder_gives_title_and_season() {
        let root = Path::new("/anime");
        let file =
            Path::new("/anime/Sword Art Online/Season 3/[Group] SAO - 13 [1080p].mkv");
        let ctx = folder_context(file, root);
        assert_eq!(ctx.title.as_deref(), Some("Sword Art Online"));
        assert_eq!(ctx.season, Some(3));
    }

    #[test]
    fn folder_skips_generic_buckets() {
        let root = Path::new("/anime");
        let file = Path::new("/anime/Frieren/Specials/OVA 01.mkv");
        let ctx = folder_context(file, root);
        assert_eq!(ctx.title.as_deref(), Some("Frieren"));
    }

    #[test]
    fn file_directly_in_root_has_no_folder_title() {
        let root = Path::new("/anime");
        let file = Path::new("/anime/[Group] Bocchi the Rock - 01.mkv");
        assert_eq!(folder_context(file, root), FolderContext::default());
    }

    #[test]
    fn bare_s_prefix_is_not_a_season_folder() {
        assert_eq!(bucket_season("Steins;Gate"), None);
        assert_eq!(bucket_season("S2"), Some(2));
        assert_eq!(bucket_season("Season 04"), Some(4));
    }
}
