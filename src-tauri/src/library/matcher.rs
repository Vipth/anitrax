//! Match a parsed file title to a cached media row. Cache-only: the caller
//! passes in an index built from `media_cache`, and nothing here touches the
//! network. Files that don't clear [`AUTO_THRESHOLD`] are left for the user to
//! link by hand.

use crate::tracker::model::ServiceKind;

/// Minimum score for an automatic match. Below this the file goes to the review
/// queue.
pub const AUTO_THRESHOLD: f64 = 0.82;
/// If the top two candidates are within this margin the match is ambiguous and
/// we'd rather ask than guess wrong.
const AMBIGUITY_MARGIN: f64 = 0.06;

/// One row of the search index, prepared once per scan.
#[derive(Debug, Clone)]
pub struct IndexEntry {
    pub service: ServiceKind,
    pub media_id: i64,
    pub season_year: Option<i32>,
    /// Every known title/synonym, normalised via [`normalize`].
    pub titles: Vec<String>,
}

impl IndexEntry {
    pub fn new(
        service: ServiceKind,
        media_id: i64,
        season_year: Option<i32>,
        raw_titles: impl IntoIterator<Item = String>,
    ) -> Self {
        let mut titles: Vec<String> = raw_titles
            .into_iter()
            .map(|t| normalize(&t))
            .filter(|t| !t.is_empty())
            .collect();
        titles.sort();
        titles.dedup();
        Self {
            service,
            media_id,
            season_year,
            titles,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Match {
    pub service: ServiceKind,
    pub media_id: i64,
    pub score: f64,
}

/// Lowercase, strip punctuation, fold common season wording, collapse spaces.
pub fn normalize(raw: &str) -> String {
    let lower = raw.to_lowercase();
    let mut out = String::with_capacity(lower.len());
    let mut last_space = true;
    for ch in lower.chars() {
        if ch.is_alphanumeric() {
            out.push(ch);
            last_space = false;
        } else if !last_space {
            out.push(' ');
            last_space = true;
        }
    }
    let out = out.trim();

    // Fold ordinal / roman season markers so "Bleach 2nd Season", "Bleach II"
    // and "Bleach Season 2" all normalise near each other.
    let mut words: Vec<String> = out.split_whitespace().map(str::to_string).collect();
    fold_season_words(&mut words);
    words.join(" ")
}

fn fold_season_words(words: &mut Vec<String>) {
    const ORDINALS: &[(&str, &str)] = &[
        ("1st", "1"),
        ("2nd", "2"),
        ("3rd", "3"),
        ("4th", "4"),
        ("5th", "5"),
        ("first", "1"),
        ("second", "2"),
        ("third", "3"),
        ("fourth", "4"),
        ("fifth", "5"),
    ];
    const ROMAN: &[(&str, &str)] = &[
        ("ii", "2"),
        ("iii", "3"),
        ("iv", "4"),
        ("v", "5"),
        ("vi", "6"),
    ];

    for w in words.iter_mut() {
        if let Some((_, n)) = ORDINALS.iter().find(|(o, _)| *o == w) {
            *w = n.to_string();
        }
    }
    // Drop bare "season"/"cour"/"part" tokens; the number beside them carries it.
    words.retain(|w| !matches!(w.as_str(), "season" | "cour" | "part"));
    // Trailing roman numeral -> digit (only at the end, to avoid mangling titles).
    if let Some(last) = words.last_mut() {
        if let Some((_, n)) = ROMAN.iter().find(|(r, _)| *r == last) {
            *last = n.to_string();
        }
    }
}

/// Jaccard similarity over the word sets of two normalised strings.
fn token_overlap(a: &str, b: &str) -> f64 {
    let sa: std::collections::HashSet<&str> = a.split_whitespace().collect();
    let sb: std::collections::HashSet<&str> = b.split_whitespace().collect();
    if sa.is_empty() || sb.is_empty() {
        return 0.0;
    }
    let inter = sa.intersection(&sb).count() as f64;
    let union = sa.union(&sb).count() as f64;
    inter / union
}

/// Score one normalised file title against one index title. 1.0 = exact.
fn score_pair(file: &str, cand: &str) -> f64 {
    if file == cand {
        return 1.0;
    }
    let overlap = token_overlap(file, cand);
    // Containment (one title fully inside the other) is a strong signal — season
    // packs are often named just "Show" while the entry is "Show Season 2".
    let contains = if file.contains(cand) || cand.contains(file) {
        0.9
    } else {
        0.0
    };
    overlap.max(contains)
}

/// Best match for a parsed file, or `None` if nothing clears the bar / it's
/// ambiguous. `file_season` (when > 1) nudges toward titles that mention it.
pub fn best_match(
    file_title: &str,
    file_season: Option<i64>,
    file_year: Option<i32>,
    index: &[IndexEntry],
) -> Option<Match> {
    let norm = normalize(file_title);
    if norm.is_empty() {
        return None;
    }
    let season_hint = file_season.filter(|s| *s > 1).map(|s| s.to_string());

    let mut scored: Vec<(f64, &IndexEntry)> = index
        .iter()
        .map(|entry| {
            let mut best = entry
                .titles
                .iter()
                .map(|t| score_pair(&norm, t))
                .fold(0.0_f64, f64::max);

            // Season agreement: if the file says S2, favour an entry whose title
            // ends in "2"; gently penalise ones that look like season 1.
            if let Some(hint) = &season_hint {
                let mentions = entry.titles.iter().any(|t| {
                    t.split_whitespace().last() == Some(hint.as_str())
                });
                if mentions {
                    best += 0.08;
                } else if entry.titles.iter().all(|t| {
                    !t.chars().last().map(|c| c.is_ascii_digit()).unwrap_or(false)
                }) {
                    best -= 0.05;
                }
            }

            // Year agreement is a light tie-breaker.
            if let (Some(fy), Some(ey)) = (file_year, entry.season_year) {
                if fy == ey {
                    best += 0.04;
                } else if (fy - ey).abs() >= 2 {
                    best -= 0.03;
                }
            }

            (best.clamp(0.0, 1.0), entry)
        })
        .collect();

    scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

    let (top_score, top) = scored.first().copied()?;
    if top_score < AUTO_THRESHOLD {
        return None;
    }
    if let Some((second, _)) = scored.get(1) {
        if top_score - second < AMBIGUITY_MARGIN
            && scored[1].1.media_id != top.media_id
            && (*second) >= AUTO_THRESHOLD
        {
            return None; // too close to call
        }
    }
    Some(Match {
        service: top.service,
        media_id: top.media_id,
        score: top_score,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn idx(id: i64, year: Option<i32>, titles: &[&str]) -> IndexEntry {
        IndexEntry::new(
            ServiceKind::AniList,
            id,
            year,
            titles.iter().map(|s| s.to_string()),
        )
    }

    #[test]
    fn normalize_folds_season_wording() {
        assert_eq!(normalize("Bleach: Thousand-Year Blood War"), normalize("bleach thousand year blood war"));
        assert_eq!(normalize("Mushoku Tensei II"), "mushoku tensei 2");
        assert_eq!(normalize("Mushoku Tensei 2nd Season"), "mushoku tensei 2");
        assert_eq!(normalize("Made in Abyss Season 2"), "made in abyss 2");
    }

    #[test]
    fn exact_title_matches() {
        let index = vec![
            idx(1, Some(2023), &["Frieren: Beyond Journey's End", "Sousou no Frieren"]),
            idx(2, Some(2021), &["Ranking of Kings"]),
        ];
        let m = best_match("Sousou no Frieren", None, None, &index).unwrap();
        assert_eq!(m.media_id, 1);
    }

    #[test]
    fn season_pack_named_bare_still_matches() {
        let index = vec![idx(10, Some(2023), &["Jujutsu Kaisen 2nd Season"])];
        let m = best_match("Jujutsu Kaisen", Some(2), None, &index).unwrap();
        assert_eq!(m.media_id, 10);
    }

    #[test]
    fn unrelated_title_does_not_match() {
        let index = vec![idx(1, None, &["Cowboy Bebop"])];
        assert!(best_match("Neon Genesis Evangelion", None, None, &index).is_none());
    }

    #[test]
    fn ambiguous_between_two_seasons_is_rejected() {
        let index = vec![
            idx(1, Some(2019), &["Mushoku Tensei"]),
            idx(2, Some(2021), &["Mushoku Tensei"]),
        ];
        assert!(best_match("Mushoku Tensei", None, None, &index).is_none());
    }

    #[test]
    fn year_breaks_a_near_tie() {
        let index = vec![
            idx(1, Some(2006), &["Higurashi no Naku Koro ni"]),
            idx(2, Some(2020), &["Higurashi no Naku Koro ni Gou"]),
        ];
        let m = best_match("Higurashi no Naku Koro ni", None, Some(2006), &index).unwrap();
        assert_eq!(m.media_id, 1);
    }
}
