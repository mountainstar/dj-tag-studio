use std::fs::File;
use std::path::Path;

use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::{DecoderOptions, CODEC_TYPE_NULL};
use symphonia::core::errors::Error;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

/// Lightweight audio features from ~20s sample (middle of track).
#[derive(Debug, Clone, Default)]
pub struct AudioFeatures {
    pub analyzed: bool,
    pub rms: f64,
    pub bass_ratio: f64,
    pub vocal_ratio: f64,
    pub brightness: f64,
    pub onset_density: f64,
}

const SAMPLE_RATE: f64 = 22050.0;
const ANALYZE_SECONDS: f64 = 20.0;
const FFT_SIZE: usize = 2048;

pub fn analyze_path(path: &str) -> AudioFeatures {
    let p = Path::new(path);
    if !p.exists() {
        return AudioFeatures::default();
    }
    analyze_file(p).unwrap_or_default()
}

fn analyze_file(path: &Path) -> Result<AudioFeatures, Box<dyn std::error::Error>> {
    let src = File::open(path)?;
    let mss = MediaSourceStream::new(Box::new(src), Default::default());
    let mut hint = Hint::new();
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        hint.with_extension(ext);
    }

    let probed = symphonia::default::get_probe().format(
        &hint,
        mss,
        &FormatOptions::default(),
        &MetadataOptions::default(),
    )?;
    let mut format = probed.format;
    let track = format
        .tracks()
        .iter()
        .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
        .ok_or("no audio track")?;

    let track_id = track.id;
    let mut decoder = symphonia::default::get_codecs().make(
        &track.codec_params,
        &DecoderOptions::default(),
    )?;

    let total_duration = track
        .codec_params
        .n_frames
        .and_then(|n| track.codec_params.sample_rate.map(|sr| n as f64 / sr as f64))
        .unwrap_or(180.0);

    let skip_seconds = (total_duration * 0.25).min(total_duration.max(0.0) - ANALYZE_SECONDS).max(0.0);
    let target_samples = (ANALYZE_SECONDS * SAMPLE_RATE) as usize;
    let mut mono: Vec<f32> = Vec::with_capacity(target_samples);
    let skip_samples = (skip_seconds * SAMPLE_RATE) as u64;
    let mut seen = 0u64;

    while mono.len() < target_samples {
        let packet = match format.next_packet() {
            Ok(p) => p,
            Err(Error::ResetRequired) => continue,
            Err(_) => break,
        };

        if packet.track_id() != track_id {
            continue;
        }

        let decoded = decoder.decode(&packet)?;
        let spec = *decoded.spec();
        let duration = decoded.capacity() as u64;

        if seen + duration <= skip_samples {
            seen += duration;
            continue;
        }

        let mut buf = SampleBuffer::<f32>::new(duration, spec);
        buf.copy_interleaved_ref(decoded);
        let channels = spec.channels.count();
        let samples = buf.samples();

        for frame in samples.chunks(channels) {
            if mono.len() >= target_samples {
                break;
            }
            let sum: f32 = frame.iter().sum();
            mono.push(sum / channels as f32);
        }
    }

    if mono.len() < FFT_SIZE * 2 {
        return Ok(AudioFeatures::default());
    }

    Ok(compute_features(&mono))
}

fn compute_features(samples: &[f32]) -> AudioFeatures {
    let rms = (samples.iter().map(|s| (*s as f64).powi(2)).sum::<f64>() / samples.len() as f64).sqrt();

    let mut planner = realfft::RealFftPlanner::<f32>::new();
    let r2c = planner.plan_fft_forward(FFT_SIZE);
    let mut scratch = r2c.make_scratch_vec();

    let mut bass_energy = 0.0f64;
    let mut vocal_energy = 0.0f64;
    let mut high_energy = 0.0f64;
    let mut total_energy = 0.0f64;
    let mut prev_mag = vec![0.0f32; FFT_SIZE / 2 + 1];
    let mut flux_sum = 0.0f64;
    let mut flux_count = 0usize;

    for window in samples.chunks(FFT_SIZE).take(64) {
        if window.len() < FFT_SIZE {
            break;
        }
        let mut input: Vec<f32> = window.to_vec();
        let mut output = r2c.make_output_vec();
        r2c.process_with_scratch(&mut input, &mut output, &mut scratch)
            .ok();

        for (i, c) in output.iter().enumerate().take(FFT_SIZE / 2 + 1) {
            let mag = (c.re * c.re + c.im * c.im).sqrt() as f64;
            let freq = i as f64 * SAMPLE_RATE / FFT_SIZE as f64;
            total_energy += mag;
            if freq < 150.0 {
                bass_energy += mag;
            }
            if (300.0..3400.0).contains(&freq) {
                vocal_energy += mag;
            }
            if freq > 4000.0 {
                high_energy += mag;
            }
            let flux = (mag as f32 - prev_mag[i]).max(0.0);
            flux_sum += flux as f64;
            prev_mag[i] = mag as f32;
        }
        flux_count += 1;
    }

    let bass_ratio = if total_energy > 0.0 { bass_energy / total_energy } else { 0.0 };
    let vocal_ratio = if total_energy > 0.0 { vocal_energy / total_energy } else { 0.0 };
    let brightness = if total_energy > 0.0 { high_energy / total_energy } else { 0.0 };
    let onset_density = if flux_count > 0 { flux_sum / flux_count as f64 } else { 0.0 };

    AudioFeatures {
        analyzed: true,
        rms,
        bass_ratio,
        vocal_ratio,
        brightness,
        onset_density,
    }
}
