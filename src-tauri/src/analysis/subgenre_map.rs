use std::collections::HashSet;

use crate::types::{TagGroup, TagSuggestion, Track};

use super::audio::AudioFeatures;
use super::signals::TrackSignals;

#[derive(Debug, Clone)]
pub struct SubgenreHit {
    pub tag_name: &'static str,
    pub confidence: f64,
    pub reason: String,
}

/// Infer house/electronic sub-genres beyond the Rekordbox genre field.
pub fn detect_subgenres(signals: &TrackSignals, audio: &AudioFeatures) -> Vec<SubgenreHit> {
    let mut hits = Vec::new();

    if let Some(hit) = detect_classic_house(signals, audio) {
        hits.push(hit);
    }
    if let Some(hit) = detect_acid_house(signals) {
        hits.push(hit);
    }
    if let Some(hit) = detect_piano_house(signals, audio) {
        // Piano house often overlaps classic — keep the stronger signal only
        if !hits.iter().any(|h| h.tag_name == "Classic House" && h.confidence >= hit.confidence) {
            hits.push(hit);
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

pub fn score_subgenre_tag(
    tag_name: &str,
    signals: &TrackSignals,
    audio: &AudioFeatures,
) -> (f64, String) {
    let hits = detect_subgenres(signals, audio);
    if let Some(hit) = hits.iter().find(|h| h.tag_name.eq_ignore_ascii_case(tag_name)) {
        return (hit.confidence, hit.reason.clone());
    }
    (0.0, String::new())
}

pub fn classic_house_signals(signals: &TrackSignals) -> bool {
    detect_classic_house(signals, &AudioFeatures::default()).is_some()
}

fn detect_classic_house(signals: &TrackSignals, audio: &AudioFeatures) -> Option<SubgenreHit> {
    if let Some(kw) = signals.contains_any(&[
        "classic house",
        "classic-house",
        "old skool house",
        "old school house",
        "oldskool house",
        "rave classic",
        "rave classics",
        "90s house",
        "90's house",
        "golden age house",
    ]) {
        return Some(SubgenreHit {
            tag_name: "Classic House",
            confidence: 0.94,
            reason: format!("keyword '{kw}'"),
        });
    }

    if signals.genre_field.contains("classic house") {
        return Some(SubgenreHit {
            tag_name: "Classic House",
            confidence: 0.96,
            reason: format!("genre field is '{}'", signals.genre_field),
        });
    }

    let era_year = signals.inferred_year();
    let house_context = signals.is_house_context();
    let classic_path = signals.contains_any(&[
        "classic house",
        "classics",
        "classic",
        "old skool",
        "old school",
        "oldskool",
        "90s",
        "1990",
        "1991",
        "1992",
        "1993",
        "1994",
        "1995",
    ]).is_some();

    if classic_path && house_context {
        return Some(SubgenreHit {
            tag_name: "Classic House",
            confidence: 0.88,
            reason: "classic/90s folder + house track".into(),
        });
    }

    if let Some(year) = era_year {
        if (1987..=1995).contains(&year) && house_context && signals.bpm >= 118.0 && signals.bpm <= 132.0
        {
            return Some(SubgenreHit {
                tag_name: "Classic House",
                confidence: 0.82,
                reason: format!("{year} house-era track at {:.0} BPM", signals.bpm),
            });
        }
    }

    if house_context && signals.bpm >= 118.0 && signals.bpm <= 130.0 {
        if let Some(artist) = classic_house_artist_match(signals) {
            return Some(SubgenreHit {
                tag_name: "Classic House",
                confidence: 0.84,
                reason: format!("classic house artist '{artist}'"),
            });
        }
    }

    // Piano-forward late-80s/90s house (e.g. N-Joi – Anthem) without explicit metadata
    if house_context
        && signals.bpm >= 118.0
        && signals.bpm <= 130.0
        && audio.analyzed
        && audio.brightness > 0.14
        && audio.onset_density > 0.01
        && audio.vocal_ratio > 0.1
        && audio.vocal_ratio < 0.42
    {
        return Some(SubgenreHit {
            tag_name: "Classic House",
            confidence: 0.68,
            reason: "piano-house spectrum + classic BPM range".into(),
        });
    }

    None
}

fn detect_acid_house(signals: &TrackSignals) -> Option<SubgenreHit> {
    if signals.contains_any(&["acid house", "acid-house", "acid tb", "303"]).is_some() {
        return Some(SubgenreHit {
            tag_name: "Acid House",
            confidence: 0.9,
            reason: "acid house keywords".into(),
        });
    }
    None
}

fn detect_piano_house(signals: &TrackSignals, audio: &AudioFeatures) -> Option<SubgenreHit> {
    if signals.contains_any(&["piano house", "piano-house"]).is_some() {
        return Some(SubgenreHit {
            tag_name: "Classic House",
            confidence: 0.9,
            reason: "piano house keywords".into(),
        });
    }
    if signals.contains_any(&["piano", "keys", "rhodes"]).is_some() && signals.is_house_context() {
        return Some(SubgenreHit {
            tag_name: "Classic House",
            confidence: 0.72,
            reason: "piano/keys + house context".into(),
        });
    }
    if audio.analyzed
        && signals.is_house_context()
        && audio.brightness > 0.15
        && audio.onset_density > 0.01
        && audio.vocal_ratio > 0.1
    {
        return Some(SubgenreHit {
            tag_name: "Classic House",
            confidence: 0.66,
            reason: "keyed house stabs in audio".into(),
        });
    }
    None
}

fn classic_house_artist_match(signals: &TrackSignals) -> Option<&'static str> {
    let artist = signals.raw_artist.to_lowercase();
    for name in CLASSIC_HOUSE_ARTISTS {
        if artist.contains(name) {
            return Some(name);
        }
    }
    None
}

const CLASSIC_HOUSE_ARTISTS: &[&str] = &[
    "n-joi",
    "n joi",
    "joe smooth",
    "robin s",
    "crystal waters",
    "black box",
    "808 state",
    "frankie knuckles",
    "marshall jefferson",
    "steve \"silk\" hurley",
    "steve silk hurley",
    "inner city",
    "rave",
    "altern 8",
    "shamen",
];

/// Suggest a sub-genre tag (e.g. Classic House) when inferred but missing from My Tags.
pub fn subgenre_genre_suggestions(
    track: &Track,
    groups: &[TagGroup],
    existing: &[TagSuggestion],
    audio: &AudioFeatures,
) -> Vec<TagSuggestion> {
    let genre_group = match groups.iter().find(|g| g.name == "Genre") {
        Some(g) => g,
        None => return vec![],
    };

    let signals = TrackSignals::from_track(track);
    let hits = detect_subgenres(&signals, audio);
    if hits.is_empty() {
        return vec![];
    }

    let already: HashSet<_> = existing
        .iter()
        .filter(|s| s.group_name == "Genre")
        .map(|s| s.tag_name.to_lowercase())
        .collect();

    let mut out = Vec::new();
    for hit in hits.into_iter().take(2) {
        let lower = hit.tag_name.to_lowercase();
        if already.contains(&lower) {
            continue;
        }
        if genre_group
            .tags
            .iter()
            .any(|t| t.name.eq_ignore_ascii_case(hit.tag_name))
        {
            continue;
        }

        out.push(TagSuggestion {
            track_id: track.id.clone(),
            tag_id: String::new(),
            tag_name: hit.tag_name.to_string(),
            group_name: genre_group.name.clone(),
            confidence: hit.confidence,
            reason: format!("Sub-genre: {} — add '{}' to Genre?", hit.reason, hit.tag_name),
            pending_create: true,
        });
        break;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_classic_house_from_genre_field() {
        let signals = TrackSignals {
            genre_field: "classic house".into(),
            ..Default::default()
        };
        let hits = detect_subgenres(&signals, &AudioFeatures::default());
        assert!(hits.iter().any(|h| h.tag_name == "Classic House"));
    }

    #[test]
    fn detects_classic_house_from_1990_path() {
        let signals = TrackSignals {
            corpus: "anthem n-joi house".into(),
            genre_field: "house".into(),
            bpm: 125.0,
            path_folders: "1990 classics".into(),
            ..Default::default()
        };
        let hits = detect_subgenres(&signals, &AudioFeatures::default());
        assert!(hits.iter().any(|h| h.tag_name == "Classic House"));
    }

    #[test]
    fn detects_classic_house_from_n_joi_artist() {
        let signals = TrackSignals {
            corpus: "anthem original mix n-joi house".into(),
            genre_field: "house".into(),
            raw_artist: "N-Joi".into(),
            bpm: 125.0,
            ..Default::default()
        };
        let hits = detect_subgenres(&signals, &AudioFeatures::default());
        assert!(hits.iter().any(|h| h.tag_name == "Classic House"));
    }

    #[test]
    fn proposes_classic_house_when_missing_from_schema() {
        use crate::types::{MyTagDef, TagGroup};

        let groups = vec![TagGroup {
            id: "g1".into(),
            name: "Genre".into(),
            seq: 1,
            tags: vec![MyTagDef {
                id: "t1".into(),
                name: "House".into(),
                group_id: "g1".into(),
                seq: 1,
            }],
        }];
        let track = Track {
            id: "1".into(),
            title: "Anthem (Original Mix)".into(),
            artist: "N-Joi".into(),
            album: String::new(),
            genre: "House".into(),
            bpm: 125.0,
            path: "/Music/1990 Classics/anthem.mp3".into(),
            rating: 0,
            comment: String::new(),
            tag_ids: vec![],
        };
        let pending = subgenre_genre_suggestions(&track, &groups, &[], &AudioFeatures::default());
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].tag_name, "Classic House");
        assert!(pending[0].pending_create);
    }
}
