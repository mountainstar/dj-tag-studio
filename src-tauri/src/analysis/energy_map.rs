use super::audio::AudioFeatures;
use super::signals::TrackSignals;

/// Score native Rekordbox Energy tags.
pub fn score_energy_tag(
    tag_name: &str,
    signals: &TrackSignals,
    audio: &AudioFeatures,
) -> (f64, String) {
    let bpm = signals.bpm;
    let loudness = normalize_loudness(audio.rms);
    let combined = energy_index(bpm, loudness);

    match tag_name {
        "Anthem" => score_anthem(signals, audio, bpm, loudness, combined),
        "Banger" => score_banger(signals, bpm, loudness, combined),
        "Chill" => score_chill(signals, bpm, loudness, combined),
        "Fun and Up Beat" => score_fun_upbeat(signals, audio, bpm, loudness, combined),
        "UnderGround" => score_underground(signals, audio, bpm),
        "DJ Tools" => score_dj_tools(signals),
        "B Side" => score_b_side(signals),
        "My Favs" => score_my_favs(signals),
        // Legacy arc buckets
        "Start" | "Build" => score_chill(signals, bpm, loudness, combined),
        "Peak" | "Sustain" => score_banger(signals, bpm, loudness, combined),
        "Release" => score_chill(signals, bpm, loudness, combined),
        _ => (0.0, String::new()),
    }
}

pub fn energy_fallback(signals: &TrackSignals, audio: &AudioFeatures) -> (&'static str, String) {
    if signals.word_match("anthem") {
        return ("Anthem", "default: anthem in title".into());
    }
    let loudness = normalize_loudness(audio.rms);
    let combined = energy_index(signals.bpm, loudness);

    if combined < 0.35 {
        return ("Chill", format!("default: low energy index ({combined:.0}%)"));
    }
    if combined > 0.62 {
        return ("Banger", format!("default: high energy ({combined:.0}%)"));
    }
    if signals.bpm >= 118.0 && signals.bpm <= 130.0 {
        return (
            "Fun and Up Beat",
            format!("default: upbeat house BPM ({:.0})", signals.bpm),
        );
    }
    ("Fun and Up Beat", "default: general dancefloor energy".into())
}

fn score_anthem(
    signals: &TrackSignals,
    audio: &AudioFeatures,
    bpm: f64,
    loudness: f64,
    combined: f64,
) -> (f64, String) {
    if signals.word_match("anthem") {
        return (0.96, "anthem in title/metadata".into());
    }
    if signals.contains_any(&["anthem", "singalong", "hands up", "hands-up"]).is_some() {
        return (0.92, "anthem keywords".into());
    }
    if bpm >= 118.0 && bpm <= 132.0 && loudness > 0.5 && combined > 0.55 {
        let vocal_hint = if audio.analyzed && audio.vocal_ratio > 0.14 {
            " + vocal hooks"
        } else {
            ""
        };
        return (
            0.78,
            format!("peak-time crowd moment ({bpm:.0} BPM, loud{vocal_hint})"),
        );
    }
    (0.0, String::new())
}

fn score_banger(signals: &TrackSignals, bpm: f64, loudness: f64, combined: f64) -> (f64, String) {
    if signals.contains_any(&["banger", "smasher", "weapon", "fire"]).is_some() {
        return (0.92, "banger keywords".into());
    }
    if bpm >= 122.0 && loudness > 0.58 && combined > 0.58 {
        return (
            0.82,
            format!("high BPM + loud ({bpm:.0}, {:.0}%)", loudness * 100.0),
        );
    }
    if combined > 0.65 {
        return (0.72, format!("strong energy index ({combined:.0}%)"));
    }
    (0.0, String::new())
}

