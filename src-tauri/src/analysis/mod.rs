mod audio;
mod comment_map;
mod components_map;
mod energy_map;
mod genre_map;
mod region_map;
mod signals;
mod subgenre_map;
mod situation_map;
mod unmapped;

use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};

use crate::types::{MyTagDef, TagGroup, TagSuggestion, Track};

use audio::{analyze_path, AudioFeatures};
use comment_map::{
    comment_genre_create_suggestions, match_comment_hints, merge_with_comment_hint,
};
use components_map::{component_fallback, pick_components, score_component_tag};
use energy_map::{energy_fallback, score_energy_tag};
use genre_map::score_genre_tag;
use region_map::{region_genre_suggestions, score_region_genre_tag};
use signals::TrackSignals;
use situation_map::{score_situation_tag, situation_fallback as native_situation_fallback};
use subgenre_map::{
    classic_house_signals, score_subgenre_tag, subgenre_genre_suggestions,
};
use unmapped::{genre_field_needs_new_tag, should_offer_existing_genre_pick};

static AUDIO_CACHE: LazyLock<Mutex<HashMap<String, AudioFeatures>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

const MIN_CONFIDENCE: f64 = 0.52;
const MAX_PER_GROUP: usize = 2;
const FALLBACK_CONFIDENCE: f64 = 0.38;

