//! Detect genre/style strings that are not yet in the My Tag schema.

use crate::types::{TagGroup, TagSuggestion, Track};

use super::signals::TrackSignals;

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
        .map(|s| s.tag_name.to_lowercase())
        .collect();

    let mut out = Vec::new();

    for candidate in candidates {
        if candidate.len() < 2 {
            continue;
        }
        let lower = candidate.to_lowercase();
        if already_suggested.contains(&lower) {
            continue;
        }
        if genre_group
            .tags
            .iter()
            .any(|t| t.name.to_lowercase() == lower)
        {
            continue;
        }
        // Skip if an existing tag is a close substring match
        if genre_group.tags.iter().any(|t| {
            let t_lower = t.name.to_lowercase();
            t_lower.contains(&lower) || lower.contains(&t_lower)
        }) {
            continue;
        }

        out.push(TagSuggestion {
            track_id: track.id.clone(),
            tag_id: String::new(),
            tag_name: candidate.clone(),
            group_name: genre_group.name.clone(),
            confidence: 0.72,
            reason: format!("Rekordbox genre '{candidate}' is not in your My Tags — add it?"),
            pending_create: true,
        });
        break; // one unmapped genre proposal per track
    }

    out
}

fn collect_genre_candidates(signals: &TrackSignals) -> Vec<String> {
    let mut out = Vec::new();

    if !signals.genre_field.is_empty() {
        out.push(normalize_genre_label(&signals.genre_field));
    }

    // Pull genre-like tokens from embedded tags / comments (after "genre:" prefix)
    for token in signals.corpus.split(|c: char| c == ',' || c == ';' || c == '|') {
        let t = token.trim();
        if t.starts_with("genre:") {
            out.push(normalize_genre_label(t.trim_start_matches("genre:").trim()));
        }
    }

    out.sort();
    out.dedup();
    out
}

fn normalize_genre_label(raw: &str) -> String {
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
        };
        let unmapped = unmapped_genre_suggestions(&track, &groups, &[]);
        assert_eq!(unmapped.len(), 1);
        assert!(unmapped[0].pending_create);
        assert_eq!(unmapped[0].tag_name, "Melodic Techno");
    }
}
