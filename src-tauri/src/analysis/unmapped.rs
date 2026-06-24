//! Detect genre/style strings that are not yet in the My Tag schema.

use crate::types::{TagGroup, TagSuggestion, Track};

use super::signals::TrackSignals;

pub const UNMAPPED_GENRE_CONFIDENCE: f64 = 0.94;

pub fn genre_key(label: &str) -> String {
    label
        .trim()
        .to_ascii_lowercase()
        .replace(['-', '_'], " ")
}

pub fn genre_in_schema(label: &str, genre_group: &TagGroup) -> bool {
    let norm = genre_key(label);
    genre_group
        .tags
        .iter()
        .any(|t| genre_key(&t.name) == norm)
}

/// True when a picked tag is a broad genre named inside a more specific Rekordbox genre field.
pub fn is_broad_genre_within_field(pick: &str, genre_field: &str) -> bool {
    let field = genre_key(genre_field);
    let p = genre_key(pick);
    if field == p || field.is_empty() || p.is_empty() {
        return false;
    }
    if field.len() <= p.len() {
        return false;
    }
    field
        .split(|c: char| !c.is_alphanumeric())
        .filter(|word| !word.is_empty())
        .any(|word| word == p)
}

pub fn genre_field_needs_new_tag(signals: &TrackSignals, groups: &[TagGroup]) -> bool {
    let genre_field = signals.genre_field.trim();
    if genre_field.is_empty() {
        return false;
    }
    groups
        .iter()
        .find(|g| g.name == "Genre")
        .map(|gg| !genre_in_schema(genre_field, gg))
        .unwrap_or(false)
}

pub fn should_offer_existing_genre_pick(
    pick_name: &str,
    signals: &TrackSignals,
    groups: &[TagGroup],
) -> bool {
    if !genre_field_needs_new_tag(signals, groups) {
        return true;
    }
    !is_broad_genre_within_field(pick_name, &signals.genre_field)
}

pub fn unmapped_genre_suggestions(
    track: &Track,
    groups: &[TagGroup],
    existing_suggestions: &[TagSuggestion],
) -> Vec<TagSuggestion> {
    let genre_group = match groups.iter().find(|g| g.name == "Genre") {
        Some(g) => g,
        None => return vec![],
    };

    let signals = TrackSignals::from_track(track);
    let candidates = collect_genre_candidates(&signals);

    let already_suggested: std::collections::HashSet<_> = existing_suggestions
        .iter()
        .filter(|s| s.group_name == "Genre")
        .map(|s| genre_key(&s.tag_name))
        .collect();

    let mut out = Vec::new();

    for candidate in candidates {
        if candidate.len() < 2 {
            continue;
        }
        let lower = genre_key(&candidate);
        if already_suggested.contains(&lower) {
            continue;
        }
        if genre_in_schema(&candidate, genre_group) {
            continue;
        }

        out.push(TagSuggestion {
            track_id: track.id.clone(),
            tag_id: String::new(),
            tag_name: candidate.clone(),
            group_name: genre_group.name.clone(),
            confidence: UNMAPPED_GENRE_CONFIDENCE,
            reason: format!(
                "Rekordbox genre '{candidate}' is not in your My Tags — add it?"
            ),
            pending_create: true,
        });
        break; // one unmapped genre proposal per track
    }

    out
}

fn collect_genre_candidates(signals: &TrackSignals) -> Vec<String> {
    let mut out = Vec::new();

    if !signals.genre_field.is_empty() {
        let full = normalize_genre_label(&signals.genre_field);
        if !full.is_empty() {
            out.push(full);
        }
        for part in signals.genre_field.split(['&', '|', '/']) {
            let label = normalize_genre_label(part);
            if label.len() >= 2 {
                out.push(label);
            }
        }
    }

    // Pull genre-like tokens from embedded tags / comments (after "genre:" prefix)
    for token in signals.corpus.split(|c: char| c == ',' || c == ';' || c == '|') {
        let t = token.trim();
        if t.starts_with("genre:") {
            out.push(normalize_genre_label(t.trim_start_matches("genre:").trim()));
        }
    }

    out.sort_by_key(|label| std::cmp::Reverse(label.len()));
    out.dedup();
    out
}

pub fn normalize_genre_label(raw: &str) -> String {
    let s = raw.trim();
    if s.is_empty() {
        return String::new();
    }
    // Title-case each word for consistency with My Tag naming
    s.split_whitespace()
        .map(|w| {
            let mut c = w.chars();
            match c.next() {
                None => String::new(),
                Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{MyTagDef, TagGroup};

    fn house_techno_group() -> TagGroup {
        TagGroup {
            id: "g1".into(),
            name: "Genre".into(),
            seq: 1,
            tags: vec![
                MyTagDef {
                    id: "t1".into(),
                    name: "House".into(),
                    group_id: "g1".into(),
                    seq: 1,
                },
                MyTagDef {
                    id: "t2".into(),
                    name: "Techno".into(),
                    group_id: "g1".into(),
                    seq: 2,
                },
            ],
        }
    }

    #[test]
    fn detects_unmapped_rekordbox_genre() {
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
            title: "Test".into(),
            artist: String::new(),
            album: String::new(),
            genre: "Melodic Techno".into(),
            bpm: 0.0,
            path: "/x.mp3".into(),
            rating: 0,
            comment: String::new(),
            tag_ids: vec![],
            ..Default::default()
        };
        let unmapped = unmapped_genre_suggestions(&track, &groups, &[]);
        assert_eq!(unmapped.len(), 1);
        assert!(unmapped[0].pending_create);
        assert_eq!(unmapped[0].tag_name, "Melodic Techno");
    }

    #[test]
    fn melodic_house_not_blocked_by_house_tag() {
        let groups = vec![house_techno_group()];
        let track = Track {
            id: "1".into(),
            title: "Alpine Air (Original Mix)".into(),
            artist: "N1NJA".into(),
            album: String::new(),
            genre: "Melodic House & Techno".into(),
            bpm: 128.0,
            path: "/x.mp3".into(),
            rating: 0,
            comment: String::new(),
            tag_ids: vec![],
            ..Default::default()
        };
        let unmapped = unmapped_genre_suggestions(&track, &groups, &[]);
        assert_eq!(unmapped.len(), 1);
        assert_eq!(unmapped[0].tag_name, "Melodic House & Techno");
        assert!(unmapped[0].pending_create);
    }

    #[test]
    fn broad_genre_within_field_detects_house_in_melodic_house() {
        assert!(is_broad_genre_within_field("House", "Melodic House & Techno"));
        assert!(is_broad_genre_within_field("Techno", "Melodic House & Techno"));
        assert!(!is_broad_genre_within_field(
            "Melodic House & Techno",
            "Melodic House & Techno"
        ));
    }
}