pub fn suggest_tags(track: &Track, groups: &[TagGroup]) -> Vec<TagSuggestion> {
    let signals = TrackSignals::from_track(track);
    let audio = cached_audio_features(track.file_path());
    let comment_hints = match_comment_hints(track, groups);
    let existing: std::collections::HashSet<_> = track.tag_ids.iter().cloned().collect();

    let mut out = Vec::new();

    for group in groups {
        let available: Vec<_> = group
            .tags
            .iter()
            .filter(|t| !existing.contains(&t.id))
            .collect();

        if available.is_empty() {
            continue;
        }

        let scored: Vec<TagSuggestion> = available
            .iter()
            .map(|tag| {
                let (confidence, reason) = if group.name == "Components" {
                    score_component_tag(&tag.name, &signals, &audio)
                } else {
                    score_tag(&group.name, &tag.name, &signals, &audio)
                };
                let (confidence, reason) = merge_with_comment_hint(
                    confidence,
                    reason,
                    &comment_hints,
                    &group.name,
                    &tag.name,
                );
                TagSuggestion {
                    track_id: track.id.clone(),
                    tag_id: tag.id.clone(),
                    tag_name: tag.name.clone(),
                    group_name: group.name.clone(),
                    confidence,
                    reason,
                    pending_create: false,
                }
            })
            .collect();

        let mut picked: Vec<TagSuggestion> = if group.name == "Components" {
            let scored_raw: Vec<(String, f64, String)> = scored
                .iter()
                .map(|s| (s.tag_name.clone(), s.confidence, s.reason.clone()))
                .collect();
            pick_components(scored_raw, MAX_PER_GROUP, MIN_CONFIDENCE)
                .into_iter()
                .filter_map(|(name, confidence, reason)| {
                    let tag = available.iter().find(|t| t.name == name)?;
                    Some(TagSuggestion {
                        track_id: track.id.clone(),
                        tag_id: tag.id.clone(),
                        tag_name: tag.name.clone(),
                        group_name: group.name.clone(),
                        confidence,
                        reason,
                        pending_create: false,
                    })
                })
                .collect()
        } else {
            let mut sorted = scored.clone();
            sorted.sort_by(|a, b| {
                b.confidence
                    .partial_cmp(&a.confidence)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
            let mut genre_picked: Vec<TagSuggestion> = sorted
                .into_iter()
                .filter(|s| s.confidence >= MIN_CONFIDENCE)
                .take(MAX_PER_GROUP)
                .collect();
            if group.name == "Genre" {
                genre_picked.retain(|s| should_offer_existing_genre_pick(&s.tag_name, &signals, groups));
                genre_picked = refine_genre_picks(genre_picked, &signals);
            }
            genre_picked
        };

        // Always include at least one tag per category
        if picked.is_empty() {
            if let Some(fallback) = category_fallback(&group.name, &available, &signals, &audio, track, groups) {
                if group.name != "Genre"
                    || should_offer_existing_genre_pick(&fallback.tag_name, &signals, groups)
                {
                    picked.push(fallback);
                }
            } else if let Some(best) = scored.iter().max_by(|a, b| {
                a.confidence
                    .partial_cmp(&b.confidence)
                    .unwrap_or(std::cmp::Ordering::Equal)
            }) {
                if group.name != "Genre"
                    || should_offer_existing_genre_pick(&best.tag_name, &signals, groups)
                {
                    picked.push(TagSuggestion {
                        confidence: best.confidence.max(FALLBACK_CONFIDENCE),
                        reason: if best.reason.is_empty() {
                            format!("Best available match for {}", group.name)
                        } else {
                            best.reason.clone()
                        },
                        pending_create: false,
                        ..best.clone()
                    });
                }
            }
        } else if picked.len() < MAX_PER_GROUP {
            // Room for a second tag in this category when scores support it.
        }

        out.extend(picked);
    }

    // Propose creating tags for Rekordbox genres not in the schema
    out.extend(unmapped::unmapped_genre_suggestions(track, groups, &out));
    out.extend(comment_genre_create_suggestions(track, groups, &out));
    out.extend(region_genre_suggestions(track, groups, &out));
    let subgenre = subgenre_genre_suggestions(track, groups, &out, &audio);
    if subgenre
        .iter()
        .any(|s| s.tag_name.eq_ignore_ascii_case("Classic House"))
    {
        out.retain(|s| {
            !(s.group_name == "Genre"
                && s.tag_name == "House"
                && s.confidence < 0.7
                && !s.pending_create)
        });
    }
    out.extend(subgenre);

    out.sort_by(|a, b| {
        b.confidence
            .partial_cmp(&a.confidence)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    out
}

pub fn suggest_for_library(tracks: &[Track], groups: &[TagGroup]) -> Vec<TagSuggestion> {
    tracks
        .iter()
        .flat_map(|t| suggest_tags(t, groups))
        .collect()
}

fn cached_audio_features(path: &str) -> AudioFeatures {
    let key = cache_key(path);
    if let Ok(cache) = AUDIO_CACHE.lock() {
        if let Some(f) = cache.get(&key) {
            return f.clone();
        }
    }
    let features = analyze_path(path);
    if let Ok(mut cache) = AUDIO_CACHE.lock() {
        cache.insert(key, features.clone());
    }
    features
}

fn cache_key(path: &str) -> String {
    let mtime = std::fs::metadata(path)
        .and_then(|m| m.modified())
        .map(|t| format!("{t:?}"))
        .unwrap_or_else(|_| "missing".into());
    format!("{path}|{mtime}")
}

fn score_tag(
    group: &str,
    tag_name: &str,
    signals: &TrackSignals,
    audio: &AudioFeatures,
) -> (f64, String) {
    match group {
        "Genre" => {
            let (mut g_conf, g_reason) = score_genre_tag(
                tag_name,
                &signals.corpus,
                &signals.genre_field,
                &signals.path_folders,
            );
            let (s_conf, s_reason) = score_subgenre_tag(tag_name, signals, audio);
            let (r_conf, r_reason) = score_region_genre_tag(tag_name, signals);

            if tag_name == "House" && classic_house_signals(signals) && s_conf < g_conf {
                g_conf *= 0.42;
            }

            let best = [(g_conf, g_reason), (s_conf, s_reason), (r_conf, r_reason)]
                .into_iter()
                .max_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal))
                .unwrap_or((0.0, String::new()));
            best
        }
        "Components" => score_component_tag(tag_name, signals, audio),
        "Situation" => score_situation_tag(tag_name, signals, audio),
        "Energy" => score_energy_tag(tag_name, signals, audio),
        _ => score_generic(tag_name, signals),
    }
}

/// Prefer specific sub-genres over generic House when both would be suggested.
fn refine_genre_picks(picked: Vec<TagSuggestion>, signals: &TrackSignals) -> Vec<TagSuggestion> {
    let has_specific_house = picked.iter().any(|s| {
        s.tag_name.ends_with(" House")
            && !s.tag_name.eq_ignore_ascii_case("House")
            && s.confidence >= MIN_CONFIDENCE
    });
    if !has_specific_house {
        return picked;
    }
    picked
        .into_iter()
        .filter(|s| {
            !(s.tag_name == "House"
                && classic_house_signals(signals)
                && s.confidence < 0.75)
        })
        .collect()
}

fn score_generic(tag: &str, s: &TrackSignals) -> (f64, String) {
    if s.word_match(tag) {
        return (0.7, format!("'{tag}' in metadata"));
    }
    (0.0, String::new())
}

fn genre_fallback(s: &TrackSignals) -> (&'static str, String) {
    let bpm = s.bpm;
    if bpm >= 170.0 {
        return ("DnB", format!("default: fast BPM ({bpm:.0}) → DnB"));
    }
    if bpm >= 118.0 && bpm <= 130.0 {
        return ("House", format!("default: BPM ({bpm:.0}) in house range"));
    }
    if bpm >= 130.0 && bpm <= 150.0 {
        return ("Techno", format!("default: BPM ({bpm:.0}) in techno range"));
    }
    ("Electronic", "default: general electronic".into())
}

fn find_tag<'a>(tags: &[&'a MyTagDef], name: &str) -> Option<&'a MyTagDef> {
    tags.iter().find(|t| t.name == name).copied()
}

