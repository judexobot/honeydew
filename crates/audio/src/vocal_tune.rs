//! VocalTune 0.3: independent, experimental monophonic real-time pitch correction.
//! Pitch estimation: decimated YIN; shifting: dual overlapping variable-delay taps.
//! This is not a clone of any existing Honeydew Pitcher implementation.
use std::f32::consts::PI;

const HISTORY: usize = 8192;
const YIN_POINTS: usize = 256;
const MAX_LAG: usize = 220;
const MIN_DELAY: f32 = 512.0;
const DELAY_SPAN: f32 = 2048.0;
const DRY_DELAY: usize = 1536;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scale {
    Chromatic,
    Major,
    Minor,
}

#[derive(Clone, Copy, Debug)]
pub struct Parameters {
    pub root: u8,
    pub scale: Scale,
    pub correction: f32,
    pub retune_ms: f32,
    pub wet: f32,
}
impl Default for Parameters {
    fn default() -> Self {
        Self {
            root: 0,
            scale: Scale::Chromatic,
            correction: 1.0,
            retune_ms: 25.0,
            wet: 1.0,
        }
    }
}

pub struct VocalTune {
    rate: f32,
    decimation: usize,
    analysis_hop: u64,
    params: Parameters,
    left: Box<[f32; HISTORY]>,
    right: Box<[f32; HISTORY]>,
    mono: Box<[f32; HISTORY]>,
    scores: [f32; MAX_LAG + 1],
    written: u64,
    at: usize,
    phase: f32,
    ratio: f32,
    target_ratio: f32,
    retune_alpha: f32,
    voiced: f32,
    pitch_hz: Option<f32>,
    note: Option<i32>,
}
impl VocalTune {
    pub fn new(rate: u32) -> Result<Self, &'static str> {
        if !(22_050..=96_000).contains(&rate) {
            return Err("unsupported sample rate");
        }
        let rate = rate as f32;
        let mut out = Self {
            rate,
            decimation: (rate / 12_000.0).round().max(1.0) as usize,
            analysis_hop: (rate / 100.0).round() as u64,
            params: Parameters::default(),
            left: Box::new([0.0; HISTORY]),
            right: Box::new([0.0; HISTORY]),
            mono: Box::new([0.0; HISTORY]),
            scores: [0.0; MAX_LAG + 1],
            written: 0,
            at: 0,
            phase: 0.0,
            ratio: 1.0,
            target_ratio: 1.0,
            retune_alpha: 0.0,
            voiced: 0.0,
            pitch_hz: None,
            note: None,
        };
        out.set_parameters(Parameters::default());
        Ok(out)
    }
    pub fn set_parameters(&mut self, p: Parameters) {
        self.params = Parameters {
            root: p.root % 12,
            scale: p.scale,
            correction: finite_clamp(p.correction, 0.0, 1.0, 1.0),
            retune_ms: finite_clamp(p.retune_ms, 2.0, 300.0, 25.0),
            wet: finite_clamp(p.wet, 0.0, 1.0, 1.0),
        };
        self.retune_alpha = 1.0 - (-1000.0 / (self.params.retune_ms * self.rate)).exp();
    }
    pub fn parameters(&self) -> Parameters {
        self.params
    }
    pub fn detected_hz(&self) -> Option<f32> {
        self.pitch_hz
    }
    pub fn target_midi(&self) -> Option<i32> {
        self.note
    }
    pub fn ratio(&self) -> f32 {
        self.ratio
    }
    pub const fn latency_samples(&self) -> usize {
        DRY_DELAY
    }

    pub fn reset(&mut self) {
        self.left.fill(0.0);
        self.right.fill(0.0);
        self.mono.fill(0.0);
        self.scores.fill(0.0);
        self.written = 0;
        self.at = 0;
        self.phase = 0.0;
        self.ratio = 1.0;
        self.target_ratio = 1.0;
        self.voiced = 0.0;
        self.pitch_hz = None;
        self.note = None;
    }
    pub fn process(&mut self, left: &mut [f32], right: &mut [f32]) -> Result<(), &'static str> {
        if left.len() != right.len() {
            return Err("channel length mismatch");
        }
        for (l, r) in left.iter_mut().zip(right.iter_mut()) {
            (*l, *r) = self.process_frame(*l, *r);
        }
        Ok(())
    }
    pub fn process_frame(&mut self, left: f32, right: f32) -> (f32, f32) {
        let left = if left.is_finite() { left } else { 0.0 };
        let right = if right.is_finite() { right } else { 0.0 };
        self.left[self.at] = left;
        self.right[self.at] = right;
        self.mono[self.at] = (left + right) * 0.5;
        if self.written >= 4096 && self.written.is_multiple_of(self.analysis_hop) {
            self.analyze();
        }
        self.ratio += (self.target_ratio - self.ratio) * self.retune_alpha;
        let target_voiced = if self.pitch_hz.is_some() { 1.0 } else { 0.0 };
        self.voiced += (target_voiced - self.voiced) * 0.001;
        self.phase = (self.phase + (1.0 - self.ratio) / DELAY_SPAN).rem_euclid(1.0);
        let other_phase = (self.phase + 0.5).fract();
        let w = (self.phase * PI).sin().powi(2);
        let dry_l = tap(&self.left, self.at, DRY_DELAY as f32);
        let dry_r = tap(&self.right, self.at, DRY_DELAY as f32);
        let d1 = MIN_DELAY + DELAY_SPAN * self.phase;
        let d2 = MIN_DELAY + DELAY_SPAN * other_phase;
        let wet_l = w * tap(&self.left, self.at, d1) + (1.0 - w) * tap(&self.left, self.at, d2);
        let wet_r = w * tap(&self.right, self.at, d1) + (1.0 - w) * tap(&self.right, self.at, d2);
        // At exact unison bypass the delay-tap comb coloration. Fade near unison.
        let shift_weight = ((self.ratio - 1.0).abs() * 100.0).min(1.0);
        let mix = self.params.wet * self.voiced * shift_weight;
        self.written += 1;
        self.at = (self.at + 1) % HISTORY;
        (dry_l + (wet_l - dry_l) * mix, dry_r + (wet_r - dry_r) * mix)
    }
    fn analyze(&mut self) {
        let stride = self.decimation;
        let min_lag = ((self.rate / 1100.0) / stride as f32).floor().max(2.0) as usize;
        let max_lag = ((self.rate / 70.0) / stride as f32)
            .ceil()
            .min(MAX_LAG as f32) as usize;
        let mut energy = 0.0;
        for j in 0..YIN_POINTS {
            let x = self.mono[lag_index(self.at, j * stride)];
            energy += x * x;
        }
        if energy < 0.00001 * YIN_POINTS as f32 {
            self.unvoiced();
            return;
        }
        let mut cumulative = 0.0;
        let mut best = (0, 1.0f32);
        let mut first = None;
        for lag in 1..=max_lag {
            let mut diff = 0.0;
            for j in 0..YIN_POINTS {
                let i = j * stride;
                let d = self.mono[lag_index(self.at, i)]
                    - self.mono[lag_index(self.at, i + lag * stride)];
                diff += d * d;
            }
            cumulative += diff;
            let score = if cumulative > 1e-10 {
                diff * lag as f32 / cumulative
            } else {
                1.0
            };
            self.scores[lag] = score;
            if lag >= min_lag {
                if score < best.1 {
                    best = (lag, score);
                }
                if score < 0.16 && first.is_none() {
                    first = Some(lag);
                }
            }
        }
        let mut lag = first.unwrap_or(best.0);
        if lag < 2 || best.1 > 0.25 {
            self.unvoiced();
            return;
        }
        while lag < max_lag && self.scores[lag + 1] < self.scores[lag] {
            lag += 1;
        }
        let a = self.scores[lag - 1];
        let b = self.scores[lag];
        let c = self.scores[(lag + 1).min(max_lag)];
        let den = a - 2.0 * b + c;
        let frac = if den.abs() > 1e-8 {
            (0.5 * (a - c) / den).clamp(-1.0, 1.0)
        } else {
            0.0
        };
        let hz = self.rate / ((lag as f32 + frac) * stride as f32);
        if !(70.0..=1100.0).contains(&hz) || !hz.is_finite() {
            self.unvoiced();
            return;
        }
        self.pitch_hz = Some(hz);
        let midi = 69.0 + 12.0 * (hz / 440.0).log2();
        let next = nearest_note(midi, self.params.root, self.params.scale);
        let next = match (next, self.note) {
            (Some(n), Some(prev))
                if n != prev
                    && allowed(prev, self.params.root, self.params.scale)
                    && (midi - prev as f32).abs() < (midi - n as f32).abs() + 0.10 =>
            {
                Some(prev)
            }
            (other, _) => other,
        };
        self.note = next;
        self.target_ratio = next.map_or(1.0, |n| {
            2.0f32
                .powf(((n as f32 - midi) * self.params.correction) / 12.0)
                .clamp(0.85, 1.18)
        });
    }
    fn unvoiced(&mut self) {
        self.pitch_hz = None;
        self.note = None;
        self.target_ratio = 1.0;
    }
}
fn finite_clamp(v: f32, min: f32, max: f32, fallback: f32) -> f32 {
    if v.is_finite() {
        v.clamp(min, max)
    } else {
        fallback
    }
}
fn lag_index(write: usize, delay: usize) -> usize {
    (write + HISTORY - delay % HISTORY) % HISTORY
}
fn tap(ring: &[f32; HISTORY], write: usize, delay: f32) -> f32 {
    let delay = delay.clamp(0.0, (HISTORY - 2) as f32);
    let whole = delay.floor() as usize;
    let frac = delay - whole as f32;
    let a = ring[lag_index(write, whole)];
    let b = ring[lag_index(write, whole + 1)];
    a + (b - a) * frac
}
fn allowed(m: i32, root: u8, scale: Scale) -> bool {
    match scale {
        Scale::Chromatic => true,
        Scale::Major => matches!((m - root as i32).rem_euclid(12), 0 | 2 | 4 | 5 | 7 | 9 | 11),
        Scale::Minor => matches!((m - root as i32).rem_euclid(12), 0 | 2 | 3 | 5 | 7 | 8 | 10),
    }
}
pub fn nearest_note(midi: f32, root: u8, scale: Scale) -> Option<i32> {
    if !midi.is_finite() {
        return None;
    }
    let around = midi.round() as i32;
    (-2..=2)
        .map(|i| around + i)
        .filter(|&n| allowed(n, root % 12, scale))
        .min_by(|a, b| {
            (midi - *a as f32)
                .abs()
                .total_cmp(&(midi - *b as f32).abs())
        })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn scale_quantization() {
        assert_eq!(nearest_note(60.6, 0, Scale::Major), Some(60));
        assert_eq!(nearest_note(61.7, 0, Scale::Major), Some(62));
        assert_eq!(nearest_note(63.1, 0, Scale::Minor), Some(63));
    }
    #[test]
    fn detects_440_hz() {
        let mut a = VocalTune::new(48_000).unwrap();
        for i in 0..24000 {
            let x = (2.0 * PI * 440.0 * i as f32 / 48000.0).sin() * 0.4;
            a.process_frame(x, x);
        }
        assert!((a.detected_hz().unwrap() - 440.0).abs() < 8.0);
    }
    #[test]
    fn respects_bypass_and_finite_audio() {
        let mut a = VocalTune::new(48_000).unwrap();
        a.set_parameters(Parameters {
            wet: 0.0,
            ..Default::default()
        });
        let mut l = vec![0.0; 16000];
        let mut r = l.clone();
        for (i, sample) in l.iter_mut().enumerate() {
            *sample = (i as f32 * 0.04).sin();
        }
        a.process(&mut l, &mut r).unwrap();
        for i in 1536..l.len() {
            assert!(l[i].is_finite());
            assert_eq!(r[i], 0.0);
        }
        assert!(a.process(&mut [0.0; 2], &mut [0.0; 1]).is_err());
    }
    #[test]
    fn retune_moves_ratio_toward_note() {
        let mut a = VocalTune::new(48_000).unwrap();
        a.set_parameters(Parameters {
            retune_ms: 2.0,
            ..Default::default()
        });
        for i in 0..48000 {
            let x = (2.0 * PI * 450.0 * i as f32 / 48000.0).sin() * 0.5;
            a.process_frame(x, x);
        }
        assert_eq!(a.target_midi(), Some(69));
        assert!((a.ratio() - 440.0 / 450.0).abs() < 0.012);
    }
}
