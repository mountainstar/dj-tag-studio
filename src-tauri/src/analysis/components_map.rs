use super::audio::AudioFeatures;
use super::signals::TrackSignals;

/// Score native Rekordbox Components tags (Vocal, Inst, Sub Bass, …).
pub fn score_component_tag(
    tag_name: &str,
    signals: &TrackSignals,
    audio: &AudioFeatures,
) -> (f64, String) {
    let normalized = tag_name.to_ascii_lowercase();
    match normalized.as_str() {
        "vocal" | "vocals" => score_vocal(signals, audio),
        "inst" | "no-vocals" | "no vocals" => score_inst(signals, audio),
        "acap" => score_acap(signals, audio),
        "sub bass" | "bass-heavy" => score_sub_bass(signals, audio),
        "piano" => score_piano(signals, audio),
        "horn/sax" | "horns" => score_horns(signals),
        "synth" => score_synth(signals, audio),
        "percussion" => score_percussion(signals, audio),
        "beat" => score_beat(signals, audio),
        "dark" => score_dark(signals, audio),
        "upper" => score_upper(signals, audio),
        _ => (0.0, String::new()),
    }
}

pub fn component_fallback(signals: &TrackSignals, audio: &AudioFeatures) -> (&'static str, String) {
    if let Some((name, reason)) = detect_vocal_vs_inst(signals, audio) {
        return (name, reason);
    }

    if audio.analyzed && audio.bass_ratio > 0.22 {
        return (
            "Sub Bass",
            format!("default: strong sub-bass ({:.0}%)", audio.bass_ratio * 100.0),
        );
    }

    if audio.analyzed && audio.onset_density > 0.012 {
        return ("Beat", "default: prominent drum/transient pattern".into());
    }

    if signals.corpus.contains("house")
        || signals.corpus.contains("techno")
        || signals.corpus.contains("electronic")
    {
        return ("Beat", "default: dancefloor rhythm".into());
    }

    ("Vocal", "default: assume vocals unless audio/metadata say otherwise".into())
}

/// Pick up to two non-conflicting component tags from scored list.
pub fn pick_components(
    mut scored: Vec<(String, f64, String)>,
    max: usize,
    min_confidence: f64,
) -> Vec<(String, f64, String)> {
    scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    let mut picked = Vec::new();
    for (name, conf, reason) in scored {
        if conf < min_confidence {
            continue;
        }
        if conflicts_with_picked(&name, &picked) {
            continue;
        }
        picked.push((name, conf, reason));
        if picked.len() >= max {
            break;
        }
    }
    picked
}

fn conflicts_with_picked(name: &str, picked: &[(String, f64, String)]) -> bool {
    let n = name.to_ascii_lowercase();
    for (existing, _, _) in picked {
        let e = existing.to_ascii_lowercase();
        if (n == "vocal" && (e == "inst" || e == "acap"))
            || ((n == "inst" || n == "acap") && e == "vocal")
        {
            return true;
        }
    }
    false
}

fn detect_vocal_vs_inst(
    signals: &TrackSignals,
    audio: &AudioFeatures,
) -> Option<(&'static str, String)> {
    let (v_conf, v_reason) = score_vocal(signals, audio);
    let (i_conf, i_reason) = score_inst(signals, audio);

    if v_conf >= 0.65 && v_conf > i_conf + 0.12 {
        return Some(("Vocal", v_reason));
    }
    if i_conf >= 0.65 && i_conf > v_conf + 0.12 {
        return Some(("Inst", i_reason));
    }
    None
}

fn score_vocal(signals: &TrackSignals, audio: &AudioFeatures) -> (f64, String) {
    if let Some(kw) = signals.contains_any(&[
        "vocal",
        "vocals",
        "acapella",
        "a cappella",
        "singing",
        "chant",
        "sample",
        "samples",
        "feat.",
        "ft.",
        "feat ",
        " ft ",
        "hook",
        "verse",
        "chorus",
    ]) {
        return (0.9, format!("metadata keyword '{kw}'"));
    }
    if signals.title_has_vocal_credits() {
        return (0.86, "featuring / vocalist credit in title".into());
    }
    if audio.analyzed && audio.vocal_ratio > 0.34 {
        return (
            0.84,
            format!("vocal band energy {:.0}%", audio.vocal_ratio * 100.0),
        );
    }
    if audio.analyzed && audio.vocal_ratio > 0.26 {
        return (
            0.68,
            format!("moderate vocal presence ({:.0}%)", audio.vocal_ratio * 100.0),
        );
    }
    (0.0, String::new())
}

fn score_inst(signals: &TrackSignals, audio: &AudioFeatures) -> (f64, String) {
    if signals.is_instrumental_version() {
        return (0.88, "instrumental version in metadata".into());
    }
    if signals.contains_any(&[
        "instrumental",
        "inst.",
        " no vocal",
        "no-vocal",
        "no vocal",
        "dub mix",
        "dub version",
    ]).is_some()
    {
        return (0.88, "instrumental / dub in metadata".into());
    }
    if audio.analyzed && audio.vocal_ratio < 0.14 {
        return (
            0.86,
            format!("instrumental spectrum ({:.0}% vocal band)", audio.vocal_ratio * 100.0),
        );
    }
    if audio.analyzed && audio.vocal_ratio < 0.2 {
        return (
            0.72,
            format!("low vocal band ({:.0}%)", audio.vocal_ratio * 100.0),
        );
    }
    (0.0, String::new())
}

