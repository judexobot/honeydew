//! Lightweight offline tempo estimation for imported audio clips.
//!
//! Works on already-decoded PCM, never in the real-time callback. No external DSP/FFI
//! dependency; the bounded analysis is usable on Android, desktop and WASM. A result
//! is an *estimate*, never proof of an exact musical tempo.

use crate::sample::SampleData;

/// Estimated constant tempo and a heuristic rhythmic-consistency score (0..=1).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TempoEstimate {
    pub bpm: f32,
    pub confidence: f32,
}

/// Analysis rate in onset-envelope frames per second.
const HZ: f64 = 200.0;
/// Limit long files so importing a song does not scan the whole recording.
const MAX_SECONDS: f64 = 60.0;
const MIN_SECONDS: f64 = 3.0;
const MIN_BPM: f64 = 55.0;
const MAX_BPM: f64 = 220.0;

/// Estimate a stable pulse in `sample`, or `None` for silence, one-shots,
/// nonrhythmic material and weak or ambiguous evidence.
///
/// An envelope of positive RMS-energy changes emphasizes transients. A
/// normalized autocorrelation finds periodicities; fractional-lag refinement
/// reduces the 200 Hz grid's BPM quantization. Half/double-time remain
/// musically ambiguous, so the confidence is *not* a probability of correctness.
pub fn detect(sample: &SampleData) -> Option<TempoEstimate> {
    let rate = sample.sample_rate as usize;
    if rate == 0 || sample.frames() < (rate as f64 * MIN_SECONDS) as usize {
        return None;
    }
    let hop = ((rate as f64 / HZ).round() as usize).max(1);
    let len = sample
        .frames()
        .min((rate as f64 * MAX_SECONDS) as usize);
    let mut rms = Vec::with_capacity(len / hop + 1);
    let mut max_level = 0.0f64;
    for start in (0..len).step_by(hop) {
        let end = (start + hop).min(len);
        let mut power = 0.0f64;
        for i in start..end {
            let l = sample.left[i];
            let r = sample.right.as_ref().map_or(l, |right| right[i]);
            if !l.is_finite() || !r.is_finite() {
                continue;
            }
            let l = f64::from(l);
            let r = f64::from(r);
            power += (l * l + r * r) * 0.5;
        }
        let level = (power / (end - start) as f64).sqrt();
        max_level = max_level.max(level);
        rms.push(level);
    }
    if max_level < 1e-5 || rms.len() < 300 {
        return None;
    }

    // The two-frame lookback captures attack slopes while attenuating sustained
    // notes. Normalize relative to the peak so a quiet file behaves like a loud one.
    let mut onsets = vec![0.0f64; rms.len()];
    let mut max_onset = 0.0f64;
    for i in 2..rms.len() {
        let value = (rms[i] - (rms[i - 1] + rms[i - 2]) * 0.5).max(0.0);
        onsets[i] = value;
        max_onset = max_onset.max(value);
    }
    if max_onset < max_level * 0.005 {
        return None;
    }
    // Reject low-level fluctuations, which otherwise give a noise-like source
    // positive correlations even when it contains no discernible beat.
    let floor = max_onset * 0.07;
    for value in &mut onsets {
        *value = (*value - floor).max(0.0);
    }
    let active = onsets.iter().filter(|&&v| v > max_onset * 0.18).count();
    if active < 4 {
        return None;
    }

    let actual_hz = rate as f64 / hop as f64;
    let min_lag = (actual_hz * 60.0 / MAX_BPM).ceil() as usize;
    let max_lag = (actual_hz * 60.0 / MIN_BPM).floor() as usize;
    if max_lag >= onsets.len() / 2 {
        return None;
    }

    // Each lag compares only overlapping samples, normalized by both energies:
    // no artificial preference for the short lag's larger overlap count.
    let mut best_lag = 0usize;
    let mut best_rank = 0.0f64;
    let mut best_correlation = 0.0f64;
    for lag in min_lag..=max_lag {
        let correlation = correlate(&onsets, lag as f64);
        let bpm = actual_hz * 60.0 / lag as f64;
        // A very small preference resolves octave ties toward the usual musical
        // beat range. This must not overrule a substantially better correlation.
        let preference = if (110.0..=210.0).contains(&bpm) {
            1.05
        } else {
            1.0
        };
        let rank = correlation * preference;
        if rank > best_rank {
            best_lag = lag;
            best_rank = rank;
            best_correlation = correlation;
        }
    }
    if best_lag == 0 || best_correlation < 0.20 {
        return None;
    }

    let mut refined = best_lag as f64;
    let mut refined_score = best_correlation;
    // Linear interpolation between onset bins approximates continuous
    // autocorrelation; use 1/8 bin steps around the integer maximum.
    for step in -8..=8 {
        let lag = best_lag as f64 + step as f64 * 0.125;
        if lag <= 0.0 {
            continue;
        }
        let correlation = correlate(&onsets, lag);
        if correlation > refined_score {
            refined = lag;
            refined_score = correlation;
        }
    }
    // Distinguish a real beat from its half-time autocorrelation peak.
    // A rigid click track repeats every beat AND every two beats; in such
    // material the two-beat peak can be slightly cleaner because consecutive
    // attacks cross the 200 Hz analysis grid at different fractions. Prefer
    // the shorter period only when its own correlation is also strong.
    // Crucially, a real 64-BPM track with no intervening 128-BPM attacks
    // has a weak half-lag correlation and stays at 64 BPM.
    let faster_lag = refined * 0.5;
    if faster_lag >= min_lag as f64 && actual_hz * 60.0 / faster_lag <= MAX_BPM {
        let mut faster_score = 0.0f64;
        let mut faster_refined = faster_lag;
        for step in -8..=8 {
            let candidate = faster_lag + step as f64 * 0.125;
            if candidate < min_lag as f64 {
                continue;
            }
            let score = correlate(&onsets, candidate);
            if score > faster_score {
                faster_score = score;
                faster_refined = candidate;
            }
        }
        if faster_score >= 0.78 && faster_score >= refined_score * 0.82 {
            refined = faster_refined;
            refined_score = faster_score;
        }
    }

    let bpm = actual_hz * 60.0 / refined;
    // Conservative score: regular repeated transients typically reach high
    // correlation; sparse/uneven material should not receive false certainty.
    let confidence = ((refined_score - 0.15) / 0.80).clamp(0.0, 1.0);
    if !(bpm.is_finite() && confidence >= 0.18) {
        return None;
    }
    Some(TempoEstimate {
        bpm: bpm as f32,
        confidence: confidence as f32,
    })
}

