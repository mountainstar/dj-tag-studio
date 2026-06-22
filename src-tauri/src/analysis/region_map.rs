use std::collections::HashSet;

use crate::types::{TagGroup, TagSuggestion, Track};

use super::signals::TrackSignals;

#[derive(Debug, Clone)]
pub struct RegionHit {
    pub tag_name: String,
    pub confidence: f64,
    pub reason: String,
}

/// Detect cultural/regional origin hints for Genre tagging.
pub fn detect_regions(signals: &TrackSignals) -> Vec<RegionHit> {
    let mut hits = Vec::new();

    for (tag, keywords, script_hint) in region_profiles() {
        let mut score = 0.0f64;
        let mut reasons = Vec::new();

        for (kw, weight) in *keywords {
            if signals.corpus.contains(kw) || signals.path_folders.contains(kw) {
                score = score.max(*weight);
                reasons.push(format!("'{kw}'"));
            }
        }

        if let Some(script_reason) = script_hint.and_then(|hint| signals.script_match(hint)) {
            score = score.max(0.88);
            reasons.push(script_reason);
        }

        if score >= 0.62 {
            hits.push(RegionHit {
                tag_name: tag.to_string(),
                confidence: score,
                reason: if reasons.is_empty() {
                    "regional metadata".into()
                } else {
                    format!("region signals: {}", reasons.join(", "))
                },
            });
        }
    }

    hits.sort_by(|a, b| {
        b.confidence
            .partial_cmp(&a.confidence)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    hits.dedup_by(|a, b| a.tag_name == b.tag_name);
    hits
}

pub fn score_region_genre_tag(tag_name: &str, signals: &TrackSignals) -> (f64, String) {
    let hits = detect_regions(signals);
    if let Some(hit) = hits.iter().find(|h| tags_equivalent(&h.tag_name, tag_name)) {
        return (hit.confidence, hit.reason.clone());
    }
    (0.0, String::new())
}

fn tags_equivalent(a: &str, b: &str) -> bool {
    a.eq_ignore_ascii_case(b)
        || (a.eq_ignore_ascii_case("East-Asian") && b.contains("Asian"))
}

/// Suggest a region Genre tag when detected but not yet in the user's My Tags schema.
pub fn region_genre_suggestions(
    track: &Track,
    groups: &[TagGroup],
    existing: &[TagSuggestion],
) -> Vec<TagSuggestion> {
    let genre_group = match groups.iter().find(|g| g.name == "Genre") {
        Some(g) => g,
        None => return vec![],
    };

    let signals = TrackSignals::from_track(track);
    let hits = detect_regions(&signals);
    if hits.is_empty() {
        return vec![];
    }

    let already: HashSet<_> = existing
        .iter()
        .filter(|s| s.group_name == "Genre")
        .map(|s| s.tag_name.to_lowercase())
        .collect();

    let hit = &hits[0];
    if already.contains(&hit.tag_name.to_lowercase()) {
        return vec![];
    }

    if genre_group
        .tags
        .iter()
        .any(|t| tags_equivalent(&t.name, &hit.tag_name))
    {
        return vec![];
    }

    vec![TagSuggestion {
        track_id: track.id.clone(),
        tag_id: String::new(),
        tag_name: hit.tag_name.clone(),
        group_name: genre_group.name.clone(),
        confidence: hit.confidence,
        reason: format!(
            "Regional origin: {} — add '{}' to Genre?",
            hit.reason, hit.tag_name
        ),
        pending_create: true,
    }]
}

type RegionProfile = (&'static str, &'static [(&'static str, f64)], Option<&'static str>);

fn region_profiles() -> &'static [RegionProfile] {
    &[
        (
            "Desi",
            &[
                ("bollywood", 0.95),
                ("bhangra", 0.92),
                ("punjabi", 0.9),
                ("desi", 0.92),
                ("hindi", 0.88),
                ("tamil", 0.88),
                ("telugu", 0.88),
                ("garba", 0.85),
                ("filmi", 0.9),
                ("india", 0.82),
                ("pakistan", 0.8),
                ("bombay", 0.78),
                ("mumbai", 0.78),
            ],
            Some("devanagari"),
        ),
        (
            "Latin",
            &[
                ("latin", 0.9),
                ("reggaeton", 0.94),
                ("salsa", 0.9),
                ("bachata", 0.9),
                ("cumbia", 0.88),
                ("dembow", 0.88),
                ("merengue", 0.88),
                ("baile funk", 0.9),
                ("funk carioca", 0.9),
                ("spanish", 0.72),
                ("portuguese", 0.7),
                ("mexico", 0.78),
                ("colombia", 0.78),
                ("brazil", 0.8),
                ("caribbean", 0.82),
            ],
            None,
        ),
        (
            "Afro",
            &[
                ("afro", 0.88),
                ("afrobeats", 0.94),
                ("afrobeat", 0.92),
                ("amapiano", 0.94),
                ("gqom", 0.9),
                ("afro house", 0.92),
                ("nigeria", 0.85),
                ("ghana", 0.85),
                ("south africa", 0.85),
                ("lagos", 0.82),
            ],
            None,
        ),
        (
            "Arabic",
            &[
                ("arabic", 0.92),
                ("khaleeji", 0.92),
                ("shaabi", 0.9),
                ("dabke", 0.88),
                ("middle east", 0.85),
                ("lebanon", 0.82),
                ("egypt", 0.82),
                ("turkish", 0.78),
                ("belly", 0.75),
            ],
            Some("arabic"),
        ),
        (
            "East-Asian",
            &[
                ("k-pop", 0.94),
                ("kpop", 0.94),
                ("j-pop", 0.92),
                ("jpop", 0.92),
                ("city pop", 0.88),
                ("mandopop", 0.9),
                ("c-pop", 0.9),
                ("anime", 0.82),
                ("korean", 0.85),
                ("japanese", 0.85),
                ("chinese", 0.85),
                ("tokyo", 0.78),
                ("seoul", 0.78),
            ],
            Some("cjk"),
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_desi_from_bollywood_path() {
        let signals = TrackSignals {
            corpus: "remix".into(),
            path_folders: "bollywood wedding".into(),
            ..Default::default()
        };
        let hits = detect_regions(&signals);
        assert!(hits.iter().any(|h| h.tag_name == "Desi"));
    }

    #[test]
    fn detects_latin_from_reggaeton() {
        let signals = TrackSignals {
            corpus: "perreo reggaeton edit".into(),
            ..Default::default()
        };
        let hits = detect_regions(&signals);
        assert!(hits.iter().any(|h| h.tag_name == "Latin"));
    }
}