fn find_tag_preferred<'a>(tags: &[&'a MyTagDef], preferred: &[&str]) -> Option<&'a MyTagDef> {
    for name in preferred {
        if let Some(t) = find_tag(tags, name) {
            return Some(t);
        }
    }
    tags.first().copied()
}

fn category_fallback(
    group_name: &str,
    available: &[&MyTagDef],
    signals: &TrackSignals,
    audio: &AudioFeatures,
    track: &Track,
    groups: &[TagGroup],
) -> Option<TagSuggestion> {
    let tag = match group_name {
        "Genre" => {
            if genre_field_needs_new_tag(signals, groups) {
                return None;
            }
            if !signals.genre_field.is_empty() {
                let field = signals.genre_field.trim();
                if let Some(tag) = find_tag(available, field) {
                    return Some(TagSuggestion {
                        track_id: track.id.clone(),
                        tag_id: tag.id.clone(),
                        tag_name: tag.name.clone(),
                        group_name: group_name.to_string(),
                        confidence: 0.9,
                        reason: format!("Rekordbox genre is '{field}'"),
                        pending_create: false,
                    });
                }
            }
            let (name, reason) = genre_fallback(signals);
            find_tag_preferred(available, &[name, "Electronic", "House"])
                .map(|t| (t, reason))
        }
        "Components" => {
            let (name, reason) = component_fallback(signals, audio);
            find_tag_preferred(
                available,
                &[
                    name,
                    "Vocal",
                    "Inst",
                    "Acap",
                    "Sub Bass",
                    "Synth",
                    "Beat",
                    "Percussion",
                    "Vocals",
                    "No-Vocals",
                ],
            )
            .map(|t| (t, reason))
        }
        "Situation" => {
            let (name, reason) = native_situation_fallback(signals, audio);
            find_tag_preferred(available, &[name, "Peak", "Early", "Late", "The Spot Event"])
                .map(|t| (t, reason))
        }
        "Energy" => {
            let (name, reason) = energy_fallback(signals, audio);
            find_tag_preferred(
                available,
                &[
                    name,
                    "Anthem",
                    "Banger",
                    "Fun and Up Beat",
                    "Chill",
                    "UnderGround",
                    "DJ Tools",
                ],
            )
            .map(|t| (t, reason))
        }
        _ => available.first().copied().map(|t| (t, format!("default pick for {group_name}"))),
    }?;

    Some(TagSuggestion {
        track_id: track.id.clone(),
        tag_id: tag.0.id.clone(),
        tag_name: tag.0.name.clone(),
        group_name: group_name.to_string(),
        confidence: FALLBACK_CONFIDENCE,
        reason: tag.1,
        pending_create: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{MyTagDef, TagGroup};

    fn genre_group() -> TagGroup {
        TagGroup {
            id: "g1".into(),
            name: "Genre".into(),
            seq: 1,
            tags: vec![MyTagDef {
                id: "t1".into(),
                name: "House".into(),
                group_id: "g1".into(),
                seq: 1,
            }],
        }
    }

    fn components_group() -> TagGroup {
        TagGroup {
            id: "g2".into(),
            name: "Components".into(),
            seq: 2,
            tags: vec![
                MyTagDef {
                    id: "t2a".into(),
                    name: "Vocal".into(),
                    group_id: "g2".into(),
                    seq: 1,
                },
                MyTagDef {
                    id: "t2b".into(),
                    name: "Inst".into(),
                    group_id: "g2".into(),
                    seq: 2,
                },
                MyTagDef {
                    id: "t2c".into(),
                    name: "Acap".into(),
                    group_id: "g2".into(),
                    seq: 3,
                },
            ],
        }
    }

    fn genre_group_full() -> TagGroup {
        TagGroup {
            id: "g1".into(),
            name: "Genre".into(),
            seq: 1,
            tags: vec![
                MyTagDef {
                    id: "t1a".into(),
                    name: "House".into(),
                    group_id: "g1".into(),
                    seq: 1,
                },
                MyTagDef {
                    id: "t1b".into(),
                    name: "Electronic".into(),
                    group_id: "g1".into(),
                    seq: 2,
                },
            ],
        }
    }

    fn situation_group() -> TagGroup {
        TagGroup {
            id: "g3".into(),
            name: "Situation".into(),
            seq: 3,
            tags: vec![
                MyTagDef {
                    id: "t3a".into(),
                    name: "Early".into(),
                    group_id: "g3".into(),
                    seq: 1,
                },
                MyTagDef {
                    id: "t3b".into(),
                    name: "Peak".into(),
                    group_id: "g3".into(),
                    seq: 2,
                },
                MyTagDef {
                    id: "t3c".into(),
                    name: "Late".into(),
                    group_id: "g3".into(),
                    seq: 3,
                },
            ],
        }
    }

    fn energy_group() -> TagGroup {
        TagGroup {
            id: "g4".into(),
            name: "Energy".into(),
            seq: 4,
            tags: vec![
                MyTagDef {
                    id: "t4a".into(),
                    name: "Anthem".into(),
                    group_id: "g4".into(),
                    seq: 1,
                },
                MyTagDef {
                    id: "t4b".into(),
                    name: "Banger".into(),
                    group_id: "g4".into(),
                    seq: 2,
                },
                MyTagDef {
                    id: "t4c".into(),
                    name: "Fun and Up Beat".into(),
                    group_id: "g4".into(),
                    seq: 3,
                },
            ],
        }
    }

    fn native_components_group() -> TagGroup {
        TagGroup {
            id: "g2".into(),
            name: "Components".into(),
            seq: 2,
            tags: vec![
                MyTagDef {
                    id: "c1".into(),
                    name: "Vocal".into(),
                    group_id: "g2".into(),
                    seq: 1,
                },
                MyTagDef {
                    id: "c2".into(),
                    name: "Inst".into(),
                    group_id: "g2".into(),
                    seq: 2,
                },
                MyTagDef {
                    id: "c3".into(),
                    name: "Piano".into(),
                    group_id: "g2".into(),
                    seq: 3,
                },
                MyTagDef {
                    id: "c4".into(),
                    name: "Synth".into(),
                    group_id: "g2".into(),
                    seq: 4,
                },
                MyTagDef {
                    id: "c5".into(),
                    name: "Beat".into(),
                    group_id: "g2".into(),
                    seq: 5,
                },
            ],
        }
    }

    fn minimal_track() -> Track {
        Track {
            id: "x".into(),
            title: "Unknown Track".into(),
            artist: "Unknown".into(),
            album: String::new(),
            genre: String::new(),
            bpm: 126.0,
            path: "/nonexistent.mp3".into(),
            rating: 0,
            comment: String::new(),
            tag_ids: vec![],
            ..Default::default()
        }
    }

    #[test]
    fn suggests_one_per_category_minimum() {
        let groups = vec![
            genre_group_full(),
            components_group(),
            situation_group(),
            energy_group(),
        ];
        let suggestions = suggest_tags(&minimal_track(), &groups);
        let categories: std::collections::HashSet<_> =
            suggestions.iter().map(|s| s.group_name.as_str()).collect();
        assert!(categories.contains("Genre"));
        assert!(categories.contains("Components"));
        assert!(categories.contains("Situation"));
        assert!(categories.contains("Energy"));
        assert!(suggestions.len() >= 4);
    }

    #[test]
    fn suggests_tags_from_comment_field() {
        let groups = vec![
            genre_group(),
            components_group(),
            situation_group(),
            energy_group(),
        ];
        let track = Track {
            id: "2".into(),
            title: "Talk To You".into(),
            artist: "ANOTR".into(),
            album: String::new(),
            genre: "House".into(),
            bpm: 126.0,
            path: "/nonexistent.mp3".into(),
            rating: 100,
            comment: "House Driving Vocals".into(),
            tag_ids: vec![],
            ..Default::default()
        };
        let suggestions = suggest_tags(&track, &groups);
        assert!(
            suggestions
                .iter()
                .any(|s| s.tag_name == "Vocal" && s.reason.contains("comments")),
            "expected Vocal from comment hints, got {:?}",
            suggestions
                .iter()
                .map(|s| (&s.tag_name, &s.reason))
                .collect::<Vec<_>>()
        );
        assert!(
            suggestions
                .iter()
                .any(|s| s.tag_name == "House" && s.reason.contains("comments")),
            "expected House from comment hints"
        );
    }

    #[test]
    fn suggests_house_from_genre_field() {
        let groups = vec![genre_group(), components_group()];
        let track = Track {
            id: "1".into(),
            title: "Talk To You".into(),
            artist: "ANOTR".into(),
            album: String::new(),
            genre: "House".into(),
            bpm: 126.0,
            path: "/nonexistent.mp3".into(),
            rating: 100,
            comment: "Driving Vocals".into(),
            tag_ids: vec![],
            ..Default::default()
        };
        let suggestions = suggest_tags(&track, &groups);
        assert!(suggestions.iter().any(|s| s.tag_name == "House"));
        assert!(suggestions.iter().any(|s| s.tag_name == "Vocal"));
    }

    #[test]
    fn suggests_acap_for_acapella_title() {
        let groups = vec![components_group()];
        let track = Track {
            id: "2".into(),
            title: "Au Revoir (Acapella)".into(),
            artist: "Test".into(),
            album: String::new(),
            genre: String::new(),
            bpm: 120.0,
            path: "/nonexistent.mp3".into(),
            rating: 0,
            comment: String::new(),
            tag_ids: vec![],
            ..Default::default()
        };
        let suggestions = suggest_tags(&track, &groups);
        assert!(suggestions.iter().any(|s| s.tag_name == "Acap"));
    }

    #[test]
    fn n_joi_anthem_suggests_peak_not_early_or_inst() {
        let groups = vec![
            genre_group(),
            native_components_group(),
            situation_group(),
            energy_group(),
        ];
        let track = Track {
            id: "anthem".into(),
            title: "Anthem (Original Mix)".into(),
            artist: "N-Joi".into(),
            album: String::new(),
            genre: "House".into(),
            bpm: 125.0,
            path: "/nonexistent.mp3".into(),
            rating: 0,
            comment: String::new(),
            tag_ids: vec![],
            ..Default::default()
        };
        let suggestions = suggest_tags(&track, &groups);
        let names: Vec<_> = suggestions.iter().map(|s| s.tag_name.as_str()).collect();
        assert!(names.contains(&"Anthem"), "expected Anthem, got {names:?}");
        assert!(suggestions.iter().any(|s| s.tag_name == "Peak"));
        assert!(
            suggestions
                .iter()
                .any(|s| s.tag_name == "Classic House" && s.pending_create),
            "expected Classic House new-tag suggestion, got {names:?}"
        );
        assert!(
            !names.contains(&"House"),
            "generic House should yield to Classic House, got {names:?}"
        );
    }

    #[test]
    fn alpine_air_suggests_rekordbox_genre_when_missing_from_schema() {
        let groups = vec![
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
            },
            components_group(),
            situation_group(),
            energy_group(),
        ];
        let track = Track {
            id: "alpine".into(),
            title: "Alpine Air (Original Mix)".into(),
            artist: "N1NJA".into(),
            album: String::new(),
            genre: "Melodic House & Techno".into(),
            bpm: 128.0,
            path: "/nonexistent.mp3".into(),
            rating: 0,
            comment: String::new(),
            tag_ids: vec![],
            ..Default::default()
        };
        let suggestions = suggest_tags(&track, &groups);
        assert!(
            suggestions.iter().any(|s| {
                s.group_name == "Genre"
                    && s.tag_name == "Melodic House & Techno"
                    && s.pending_create
            }),
            "expected Melodic House & Techno pending create, got {:?}",
            suggestions
                .iter()
                .map(|s| (&s.group_name, &s.tag_name, s.pending_create))
                .collect::<Vec<_>>()
        );
        assert!(
            !suggestions.iter().any(|s| s.tag_name == "House" && !s.pending_create),
            "should not suggest generic House when genre field is more specific"
        );
    }
}