fn score_acap(signals: &TrackSignals, audio: &AudioFeatures) -> (f64, String) {
    if signals.contains_any(&["acapella", "a cappella", "acap"]).is_some() {
        return (0.95, "acapella in metadata".into());
    }
    if audio.analyzed && audio.vocal_ratio > 0.42 && audio.bass_ratio < 0.12 {
        return (
            0.78,
            format!(
                "voice-dominant, minimal bass ({:.0}% vocal)",
                audio.vocal_ratio * 100.0
            ),
        );
    }
    (0.0, String::new())
}

fn score_sub_bass(signals: &TrackSignals, audio: &AudioFeatures) -> (f64, String) {
    if signals.contains_any(&[
        "sub bass",
        "sub-bass",
        "808",
        "bassline",
        "dnb",
        "drum and bass",
        "drum & bass",
        "jungle",
    ]).is_some()
    {
        return (0.88, "bass-heavy keywords".into());
    }
    if audio.analyzed && audio.bass_ratio > 0.26 {
        return (
            0.84,
            format!("strong sub-bass ({:.0}%)", audio.bass_ratio * 100.0),
        );
    }
    if audio.analyzed && audio.bass_ratio > 0.2 {
        return (
            0.68,
            format!("elevated sub-bass ({:.0}%)", audio.bass_ratio * 100.0),
        );
    }
    (0.0, String::new())
}

fn score_piano(signals: &TrackSignals, audio: &AudioFeatures) -> (f64, String) {
    if let Some(kw) = signals.contains_any(&[
        "piano",
        "keys",
        "rhodes",
        "organ",
        "keyboard",
        "wurlitzer",
        "clav",
    ]) {
        return (0.9, format!("keyword '{kw}'"));
    }
    // Piano-house / stabby keys without explicit metadata
    if audio.analyzed
        && audio.brightness > 0.15
        && audio.onset_density > 0.01
        && audio.vocal_ratio > 0.1
        && audio.vocal_ratio < 0.4
    {
        return (
            0.68,
            format!(
                "bright keyed stabs ({:.0}% brightness)",
                audio.brightness * 100.0
            ),
        );
    }
    (0.0, String::new())
}

fn score_horns(signals: &TrackSignals) -> (f64, String) {
    if let Some(kw) = signals.contains_any(&[
        "horn",
        "horns",
        "brass",
        "trumpet",
        "sax",
        "saxophone",
        "trombone",
    ]) {
        return (0.9, format!("keyword '{kw}'"));
    }
    (0.0, String::new())
}

fn score_synth(signals: &TrackSignals, audio: &AudioFeatures) -> (f64, String) {
    if signals.contains_any(&[
        "synth",
        "synthesizer",
        "analog",
        "modular",
        "acid",
    ]).is_some()
    {
        return (0.86, "synth keywords".into());
    }
    if audio.analyzed && audio.brightness > 0.22 && audio.vocal_ratio < 0.18 {
        return (
            0.68,
            format!("bright synth-like spectrum ({:.0}%)", audio.brightness * 100.0),
        );
    }
    (0.0, String::new())
}

fn score_percussion(signals: &TrackSignals, audio: &AudioFeatures) -> (f64, String) {
    if signals.contains_any(&[
        "percussion",
        "percussive",
        "tribal",
        "conga",
        "bongo",
        "shaker",
        "latin percussion",
    ]).is_some()
    {
        return (0.86, "percussion keywords".into());
    }
    if audio.analyzed && audio.onset_density > 0.018 {
        return (0.74, "high transient / percussion density".into());
    }
    (0.0, String::new())
}

fn score_beat(signals: &TrackSignals, audio: &AudioFeatures) -> (f64, String) {
    if signals.contains_any(&["drum", "drums", "kick", "breakbeat", "breaks"]).is_some() {
        return (0.82, "drum keywords".into());
    }
    if audio.analyzed && audio.onset_density > 0.014 && audio.bass_ratio > 0.14 {
        return (0.76, "strong kick/transient pattern".into());
    }
    (0.0, String::new())
}

fn score_dark(signals: &TrackSignals, audio: &AudioFeatures) -> (f64, String) {
    if signals.contains_any(&["dark", "minimal", "hypnotic", "industrial", "dub techno"]).is_some() {
        return (0.84, "dark / minimal keywords".into());
    }
    if audio.analyzed && audio.brightness < 0.12 && audio.rms < 0.12 {
        return (0.72, "dark, low-brightness spectrum".into());
    }
    (0.0, String::new())
}

fn score_upper(signals: &TrackSignals, audio: &AudioFeatures) -> (f64, String) {
    if signals.contains_any(&["hi-hat", "hihat", "shaker", "top line", "upper"]).is_some() {
        return (0.82, "upper / hat keywords".into());
    }
    if audio.analyzed && audio.brightness > 0.22 && audio.onset_density > 0.012 {
        return (
            0.7,
            format!("bright top-end ({:.0}%)", audio.brightness * 100.0),
        );
    }
    (0.0, String::new())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analysis::audio::AudioFeatures;

    fn signals_with(corpus: &str) -> TrackSignals {
        TrackSignals {
            corpus: corpus.to_string(),
            ..Default::default()
        }
    }

    #[test]
    fn scores_native_vocal_from_acapella() {
        let s = signals_with("au revoir acapella radio edit");
        let (conf, _) = score_component_tag("Acap", &s, &AudioFeatures::default());
        assert!(conf > 0.9);
    }

    #[test]
    fn vocal_and_inst_conflict() {
        let scored = vec![
            ("Vocal".into(), 0.9, "vocal".into()),
            ("Inst".into(), 0.85, "inst".into()),
            ("Sub Bass".into(), 0.7, "bass".into()),
        ];
        let picked = pick_components(scored, 2, 0.5);
        assert_eq!(picked.len(), 2);
        assert!(picked.iter().any(|(n, _, _)| n == "Vocal"));
        assert!(!picked.iter().any(|(n, _, _)| n == "Inst"));
    }
}
