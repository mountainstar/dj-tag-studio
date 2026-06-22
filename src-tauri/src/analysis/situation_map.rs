use super::audio::AudioFeatures;
use super::signals::TrackSignals;

/// Score native Rekordbox Situation tags: Early, Late, Peak, The Spot Event.
pub fn score_situation_tag(
    tag_name: &str,
    signals: &TrackSignals,
    audio: &AudioFeatures,
) -> (f64, String) {
    let bpm = signals.bpm;
    let loud = audio.rms;

    match tag_name {
        "Early" => score_early(signals, bpm, loud),
        "Peak" => score_peak(signals, audio, bpm, loud),
        "Late" => score_late(signals, bpm, loud, audio),
        "The Spot Event" => score_spot_event(signals),
        // Legacy pack names — map if they ever appear
        "Warm-Up" | "Opening-Set" => score_early(signals, bpm, loud),
        "Peak-Time" => score_peak(signals, audio, bpm, loud),
        "Closing" | "After-Hours" => score_late(signals, bpm, loud, audio),
        "Wedding" | "Corporate" => score_spot_event(signals),
        _ => (0.0, String::new()),
    }
}

pub fn situation_fallback(signals: &TrackSignals, audio: &AudioFeatures) -> (&'static str, String) {
    let bpm = signals.bpm;
    let loud = audio.rms;

    if signals.contains_any(&["wedding", "corporate", "the spot", "spot event"]).is_some() {
        return ("The Spot Event", "default: event keywords".into());
    }
    if bpm > 0.0 && bpm < 112.0 {
        return ("Early", format!("default: slow/opening BPM ({bpm:.0})"));
    }
    if bpm >= 112.0 && bpm <= 134.0 {
        return ("Peak", format!("default: main-room BPM ({bpm:.0})"));
    }
    if bpm > 134.0 || loud < 0.08 {
        return ("Late", format!("default: late-set BPM/loudness ({bpm:.0})"));
    }
    ("Peak", "default: general peak-time placement".into())
}

fn score_early(signals: &TrackSignals, bpm: f64, loud: f64) -> (f64, String) {
    if signals.contains_any(&[
        "warm up",
        "warm-up",
        "warmup",
        "opening",
        "open set",
        "first hour",
        "early",
        "chill",
        "lounge",
    ]).is_some()
    {
        return (0.88, "opening / warm-up keywords".into());
    }
    if bpm > 0.0 && bpm < 110.0 {
        return (0.82, format!("slow BPM ({bpm:.0})"));
    }
    if bpm > 0.0 && bpm < 118.0 && loud < 0.09 {
        return (0.74, format!("mid BPM + low loudness ({bpm:.0})"));
    }
    (0.0, String::new())
}

fn score_peak(signals: &TrackSignals, audio: &AudioFeatures, bpm: f64, loud: f64) -> (f64, String) {
    if signals.word_match("anthem") {
        return (0.92, "anthem track → peak-time moment".into());
    }
    if signals.contains_any(&[
        "peak",
        "peak time",
        "peak-time",
        "main room",
        "prime time",
        "banger",
        "anthem",
    ]).is_some()
    {
        return (0.9, "peak-time keywords".into());
    }
    if bpm >= 120.0 && bpm <= 134.0 && loud > 0.08 {
        return (0.84, format!("peak BPM + energy ({bpm:.0})"));
    }
    if bpm >= 118.0 && bpm <= 136.0 {
        return (0.68, format!("peak-range BPM ({bpm:.0})"));
    }
    if audio.analyzed && loud > 0.1 && bpm >= 118.0 {
        return (0.62, format!("loud dancefloor energy ({bpm:.0} BPM)"));
    }
    (0.0, String::new())
}

fn score_late(signals: &TrackSignals, bpm: f64, loud: f64, audio: &AudioFeatures) -> (f64, String) {
    if signals.contains_any(&[
        "late",
        "after hours",
        "after-hours",
        "late night",
        "closing",
        "close out",
        "close-out",
        "wind down",
        "last hour",
    ]).is_some()
    {
        return (0.9, "late-set keywords".into());
    }
    if bpm > 0.0 && bpm < 105.0 {
        return (0.78, format!("closing BPM ({bpm:.0})"));
    }
    if bpm >= 118.0 && bpm <= 126.0 && loud < 0.09 && audio.analyzed {
        return (0.66, "deep late-set BPM/loudness".into());
    }
    (0.0, String::new())
}

fn score_spot_event(signals: &TrackSignals) -> (f64, String) {
    if signals.contains_any(&[
        "wedding",
        "reception",
        "corporate",
        "gala",
        "the spot",
        "spot event",
        "wirth_it",
        "wirth it",
    ]).is_some()
    {
        return (0.9, "event / special-set keywords".into());
    }
    (0.0, String::new())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn peak_for_anthem_title_at_125_bpm() {
        let signals = TrackSignals {
            corpus: "anthem original mix n-joi house".into(),
            bpm: 125.0,
            ..Default::default()
        };
        let (conf, _) = score_situation_tag("Peak", &signals, &AudioFeatures::default());
        assert!(conf > 0.85);
    }

    #[test]
    fn early_not_default_for_peak_bpm() {
        let signals = TrackSignals {
            bpm: 125.0,
            ..Default::default()
        };
        let (conf, _) = score_situation_tag("Early", &signals, &AudioFeatures::default());
        assert!(conf < 0.5);
    }
}
