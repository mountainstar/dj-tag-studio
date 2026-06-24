//! Match Rekordbox comment text to My Tag names (DJ notes often list intended tags).

use std::collections::HashSet;

use crate::types::{TagGroup, TagSuggestion, Track};

use super::unmapped::genre_in_schema;

pub const COMMENT_HINT_CONFIDENCE: f64 = 0.96;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommentHint {
    pub tag_name: String,
    pub group_name: String,
    pub reason: String,
}

/// Collect tag hints from the track comment field (and embedded file comment when present).
pub fn match_comment_hints(track: &Track, groups: &[TagGroup]) -> Vec<CommentHint> {
    let mut hints = Vec::new();
    let mut seen = HashSet::new();

    for source in comment_sources(track) {
        for hint in hints_from_text(&source.text, groups) {
            let key = (hint.group_name.clone(), hint.tag_name.clone());
            if seen.insert(key) {
                hints.push(hint);
            }
        }
    }

    hints
}

/// Suggest creating Genre tags named in comments when they are not yet in the My Tag schema.
pub fn comment_genre_create_suggestions(
    track: &Track,
    groups: &[TagGroup],
    existing: &[TagSuggestion],
) -> Vec<TagSuggestion> {
    let genre_group = match groups.iter().find(|g| g.name == "Genre") {
        Some(g) => g,
        None => return vec![],
    };

    let already_suggested: HashSet<_> = existing
        .iter()
        .filter(|s| s.group_name == "Genre")
        .map(|s| normalize(&s.tag_name))
        .collect();

    let mut out = Vec::new();
    let mut seen = HashSet::new();

    for source in comment_sources(track) {
        let tokens = tokenize(&source.text);
        let mut candidates = Vec::new();

        for token in &tokens {
            if let Some(label) = genre_label_from_comment_token(token) {
                candidates.push(label);
            }
        }
        for pair in adjacent_pairs(&tokens) {
            if let Some(label) = genre_label_from_comment_pair(&pair) {
                candidates.push(label);
            }
        }

        for label in candidates {
            let norm = normalize(&label);
            if norm.len() < 2 || seen.contains(&norm) || already_suggested.contains(&norm) {
                continue;
            }
            if genre_in_schema(&label, genre_group) {
                continue;
            }

            seen.insert(norm);
            out.push(TagSuggestion {
                track_id: track.id.clone(),
                tag_id: String::new(),
                tag_name: label.clone(),
                group_name: genre_group.name.clone(),
                confidence: COMMENT_HINT_CONFIDENCE,
                reason: format!("genre in comments: '{label}' — add it?"),
                pending_create: true,
            });
        }
    }

    out
}

pub fn score_comment_hint(
    hints: &[CommentHint],
    group_name: &str,
    tag_name: &str,
) -> (f64, String) {
    hints
        .iter()
        .find(|h| h.group_name == group_name && tags_equivalent(&h.tag_name, tag_name))
        .map(|h| (COMMENT_HINT_CONFIDENCE, h.reason.clone()))
        .unwrap_or((0.0, String::new()))
}

pub fn merge_with_comment_hint(
    confidence: f64,
    reason: String,
    hints: &[CommentHint],
    group_name: &str,
    tag_name: &str,
) -> (f64, String) {
    let (hint_conf, hint_reason) = score_comment_hint(hints, group_name, tag_name);
    if hint_conf <= 0.0 {
        return (confidence, reason);
    }

    let merged_conf = confidence.max(hint_conf);
    let merged_reason = if reason.is_empty() {
        hint_reason
    } else if confidence >= hint_conf {
        reason
    } else {
        hint_reason
    };
    (merged_conf, merged_reason)
}

struct CommentSource {
    text: String,
}

fn comment_sources(track: &Track) -> Vec<CommentSource> {
    let mut out = Vec::new();
    let rb = track.comment.trim();
    if !rb.is_empty() {
        out.push(CommentSource {
            text: rb.to_string(),
        });
    }

    if let Some(embedded) = super::signals::read_embedded_comment(track.file_path()) {
        let embedded = embedded.trim();
        if !embedded.is_empty()
            && !embedded.eq_ignore_ascii_case(rb)
            && !out.iter().any(|s| s.text.eq_ignore_ascii_case(embedded))
        {
            out.push(CommentSource {
                text: embedded.to_string(),
            });
        }
    }

    out
}