fn score_chill(signals: &TrackSignals, bpm: f64, loudness: f64, combined: f64) -> (f64, String) {
    if signals.contains_any(&["chill", "lounge", "downtempo", "ambient", "slow"]).is_some() {
        return (0.9, "chill keywords".into());
    }
    if bpm > 0.0 && bpm < 108.0 {
        return (0.8, format!("slow BPM ({bpm:.0})"));
    }
    if combined < 0.32 || loudness < 0.35 {
        return (0.74, format!("low energy ({combined:.0}%)"));
    }
    (0.0, String::new())
}

fn score_fun_upbeat(
    signals: &TrackSignals,
    audio: &AudioFeatures,
    bpm: f64,
    _loudness: f64,
    combined: f64,
) -> (f64, String) {
    if signals.contains_any(&[
        "funky",
        "fun",
        "upbeat",
        "feel good",
        "feel-good",
        "disco",
        "nu disco",
        "piano",
    ]).is_some()
    {
        return (0.86, "upbeat / fun keywords".into());
    }
    if bpm >= 118.0 && bpm <= 128.0 && combined >= 0.42 && combined <= 0.72 {
        return (0.72, format!("upbeat house-range BPM ({bpm:.0})"));
    }
    if audio.analyzed && audio.brightness > 0.16 && bpm >= 118.0 && bpm <= 130.0 {
        return (0.66, "bright, uplifting dancefloor energy".into());
    }
    (0.0, String::new())
}

fn score_underground(signals: &TrackSignals, audio: &AudioFeatures, bpm: f64) -> (f64, String) {
    if signals.contains_any(&[
        "underground",
        "minimal",
        "dub",
        "hypnotic",
        "industrial",
        "raw",
        "deep",
    ]).is_some()
    {
        return (0.88, "underground keywords".into());
    }
    if audio.analyzed && audio.brightness < 0.12 && audio.rms < 0.1 {
        return (0.72, "dark, low-brightness spectrum".into());
    }
    if bpm >= 128.0 && bpm <= 140.0 && signals.corpus.contains("techno") {
        return (0.68, "techno-range underground".into());
    }
    (0.0, String::new())
}

fn score_dj_tools(signals: &TrackSignals) -> (f64, String) {
    if signals.contains_any(&[
        "tool",
        "tools",
        "loop",
        "loops",
        "sample pack",
        "dj tool",
        "edit tool",
        "intro edit",
        "outro",
        "strip",
    ]).is_some()
    {
        return (0.9, "DJ tool keywords".into());
    }
    (0.0, String::new())
}

fn score_b_side(signals: &TrackSignals) -> (f64, String) {
    if signals.contains_any(&["b side", "b-side", "flip", "bonus"]).is_some() {
        return (0.88, "b-side keywords".into());
    }
    (0.0, String::new())
}

fn score_my_favs(signals: &TrackSignals) -> (f64, String) {
    // Rekordbox rating is often 0–255 (5 stars ≈ 255)
    if signals.rating >= 200 {
        return (0.85, format!("high library rating ({})", signals.rating));
    }
    if signals.rating >= 150 {
        return (0.68, format!("rated track ({})", signals.rating));
    }
    (0.0, String::new())
}

fn normalize_loudness(rms: f64) -> f64 {
    if rms <= 0.0 {
        return 0.45;
    }
    ((rms - 0.02) / 0.22).clamp(0.0, 1.0)
}

fn energy_index(bpm: f64, loudness: f64) -> f64 {
    let bpm_energy = if bpm <= 0.0 {
        0.5
    } else {
        ((bpm - 80.0) / 80.0).clamp(0.0, 1.0)
    };
    bpm_energy * 0.55 + loudness * 0.45
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn anthem_title_scores_high() {
        let signals = TrackSignals {
            corpus: "anthem original mix n-joi".into(),
            raw_title: "Anthem (Original Mix)".into(),
            bpm: 125.0,
            ..Default::default()
        };
        let (conf, _) = score_energy_tag("Anthem", &signals, &AudioFeatures::default());
        assert!(conf > 0.9);
    }
}
