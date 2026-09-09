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
/// A match found only through a `synonyms` entry (fan translations, other-
/// language titles, abbreviations) is discounted to this fraction of its raw
/// score — an obscure show sharing a synonym with a household name shouldn't win
/// on that alone.
const SYNONYM_WEIGHT: f64 = 0.85;

/// Raw fields from one `media_cache` row, before normalisation.
pub struct IndexInput {
    pub service: ServiceKind,
    pub media_id: i64,
    pub season_year: Option<i32>,
    pub popularity: Option<i64>,
    pub format: Option<String>,
    /// romaji / english / native.
    pub primary: Vec<String>,
    pub synonyms: Vec<String>,
}

/// One row of the search index, prepared once per scan.
#[derive(Debug, Clone)]
pub struct IndexEntry {
    pub service: ServiceKind,
    pub media_id: i64,
    pub season_year: Option<i32>,
    /// AniList list-count; 0 when unknown. Breaks near-ties.
    pub popularity: i64,
    /// TV_SHORT / MUSIC — weaker candidates when a title collides.
    pub minor_format: bool,
    /// Normalised romaji / english / native.
    pub primary_titles: Vec<String>,
    /// Normalised synonyms.
    pub synonyms: Vec<String>,
}

impl IndexEntry {
    pub fn new(input: IndexInput) -> Self {
        let norm = |raw: Vec<String>| {
            let mut v: Vec<String> = raw
                .into_iter()
                .map(|t| normalize(&t))
                .filter(|t| !t.is_empty())
                .collect();
            v.sort();
            v.dedup();
            v
        };
        Self {
            service: input.service,
            media_id: input.media_id,
            season_year: input.season_year,
            popularity: input.popularity.unwrap_or(0).max(0),
            minor_format: matches!(input.format.as_deref(), Some("TV_SHORT") | Some("MUSIC")),
            primary_titles: norm(input.primary),
            synonyms: norm(input.synonyms),
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
    // Containment ("Show" inside "Show 2") is a good signal, but scale it by how
    // much extra the longer title carries — "Sword Art Online" sits inside both
    // "Sword Art Online II" *and* "Sword Art Online Alternative: GGO II", and
    // only the first is a real match. A *prefix* relationship
    // ("demon slayer" → "demon slayer kimetsu no yaiba") is stronger still.
    let words = |s: &str| s.split_whitespace().count() as f64;
    let contains = if file.contains(cand) || cand.contains(file) {
        let (short, long) = {
            let (a, b) = (words(file), words(cand));
            (a.min(b), a.max(b))
        };
        let ratio = short / long;
        let prefix = cand.starts_with(file) || file.starts_with(cand);
        let floor = if prefix { 0.72 } else { 0.55 };
        floor + (1.0 - floor) * ratio
    } else {
        0.0
    };
    overlap.max(contains)
}

/// Best match for a parsed file, trying the file-name title and the folder-name
/// guess and keeping whichever scores higher. `None` if nothing clears the bar
/// or it's ambiguous. `file_season` (when > 1) nudges toward titles that mention
/// it.
pub fn best_match(
    file_title: &str,
    folder_title: Option<&str>,
    file_season: Option<i64>,
    file_year: Option<i32>,
    index: &[IndexEntry],
) -> Option<Match> {
    let mut best: Option<Match> = None;
    for candidate in [Some(file_title), folder_title].into_iter().flatten() {
        if let Some(m) = match_one(candidate, file_season, file_year, index) {
            if best.map(|b| m.score > b.score).unwrap_or(true) {
                best = Some(m);
            }
        }
    }
    best
}

struct Scored<'a> {
    score: f64,
    /// This entry's title carries the file's season number (e.g. "… II").
    season_matched: bool,
    entry: &'a IndexEntry,
}

fn match_one(
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

    let mut scored: Vec<Scored> = index
        .iter()
        .map(|entry| {
            let primary = entry
                .primary_titles
                .iter()
                .map(|t| score_pair(&norm, t))
                .fold(0.0_f64, f64::max);
            let synonym = entry
                .synonyms
                .iter()
                .map(|t| score_pair(&norm, t) * SYNONYM_WEIGHT)
                .fold(0.0_f64, f64::max);
            let mut best = primary.max(synonym);

            // Season agreement: if the file says S2, favour an entry whose title
            // ends in "2"; gently penalise ones that look like season 1.
            let mut season_matched = false;
            if let Some(hint) = &season_hint {
                let all_titles = entry.primary_titles.iter().chain(&entry.synonyms);
                let mentions = all_titles
                    .clone()
                    .any(|t| t.split_whitespace().last() == Some(hint.as_str()));
                if mentions {
                    best += 0.10;
                    season_matched = true;
                } else if all_titles.clone().all(|t| {
                    !t.chars().last().map(|c| c.is_ascii_digit()).unwrap_or(false)
                }) {
                    best -= 0.06;
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

            Scored {
                score: best.clamp(0.0, 1.0),
                season_matched,
                entry,
            }
        })
        .collect();

    scored.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));

    let top = scored.first()?;
    if top.score < AUTO_THRESHOLD {
        return None;
    }
    let mk = |s: &Scored| Match {
        service: s.entry.service,
        media_id: s.entry.media_id,
        score: s.score,
    };

    let Some(second) = scored.get(1) else {
        return Some(mk(top));
    };
    // The season number is a decisive signal: if only the top entry carries it,
    // a close runner-up isn't real ambiguity.
    let season_breaks_tie = top.season_matched && !second.season_matched;
    let close = top.score - second.score < AMBIGUITY_MARGIN
        && second.entry.media_id != top.entry.media_id
        && second.score >= AUTO_THRESHOLD;

    if !close || season_breaks_tie {
        return Some(mk(top));
    }

    // A near-tie. Popularity can still settle it — but only the lopsided case:
    // an obscure entry that only ranks because it shares a synonym with a
    // household name loses to the household name. Two comparably popular
    // franchise entries stay a genuine tie and go to review.
    let cluster: Vec<&Scored> = scored
        .iter()
        .take_while(|s| top.score - s.score < AMBIGUITY_MARGIN)
        .collect();
    rescue_by_popularity(&cluster).map(mk)
}

/// A TV short / music video sharing a title is even less likely to be what the
/// folder meant — discount its weight.
fn pop_weight(s: &Scored) -> i64 {
    if s.entry.minor_format {
        s.entry.popularity / 4
    } else {
        s.entry.popularity
    }
}

/// If the highest-*scored* entry is obscure and a near-tied alternative dwarfs
/// it in popularity (≥10×), that alternative is what the folder meant.
fn rescue_by_popularity<'a>(cluster: &[&'a Scored<'a>]) -> Option<&'a Scored<'a>> {
    let score_winner = cluster.first()?;
    let popular = cluster.iter().max_by_key(|s| pop_weight(s))?;
    if popular.entry.media_id != score_winner.entry.media_id
        && pop_weight(popular) >= 20_000
        && pop_weight(popular) >= pop_weight(score_winner).max(1) * 10
    {
        Some(*popular)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn idx(id: i64, year: Option<i32>, titles: &[&str]) -> IndexEntry {
        IndexEntry::new(IndexInput {
            service: ServiceKind::AniList,
            media_id: id,
            season_year: year,
            popularity: None,
            format: Some("TV".into()),
            primary: titles.iter().map(|s| s.to_string()).collect(),
            synonyms: vec![],
        })
    }

    /// An entry with distinct primary titles, synonyms, popularity and format.
    fn idx_full(
        id: i64,
        year: Option<i32>,
        popularity: i64,
        format: &str,
        primary: &[&str],
        synonyms: &[&str],
    ) -> IndexEntry {
        IndexEntry::new(IndexInput {
            service: ServiceKind::AniList,
            media_id: id,
            season_year: year,
            popularity: Some(popularity),
            format: Some(format.into()),
            primary: primary.iter().map(|s| s.to_string()).collect(),
            synonyms: synonyms.iter().map(|s| s.to_string()).collect(),
        })
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
        let m = best_match("Sousou no Frieren", None, None, None, &index).unwrap();
        assert_eq!(m.media_id, 1);
    }

    #[test]
    fn season_pack_named_bare_still_matches() {
        let index = vec![idx(10, Some(2023), &["Jujutsu Kaisen 2nd Season"])];
        let m = best_match("Jujutsu Kaisen", None, Some(2), None, &index).unwrap();
        assert_eq!(m.media_id, 10);
    }

    #[test]
    fn unrelated_title_does_not_match() {
        let index = vec![idx(1, None, &["Cowboy Bebop"])];
        assert!(best_match("Neon Genesis Evangelion", None, None, None, &index).is_none());
    }

    #[test]
    fn ambiguous_between_two_seasons_is_rejected() {
        let index = vec![
            idx(1, Some(2019), &["Mushoku Tensei"]),
            idx(2, Some(2021), &["Mushoku Tensei"]),
        ];
        assert!(best_match("Mushoku Tensei", None, None, None, &index).is_none());
    }

    #[test]
    fn year_breaks_a_near_tie() {
        let index = vec![
            idx(1, Some(2006), &["Higurashi no Naku Koro ni"]),
            idx(2, Some(2020), &["Higurashi no Naku Koro ni Gou"]),
        ];
        let m =
            best_match("Higurashi no Naku Koro ni", None, None, Some(2006), &index).unwrap();
        assert_eq!(m.media_id, 1);
    }

    #[test]
    fn season_number_picks_the_sequel_over_the_original() {
        let index = vec![
            idx(11757, Some(2012), &["Sword Art Online"]),
            idx(20594, Some(2014), &["Sword Art Online II"]),
        ];
        // Folder "Sword Art Online", Season 2 -> the "II" entry, not the original.
        let m = best_match("SAO", Some("Sword Art Online"), Some(2), None, &index).unwrap();
        assert_eq!(m.media_id, 20594);
        // Season 1 -> the original.
        let m = best_match("SAO", Some("Sword Art Online"), Some(1), None, &index).unwrap();
        assert_eq!(m.media_id, 11757);
    }

    #[test]
    fn obscure_synonym_collision_loses_to_the_popular_show() {
        // "Onigiri" (2016 short) carries the synonym "Demon Slayer"; the folder
        // "Demon Slayer" means the household name, not the short.
        let index = vec![
            idx_full(21612, Some(2016), 3_000, "TV_SHORT", &["Onigiri"], &["Demon Slayer"]),
            idx_full(
                101922,
                Some(2019),
                600_000,
                "TV",
                &["Kimetsu no Yaiba", "Demon Slayer: Kimetsu no Yaiba"],
                &["KnY"],
            ),
        ];
        let m = best_match("Demon Slayer", Some("Demon Slayer"), Some(1), None, &index)
            .unwrap();
        assert_eq!(m.media_id, 101922);
    }

    #[test]
    fn arc_named_second_season_goes_to_review() {
        // Folder "Demon Slayer" S2, but the real S2 entries are arc-named with no
        // "2" anywhere — nothing should auto-match.
        let index = vec![
            idx_full(21612, Some(2016), 3_000, "TV_SHORT", &["Onigiri"], &["Demon Slayer"]),
            idx_full(
                101922,
                Some(2019),
                600_000,
                "TV",
                &["Kimetsu no Yaiba", "Demon Slayer: Kimetsu no Yaiba"],
                &[],
            ),
            idx_full(
                142329,
                Some(2021),
                300_000,
                "TV",
                &["Kimetsu no Yaiba: Yuukaku-hen", "Demon Slayer: Entertainment District Arc"],
                &[],
            ),
        ];
        assert!(
            best_match("Demon Slayer", Some("Demon Slayer"), Some(2), None, &index).is_none()
        );
    }

    #[test]
    fn folder_title_rescues_a_generic_file_name() {
        let index = vec![idx(1, Some(2012), &["Sword Art Online"])];
        // The file name is useless ("13"); the folder carries the real title.
        assert!(best_match("13", None, None, None, &index).is_none());
        let m = best_match("13", Some("Sword Art Online"), None, None, &index).unwrap();
        assert_eq!(m.media_id, 1);
    }
}