fn hints_from_text(text: &str, groups: &[TagGroup]) -> Vec<CommentHint> {
    let tokens = tokenize(text);
    let mut hints = Vec::new();

    for group in groups {
        for tag in &group.tags {
            if comment_mentions_tag(text, &tokens, &tag.name) {
                hints.push(CommentHint {
                    tag_name: tag.name.clone(),
                    group_name: group.name.clone(),
                    reason: format!("listed in comments: '{text}'"),
                });
            }
        }
    }

    hints
}

fn comment_mentions_tag(comment: &str, tokens: &[String], tag_name: &str) -> bool {
    let norm_comment = normalize(comment);
    let norm_tag = normalize(tag_name);

    if contains_phrase(&norm_comment, &norm_tag) {
        return true;
    }

    for token in tokens {
        if token_matches_tag(token, tag_name) {
            return true;
        }
    }

    for pair in adjacent_pairs(tokens) {
        if pair == norm_tag {
            return true;
        }
    }

    false
}

fn tokenize(text: &str) -> Vec<String> {
    text.split(|c: char| c.is_whitespace() || c == ',' || c == ';' || c == '|')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(ToString::to_string)
        .collect()
}

fn adjacent_pairs(tokens: &[String]) -> Vec<String> {
    let mut out = Vec::new();
    for window in tokens.windows(2) {
        out.push(format!("{} {}", normalize(&window[0]), normalize(&window[1])));
    }
    out
}

