//! Pure rule evaluation: given a parsed feed item and a rule, decide whether to
//! download it. No I/O — the scheduler handles fetching, dedupe and the client
//! hand-off. Kept separate so it's cheap to unit-test against fixture titles.

use super::Rule;

/// The bits of a feed item's release title the rule cares about, produced by
/// running the title through the same anitomy parser the library scanner uses.
#[derive(Debug, Clone, Default)]
pub struct ParsedItem {
    pub title: String,
    pub episode: Option<i64>,
    /// anitomy-parsed season number, or `None` when the title carries no season
    /// marker (which we treat as season 1).
    pub season: Option<i64>,
    /// Vertical resolution in pixels (`1080` for "1080p"), when the title says.
    pub resolution_height: Option<i64>,
    pub release_group: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Decision {
    Download,
    /// Skipped, with a short reason for logs / the "why didn't this match" case.
    Skip(&'static str),
}

impl Decision {
    pub fn is_download(&self) -> bool {
        matches!(self, Decision::Download)
    }
}

/// `"1080p"` / `"1920x1080"` / `"1080"` -> `1080`.
pub fn resolution_height(raw: &str) -> Option<i64> {
    let lower = raw.trim().to_ascii_lowercase();
    let digits = lower
        .rsplit(|c: char| c == 'x' || c == '×')
        .next()
        .unwrap_or(&lower)
        .trim_end_matches('p')
        .trim();
    digits.parse().ok().filter(|h| *h > 0)
}

/// Does every whitespace-separated word of `needle` appear in `haystack`
/// (case-insensitive)? Empty / whitespace `needle` matches everything.
fn contains_all_words(haystack: &str, needle: &str) -> bool {
    let hay = haystack.to_lowercase();
    needle
        .split_whitespace()
        .all(|w| hay.contains(&w.to_lowercase()))
}

/// Does *any* whitespace-separated word of `needle` appear in `haystack`
/// (case-insensitive)? Empty `needle` matches nothing.
fn contains_any_word(haystack: &str, needle: &str) -> bool {
    let hay = haystack.to_lowercase();
    needle
        .split_whitespace()
        .any(|w| hay.contains(&w.to_lowercase()))
}

/// Evaluate one rule against one parsed item. Dedupe (`rss_history`) and the
/// feed/enabled checks on the *feed* are the scheduler's job; this covers the
/// rule's own constraints.
pub fn evaluate(rule: &Rule, item: &ParsedItem) -> Decision {
    if !rule.enabled {
        return Decision::Skip("rule disabled");
    }

    if let Some(needle) = rule.title_contains.as_deref() {
        if !needle.trim().is_empty() && !contains_all_words(&item.title, needle) {
            return Decision::Skip("title doesn't contain the required words");
        }
    }

    if let Some(needle) = rule.exclude_contains.as_deref() {
        if !needle.trim().is_empty() && contains_any_word(&item.title, needle) {
            return Decision::Skip("title contains an excluded word");
        }
    }

    if let Some(want) = rule.season {
        // An untagged release is season 1 by convention.
        let got = item.season.unwrap_or(1);
        if got != want {
            return Decision::Skip("wrong season");
        }
    }

    if let Some(group) = rule.release_group.as_deref() {
        if !group.trim().is_empty() {
            match item.release_group.as_deref() {
                Some(g) if g.eq_ignore_ascii_case(group.trim()) => {}
                _ => return Decision::Skip("release group doesn't match"),
            }
        }
    }

    if let Some(min) = rule.min_resolution {
        match item.resolution_height {
            Some(h) if h >= min => {}
            Some(_) => return Decision::Skip("resolution below the minimum"),
            None => return Decision::Skip("release doesn't state a resolution"),
        }
    }

    let has_range = rule.episode_from.is_some() || rule.episode_to.is_some();
    if has_range {
        let Some(ep) = item.episode else {
            return Decision::Skip("no single episode number (batch or unparsed)");
        };
        if let Some(from) = rule.episode_from {
            if ep < from {
                return Decision::Skip("episode before the range");
            }
        }
        if let Some(to) = rule.episode_to {
            if ep > to {
                return Decision::Skip("episode after the range");
            }
        }
    }

    Decision::Download
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rule() -> Rule {
        Rule {
            id: 1,
            name: "test".into(),
            enabled: true,
            feed_id: None,
            service: Some("anilist".into()),
            media_id: Some(123),
            title_contains: None,
            exclude_contains: None,
            release_group: None,
            min_resolution: None,
            season: None,
            episode_from: None,
            episode_to: None,
            dest_path: None,
            category: None,
            paused: false,
            created_at: "2026-09-10T00:00:00Z".into(),
            media_title: None,
        }
    }

    fn item(title: &str, ep: Option<i64>) -> ParsedItem {
        ParsedItem {
            title: title.into(),
            episode: ep,
            ..Default::default()
        }
    }

    #[test]
    fn resolution_parsing() {
        assert_eq!(resolution_height("1080p"), Some(1080));
        assert_eq!(resolution_height("1920x1080"), Some(1080));
        assert_eq!(resolution_height("720"), Some(720));
        assert_eq!(resolution_height("BD"), None);
    }

    #[test]
    fn disabled_rule_never_downloads() {
        let mut r = rule();
        r.enabled = false;
        assert!(!evaluate(&r, &item("anything", Some(1))).is_download());
    }

    #[test]
    fn title_contains_needs_every_word() {
        let mut r = rule();
        r.title_contains = Some("Frieren Beyond".into());
        assert!(evaluate(&r, &item("[Grp] Frieren Beyond Journey's End - 05", Some(5))).is_download());
        assert_eq!(
            evaluate(&r, &item("[Grp] Frieren - 05", Some(5))),
            Decision::Skip("title doesn't contain the required words")
        );
    }

    #[test]
    fn release_group_is_case_insensitive_exact() {
        let mut r = rule();
        r.release_group = Some("SubsPlease".into());
        let mut it = item("Show - 01", Some(1));
        it.release_group = Some("subsplease".into());
        assert!(evaluate(&r, &it).is_download());
        it.release_group = Some("Erai-raws".into());
        assert!(!evaluate(&r, &it).is_download());
    }

    #[test]
    fn min_resolution_rejects_lower_and_unknown() {
        let mut r = rule();
        r.min_resolution = Some(1080);
        let mut it = item("Show - 01", Some(1));
        it.resolution_height = Some(1080);
        assert!(evaluate(&r, &it).is_download());
        it.resolution_height = Some(720);
        assert!(!evaluate(&r, &it).is_download());
        it.resolution_height = None;
        assert!(!evaluate(&r, &it).is_download());
    }

    #[test]
    fn episode_range_is_inclusive_and_needs_a_number() {
        let mut r = rule();
        r.episode_from = Some(5);
        r.episode_to = Some(12);
        assert!(evaluate(&r, &item("Show - 05", Some(5))).is_download());
        assert!(evaluate(&r, &item("Show - 12", Some(12))).is_download());
        assert!(!evaluate(&r, &item("Show - 04", Some(4))).is_download());
        assert!(!evaluate(&r, &item("Show - 13", Some(13))).is_download());
        assert!(!evaluate(&r, &item("Show Batch 01-24", None)).is_download());
    }

    #[test]
    fn exclude_contains_rejects_on_any_word() {
        let mut r = rule();
        r.exclude_contains = Some("Batch V2".into());
        assert!(evaluate(&r, &item("[Grp] Frieren - 10 (1080p)", Some(10))).is_download());
        assert!(!evaluate(&r, &item("[Grp] Frieren (01-28) [Batch]", None)).is_download());
        assert!(!evaluate(&r, &item("[Grp] Frieren - 10 (1080p) [V2]", Some(10))).is_download());
    }

    #[test]
    fn season_filter_treats_untagged_as_season_one() {
        let mut r = rule();
        r.season = Some(1);
        // Untagged S1 release — no season token.
        assert!(evaluate(
            &r,
            &ParsedItem {
                title: "[SubsPlease] Sousou no Frieren - 05 (1080p)".into(),
                episode: Some(5),
                season: None,
                ..Default::default()
            }
        )
        .is_download());
        // S2 release is rejected by a season-1 rule.
        assert_eq!(
            evaluate(
                &r,
                &ParsedItem {
                    title: "[SubsPlease] Sousou no Frieren S2 - 10 (1080p)".into(),
                    episode: Some(10),
                    season: Some(2),
                    ..Default::default()
                }
            ),
            Decision::Skip("wrong season")
        );
    }

    #[test]
    fn no_constraints_downloads_everything() {
        assert!(evaluate(&rule(), &item("literally anything", None)).is_download());
    }
}