fn correlate(onsets: &[f64], lag: f64) -> f64 {
    let integer = lag.floor() as usize;
    let frac = lag.fract();
    if integer + 1 >= onsets.len() {
        return 0.0;
    }
    let len = onsets.len() - integer - 1;
    let mut dot = 0.0f64;
    let mut a2 = 0.0f64;
    let mut b2 = 0.0f64;
    for i in 0..len {
        let a = onsets[i];
        let b = onsets[i + integer] * (1.0 - frac) + onsets[i + integer + 1] * frac;
        dot += a * b;
        a2 += a * a;
        b2 += b * b;
    }
    if a2 <= 1e-20 || b2 <= 1e-20 {
        0.0
    } else {
        (dot / (a2 * b2).sqrt()).clamp(0.0, 1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    fn clicks(bpm: f64, seconds: f64) -> SampleData {
        let rate = 16_000usize;
        let mut left = vec![0.0f32; (rate as f64 * seconds) as usize];
        let mut t = 0.0;
        while t < seconds {
            let pos = (t * rate as f64) as usize;
            for k in 0..80 {
                if let Some(sample) = left.get_mut(pos + k) {
                    *sample += (1.0 - k as f32 / 80.0) * if ((t / (60.0 / bpm)) as usize).is_multiple_of(4) { 1.0 } else { 0.8 };
                }
            }
            t += 60.0 / bpm;
        }
        SampleData {
            sample_rate: rate as u32,
            left: Arc::from(left),
            right: None,
            ..SampleData::default()
        }
    }

    #[test]
    fn steady_drum_loops_have_fractional_bpm() {
        for bpm in [90.0, 120.0, 127.8, 174.0] {
            let estimate = detect(&clicks(bpm, 12.0)).expect("regular beat");
            assert!(
                (f64::from(estimate.bpm) - bpm).abs() < 1.2,
                "wanted {bpm}, got {estimate:?}"
            );
            assert!(estimate.confidence > 0.2);
        }
    }

    #[test]
    fn octave_ambiguity_uses_actual_pulse_density() {
        // Both lags of 128-BPM clicks can correlate strongly; 64-BPM clicks
        // lack the intervening beats and must not be spuriously doubled.
        for bpm in [64.0, 128.0, 174.0] {
            let estimate = detect(&clicks(bpm, 10.0)).expect("regular clicks");
            assert!(
                (estimate.bpm as f64 - bpm).abs() < 1.2,
                "expected {bpm} BPM, got {estimate:?}"
            );
        }
    }

    #[test]
    fn silence_and_one_shots_do_not_have_a_tempo() {
        let mut one = clicks(120.0, 0.5);
        assert_eq!(detect(&one), None);
        one.left = Arc::from(vec![0.0; 16000 * 8]);
        assert_eq!(detect(&one), None);
        let mut impulse = vec![0.0f32; 16000 * 8];
        impulse[16000] = 1.0;
        one.left = Arc::from(impulse);
        assert_eq!(detect(&one), None);
    }

    #[test]
    fn stereo_and_invalid_data_are_safe() {
        let mut sample = clicks(128.0, 8.0);
        sample.right = Some(sample.left.clone());
        assert!(detect(&sample).is_some());
        sample.sample_rate = 0;
        assert_eq!(detect(&sample), None);
    }
}