fn normalize(value: &str) -> String {
    value
        .to_ascii_lowercase()
        .replace(['-', '_'], " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn contains_phrase(haystack: &str, needle: &str) -> bool {
    if needle.is_empty() {
        return false;
    }
    if haystack == needle {
        return true;
    }
    haystack
        .split_whitespace()
        .collect::<Vec<_>>()
        .windows(needle.split_whitespace().count())
        .any(|window| window.join(" ") == needle)
}

fn tags_equivalent(a: &str, b: &str) -> bool {
    normalize(a) == normalize(b)
}

fn token_matches_tag(token: &str, tag_name: &str) -> bool {
    let token_norm = normalize(token);
    let tag_norm = normalize(tag_name);

    if token_norm == tag_norm {
        return true;
    }

    // Singular/plural: Vocal ↔ Vocals
    if token_norm == tag_norm.trim_end_matches('s')
        || format!("{token_norm}s") == tag_norm
        || token_norm.trim_end_matches('s') == tag_norm
    {
        return true;
    }

    match token_norm.as_str() {
        "anthem" | "anthemic" if tag_norm == "anthem" => true,
        "peak" if tag_norm == "peak" || tag_norm == "peak time" => true,
        "peak time" if tag_norm == "peak time" || tag_norm == "peak" => true,
        "warm up" | "warmup" if tag_norm == "warm up" || tag_norm == "opening set" => true,
        "opening" if tag_norm == "opening set" => true,
        "inst" | "instrumental" if tag_norm == "inst" || tag_norm.contains("no vocal") => true,
        "no vocal" | "no vocals" | "novocals" if tag_norm.contains("no vocal") => true,
        "hip hop" if tag_norm == "hip hop" => true,
        "rnb" | "r and b" if tag_norm == "r&b" || tag_norm == "r b" => true,
        "dnb" | "drum and bass" if tag_norm == "dnb" => true,
        "breakbeat" | "breakbeats" | "breaks" | "big beat" | "bigbeat"
            if tag_norm == "breakbeat" =>
            true,
        "east asian" | "asian" if tag_norm == "east asian" => true,
        _ => false,
    }
}

fn genre_label_from_comment_token(token: &str) -> Option<String> {
    if token_matches_non_genre_comment_token(token) {
        return None;
    }

    match normalize(token).as_str() {
        "breakbeat" | "breakbeats" | "breaks" | "bigbeat" => Some("Breakbeat".into()),
        _ => None,
    }
}

fn genre_label_from_comment_pair(pair: &str) -> Option<String> {
    match pair {
        "big beat" => Some("Breakbeat".into()),
        _ => None,
    }
}

fn token_matches_non_genre_comment_token(token: &str) -> bool {
    matches!(
        normalize(token).as_str(),
        "driving"
            | "anthemic"
            | "anthem"
            | "peak"
            | "build"
            | "start"
            | "sustain"
            | "release"
            | "vocals"
            | "vocal"
            | "instrumental"
            | "inst"
            | "french"
            | "warm"
            | "corporate"
            | "wedding"
            | "opening"
            | "closing"
            | "piano"
            | "horns"
            | "synth"
            | "percussion"
            | "cowbell"
            | "bass"
            | "heavy"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{MyTagDef, TagGroup};

    fn default_groups() -> Vec<TagGroup> {
        let pack: crate::types::TagPack = serde_json::from_str(include_str!(
            "../../../tag-packs/rekordbox-default.json"
        ))
        .unwrap();
        pack.groups
            .into_iter()
            .enumerate()
            .map(|(idx, g)| TagGroup {
                id: format!("g{idx}"),
                name: g.name,
                seq: idx as i64 + 1,
                tags: g
                    .tags
                    .into_iter()
                    .enumerate()
                    .map(|(tidx, name)| MyTagDef {
                        id: format!("g{idx}-t{tidx}"),
                        name,
                        group_id: format!("g{idx}"),
                        seq: tidx as i64 + 1,
                    })
                    .collect(),
            })
            .collect()
    }

    fn track_with_comment(comment: &str) -> Track {
        Track {
            id: "1".into(),
            title: "Talk To You".into(),
            artist: "ANOTR".into(),
            album: String::new(),
            genre: "House".into(),
            bpm: 126.0,
            path: "/nonexistent.mp3".into(),
            rating: 0,
            comment: comment.into(),
            tag_ids: vec![],
            ..Default::default()
        }
    }

    #[test]
    fn parses_space_separated_comment_tags() {
        let hints = match_comment_hints(&track_with_comment("House Driving Vocals"), &default_groups());
        let names: Vec<_> = hints
            .iter()
            .map(|h| format!("{}:{}", h.group_name, h.tag_name))
            .collect();
        assert!(names.contains(&"Genre:House".to_string()));
        assert!(names.contains(&"Components:Vocals".to_string()));
    }

    #[test]
    fn parses_hyphenated_no_vocals() {
        let hints =
            match_comment_hints(&track_with_comment("Techno Progressive No-Vocals"), &default_groups());
        assert!(hints.iter().any(|h| h.tag_name == "Techno"));
        assert!(hints.iter().any(|h| h.tag_name == "No-Vocals"));
    }

    #[test]
    fn parses_peak_and_genre_from_comment() {
        let hints =
            match_comment_hints(&track_with_comment("House Anthemic Peak"), &default_groups());
        assert!(hints.iter().any(|h| h.tag_name == "House"));
        assert!(hints.iter().any(|h| h.tag_name == "Peak"));
    }

    #[test]
    fn demo_talk_to_you_comment_suggests_vocals() {
        use super::super::suggest_tags;

        let groups = default_groups();
        let track = track_with_comment("House Driving Vocals");
        let suggestions = suggest_tags(&track, &groups);
        assert!(
            suggestions
                .iter()
                .any(|s| s.tag_name == "Vocals" && s.confidence >= 0.96),
            "expected Vocals from comment, got {:?}",
            suggestions
                .iter()
                .map(|s| (&s.group_name, &s.tag_name, s.confidence))
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn breakbeat_in_comment_suggests_genre_tag() {
        use super::super::suggest_tags;

        let groups = default_groups();
        let track = track_with_comment("Breakbeat Driving Vocals");
        let suggestions = suggest_tags(&track, &groups);
        assert!(
            suggestions.iter().any(|s| {
                s.group_name == "Genre"
                    && s.tag_name == "Breakbeat"
                    && s.confidence >= 0.96
                    && !s.pending_create
            }),
            "expected Breakbeat genre from comment, got {:?}",
            suggestions
                .iter()
                .map(|s| (&s.group_name, &s.tag_name, s.pending_create, s.confidence))
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn breakbeat_in_comment_proposes_create_when_missing_from_schema() {
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
        let track = track_with_comment("Breakbeat Vocals");
        let suggestions = comment_genre_create_suggestions(&track, &groups, &[]);
        assert_eq!(suggestions.len(), 1);
        assert_eq!(suggestions[0].tag_name, "Breakbeat");
        assert!(suggestions[0].pending_create);
    }
}
