//! Independent monophonic vocal correction: anti-aliased YIN, linked stereo
//! phase-locked spectral shifting and cepstral formant preservation.
mod spectral;
use spectral::Spectral;
use std::f32::consts::{PI, TAU};

pub const NUM_PARAMS: usize = 33;
pub const DEFAULTS: [f32; NUM_PARAMS] = [
    1.0, 0.0, 0.0, 1.0, 0.25, 1.0, 0.2, 0.5, 0.1, 0.5, 0.0, 0.5, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0,
    1.0, 1.0, 1.0, 1.0, 1.0, 0.75, 1.0, 0.5, 0.35, 0.15, 0.1, 0.0, 0.5, 0.0,
];
const HISTORY: usize = 16_384;
const DETECTOR: usize = 1024;
const MAX_LAG: usize = 256;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scale {
    Chromatic,
    Major,
    Minor,
    Pentatonic,
    Blues,
    Custom,
}
#[derive(Clone, Copy, Debug)]
pub struct Parameters {
    pub root: u8,
    pub scale: Scale,
    pub correction: f32,
    pub retune_ms: f32,
    pub wet: f32,
    pub humanize: f32,
    pub sensitivity: f32,
    pub deadband_cents: f32,
    pub reference_hz: f32,
    pub hard_tune: bool,
    pub detune_cents: f32,
    pub custom_notes: u16,
    pub protect_sibilants: f32,
    pub formant_preserve: bool,
    pub formant_shift: f32,
    pub vibrato: f32,
    pub transition_ms: f32,
    pub flex_cents: f32,
    /// 0: full (55..1500 Hz), 1: low, 2: medium, 3: high.
    pub input_range: u8,
    pub output_db: f32,
    pub studio: bool,
    pub enabled: bool,
}
impl Default for Parameters {
    fn default() -> Self {
        Self {
            root: 0,
            scale: Scale::Chromatic,
            correction: 1.0,
            retune_ms: 25.0,
            wet: 1.0,
            humanize: 0.2,
            sensitivity: 0.5,
            deadband_cents: 5.0,
            reference_hz: 440.0,
            hard_tune: false,
            detune_cents: 0.0,
            custom_notes: 0xfff,
            protect_sibilants: 0.75,
            formant_preserve: true,
            formant_shift: 0.0,
            vibrato: 0.35,
            transition_ms: 15.0,
            flex_cents: 10.0,
            input_range: 0,
            output_db: 0.0,
            studio: false,
            enabled: true,
        }
    }
}

pub struct VocalTune {
    rate: f32,
    decimation: usize,
    hop: usize,
    params: Parameters,
    spectral: [Spectral; 2],
    left: Vec<f32>,
    right: Vec<f32>,
    at: usize,
    written: u64,
    fir: Vec<f32>,
    fir_ring: Vec<f32>,
    fir_at: usize,
    detector: [f32; DETECTOR],
    detector_at: usize,
    detector_written: usize,
    scores: [f32; MAX_LAG + 1],
    pitch_hz: Option<f32>,
    note: Option<i32>,
    ratio: f32,
    target_ratio: f32,
    clarity: f32,
    sibilance: f32,
    sibilance_smoothed: f32,
    wet_mix_smoothed: f32,
    voiced: f32,
    sustain: usize,
    base_midi: f32,
    recent: [f32; 3],
    pitch_count: usize,
    retune_alpha: f32,
    gain: f32,
    target_gain: f32,
    bypass: f32,
    dry: (f32, f32),
    energies: [f32; 2],
    switch_from: Option<usize>,
    switch_left: usize,
    switch_total: usize,
}
impl VocalTune {
    pub fn new(rate: u32) -> Result<Self, &'static str> {
        if !(8_000..=192_000).contains(&rate) {
            return Err("unsupported sample rate");
        }
        let decimation = (rate as f32 / 12_000.0).ceil().max(1.0) as usize;
        let taps = decimation * 16 + 1;
        let cutoff = 0.44 / decimation as f32;
        let mut fir: Vec<f32> = (0..taps)
            .map(|i| {
                let t = i as f32 - (taps - 1) as f32 * 0.5;
                let sinc = if t.abs() < 1e-6 {
                    2.0 * cutoff
                } else {
                    (TAU * cutoff * t).sin() / (PI * t)
                };
                sinc * (0.42 - 0.5 * (TAU * i as f32 / (taps - 1) as f32).cos()
                    + 0.08 * (2.0 * TAU * i as f32 / (taps - 1) as f32).cos())
            })
            .collect();
        let sum: f32 = fir.iter().sum();
        for v in &mut fir {
            *v /= sum;
        }
        let mut out = Self {
            rate: rate as f32,
            decimation,
            hop: (rate as usize / 200).max(1),
            params: Parameters::default(),
            spectral: [Spectral::new(rate, false), Spectral::new(rate, true)],
            left: vec![0.0; HISTORY],
            right: vec![0.0; HISTORY],
            at: 0,
            written: 0,
            fir_ring: vec![0.0; taps],
            fir,
            fir_at: 0,
            detector: [0.0; DETECTOR],
            detector_at: 0,
            detector_written: 0,
            scores: [1.0; MAX_LAG + 1],
            pitch_hz: None,
            note: None,
            ratio: 1.0,
            target_ratio: 1.0,
            clarity: 0.0,
            sibilance: 0.0,
            sibilance_smoothed: 0.0,
            wet_mix_smoothed: 0.0,
            voiced: 0.0,
            sustain: 0,
            base_midi: 69.0,
            recent: [69.0; 3],
            pitch_count: 0,
            retune_alpha: 0.0,
            gain: 1.0,
            target_gain: 1.0,
            bypass: 1.0,
            dry: (0.0, 0.0),
            energies: [0.0; 2],
            switch_from: None,
            switch_left: 0,
            switch_total: 0,
        };
        out.set_parameters(Parameters::default());
        Ok(out)
    }
    pub fn set_parameters(&mut self, p: Parameters) {
        if p.studio != self.params.studio {
            let new = usize::from(p.studio);
            self.spectral[new].reset();
            if self.written > 0 {
                self.switch_from = Some(usize::from(self.params.studio));
                self.switch_total = self.spectral[new].latency() + (self.rate * 0.020) as usize;
                self.switch_left = self.switch_total;
            }
        }
        self.params = Parameters {
            root: p.root % 12,
            scale: p.scale,
            correction: clean(p.correction, 0.0, 1.0, 1.0),
            retune_ms: clean(p.retune_ms, 2.0, 300.0, 25.0),
            wet: clean(p.wet, 0.0, 1.0, 1.0),
            humanize: clean(p.humanize, 0.0, 1.0, 0.2),
            sensitivity: clean(p.sensitivity, 0.0, 1.0, 0.5),
            deadband_cents: clean(p.deadband_cents, 0.0, 50.0, 5.0),
            reference_hz: clean(p.reference_hz, 430.0, 450.0, 440.0),
            hard_tune: p.hard_tune,
            detune_cents: clean(p.detune_cents, -50.0, 50.0, 0.0),
            custom_notes: p.custom_notes & 0xfff,
            protect_sibilants: clean(p.protect_sibilants, 0.0, 1.0, 0.75),
            formant_preserve: p.formant_preserve,
            formant_shift: clean(p.formant_shift, -12.0, 12.0, 0.0),
            vibrato: clean(p.vibrato, 0.0, 1.0, 0.35),
            transition_ms: clean(p.transition_ms, 0.0, 200.0, 15.0),
            flex_cents: clean(p.flex_cents, 0.0, 100.0, 10.0),
            input_range: p.input_range.min(3),
            output_db: clean(p.output_db, -24.0, 24.0, 0.0),
            studio: p.studio,
            enabled: p.enabled,
        };
        self.retune_alpha = 1.0 - (-1000.0 / (self.params.retune_ms * self.rate)).exp();
        self.target_gain = 10.0_f32.powf(self.params.output_db / 20.0);
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
    pub fn clarity(&self) -> f32 {
        self.clarity
    }
    pub fn latency_samples(&self) -> usize {
        self.spectral[usize::from(self.params.studio)].latency()
    }
    pub fn maximum_latency_samples(&self) -> usize {
        self.spectral[1].latency()
    }
    /// The same latency-aligned dry signal used by the effect, for the host's common Mix.
    pub fn dry_frame(&self) -> (f32, f32) {
        self.dry
    }
    pub fn reset(&mut self) {
        self.left.fill(0.0);
        self.right.fill(0.0);
        self.detector.fill(0.0);
        self.fir_ring.fill(0.0);
        for s in &mut self.spectral {
            s.reset();
        }
        self.at = 0;
        self.written = 0;
        self.detector_at = 0;
        self.detector_written = 0;
        self.fir_at = 0;
        self.ratio = 1.0;
        self.target_ratio = 1.0;
        self.voiced = 0.0;
        self.pitch_hz = None;
        self.note = None;
        self.sustain = 0;
        self.clarity = 0.0;
        self.sibilance = 0.0;
        self.sibilance_smoothed = 0.0;
        self.wet_mix_smoothed = 0.0;
        self.pitch_count = 0;
        self.gain = self.target_gain;
        self.bypass = f32::from(self.params.enabled);
        self.dry = (0.0, 0.0);
        self.energies = [0.0; 2];
        self.switch_from = None;
        self.switch_left = 0;
    }
    pub fn process(&mut self, left: &mut [f32], right: &mut [f32]) -> Result<(), &'static str> {
        if left.len() != right.len() {
            return Err("channel length mismatch");
        }
        for (l, r) in left.iter_mut().zip(right) {
            (*l, *r) = self.process_frame(*l, *r);
        }
        Ok(())
    }
    pub fn process_frame(&mut self, left: f32, right: f32) -> (f32, f32) {
        let input = [
            if left.is_finite() { left } else { 0.0 },
            if right.is_finite() { right } else { 0.0 },
        ];
        self.left[self.at] = input[0];
        self.right[self.at] = input[1];
        for (energy, x) in self.energies.iter_mut().zip(input) {
            *energy += (x * x - *energy) * 0.001;
        }
        // Prefer the louder channel; a mid signal alone loses an anti-phase vocal.
        let mono = if self.energies[0] >= self.energies[1] {
            input[0]
        } else {
            input[1]
        };
        self.fir_ring[self.fir_at] = mono;
        self.fir_at = (self.fir_at + 1) % self.fir.len();
        if self.written.is_multiple_of(self.decimation as u64) {
            let mut x = 0.0;
            for (j, &coefficient) in self.fir.iter().enumerate() {
                x += self.fir_ring[(self.fir_at + j) % self.fir.len()] * coefficient;
            }
            self.detector[self.detector_at] = x;
            self.detector_at = (self.detector_at + 1) % DETECTOR;
            self.detector_written = self.detector_written.saturating_add(1);
        }
        if self.written.is_multiple_of(self.hop as u64) && self.detector_written >= 640 {
            self.analyze();
        }
        let held = (self.sustain as f32 / (self.rate * 0.35)).min(1.0);
        let humanize = if self.params.hard_tune {
            0.0
        } else {
            self.params.humanize
        };
        let transition = if self.params.hard_tune {
            1.0
        } else {
            1.0 + self.params.transition_ms / self.params.retune_ms * (1.0 - held)
        };
        let alpha = self.retune_alpha / ((1.0 + 5.0 * humanize * held) * transition);
        self.ratio += (self.target_ratio - self.ratio) * alpha;
        self.voiced +=
            (f32::from(self.pitch_hz.is_some()) - self.voiced) * (1.0 / (self.rate * 0.004));
        self.gain += (self.target_gain - self.gain) * 0.001;
        self.bypass += (f32::from(self.params.enabled) - self.bypass) * 0.002;
        let profile = usize::from(self.params.studio);
        let delay = self.spectral[profile].latency();
        let read = (self.at + HISTORY - delay) % HISTORY;
        self.dry = (self.left[read], self.right[read]);
        let wet = self.spectral[profile].frame(
            input,
            self.ratio,
            self.pitch_hz.unwrap_or(150.0),
            self.params.formant_preserve,
            self.params.formant_shift,
            self.params.protect_sibilants,
        );
        // Smooth pitch confidence, consonant protection and Wet automation together.
        // Abrupt detector changes must never hard-switch the spectral output against the dry voice.
        let consonant_alpha = 1.0 - (-1.0 / (self.rate * 0.008)).exp();
        self.sibilance_smoothed += (self.sibilance - self.sibilance_smoothed) * consonant_alpha;
        let confidence = ((self.clarity - 0.28) / 0.52).clamp(0.0, 1.0);
        let target_mix = self.params.wet
            * self.voiced
            * confidence
            * (1.0 - self.sibilance_smoothed * self.params.protect_sibilants)
            * self.bypass;
        let wet_alpha = 1.0 - (-1.0 / (self.rate * 0.012)).exp();
        self.wet_mix_smoothed += (target_mix - self.wet_mix_smoothed) * wet_alpha;
        let mix = self.wet_mix_smoothed.clamp(0.0, 1.0);
        let gain = 1.0 + (self.gain - 1.0) * self.bypass;
        let mut output = [
            self.dry.0 + (wet[0] - self.dry.0) * mix,
            self.dry.1 + (wet[1] - self.dry.1) * mix,
        ];
        if let Some(old) = self.switch_from {
            let old_delay = self.spectral[old].latency();
            let read = (self.at + HISTORY - old_delay) % HISTORY;
            let old_dry = [self.left[read], self.right[read]];
            let old_wet = self.spectral[old].frame(
                input,
                self.ratio,
                self.pitch_hz.unwrap_or(150.0),
                self.params.formant_preserve,
                self.params.formant_shift,
                self.params.protect_sibilants,
            );
            let fade = (self.rate * 0.020).max(1.0);
            let weight = (1.0 - self.switch_left as f32 / fade).clamp(0.0, 1.0);
            for ch in 0..2 {
                let old_output = old_dry[ch] + (old_wet[ch] - old_dry[ch]) * mix;
                output[ch] = old_output + (output[ch] - old_output) * weight;
            }
            self.dry = (
                old_dry[0] + (self.dry.0 - old_dry[0]) * weight,
                old_dry[1] + (self.dry.1 - old_dry[1]) * weight,
            );
            self.switch_left = self.switch_left.saturating_sub(1);
            if self.switch_left == 0 {
                self.switch_from = None;
            }
        }
        self.at = (self.at + 1) % HISTORY;
        self.written += 1;
        (output[0] * gain, output[1] * gain)
    }
    fn sample(&self, delay: usize) -> f32 {
        self.detector[(self.detector_at + DETECTOR - 1 - delay) % DETECTOR]
    }
    fn analyze(&mut self) {
        let rate = self.rate / self.decimation as f32;
        let (low, high) = match self.params.input_range {
            1 => (55.0, 440.0),
            2 => (80.0, 880.0),
            3 => (130.0, 1500.0),
            _ => (55.0, 1500.0),
        };
        let min_lag = (rate / high).floor().max(2.0) as usize;
        let max_lag = ((rate / low).ceil() as usize).min(MAX_LAG);
        let points = if self.params.studio { 384 } else { 256 };
        let mut energy = 0.0;
        let mut roughness = 0.0;
        let mut absolute = 0.0;
        for j in 0..points {
            let x = self.sample(j);
            energy += x * x;
            absolute += x.abs();
            if j > 0 {
                roughness += (x - self.sample(j - 1)).abs();
            }
        }
        self.sibilance = ((roughness / (2.0 * absolute + 1e-8) - 0.28) / 0.45).clamp(0.0, 1.0);
        if energy / (points as f32) < 0.00005 - self.params.sensitivity * 0.000049 {
            self.unvoiced();
            return;
        }
        let threshold = 0.08 + 0.18 * self.params.sensitivity;
        let mut cumulative = 0.0;
        let mut best = (min_lag, 1.0);
        for lag in 1..=max_lag {
            let mut d = 0.0;
            for j in 0..points {
                let difference = self.sample(j) - self.sample(j + lag);
                d += difference * difference;
            }
            cumulative += d;
            self.scores[lag] = if cumulative > 1e-12 {
                d * lag as f32 / cumulative
            } else {
                1.0
            };
            if lag >= min_lag && self.scores[lag] < best.1 {
                best = (lag, self.scores[lag]);
            }
        }
        if best.1 > threshold + 0.1 {
            self.unvoiced();
            return;
        }
        let mut lag = (min_lag..=max_lag)
            .find(|&l| self.scores[l] < threshold)
            .unwrap_or(best.0);
        while lag < max_lag && self.scores[lag + 1] < self.scores[lag] {
            lag += 1;
        }
        if let Some(previous) = self.pitch_hz {
            let old = (rate / previous).round() as usize;
            if old >= min_lag
                && old < max_lag
                && self.scores[old] < self.scores[lag] + 0.025
                && (1200.0 * (rate / lag as f32 / previous).log2()).abs() > 700.0
            {
                lag = old;
            }
        }
        let a = self.scores[lag - 1];
        let b = self.scores[lag];
        let c = self.scores[(lag + 1).min(max_lag)];
        let den = a - 2.0 * b + c;
        let frac = if den.abs() > 1e-8 {
            (0.5 * (a - c) / den).clamp(-0.5, 0.5)
        } else {
            0.0
        };
        let hz = rate / (lag as f32 + frac);
        if !hz.is_finite() || hz < low * 0.98 || hz > high * 1.02 {
            self.unvoiced();
            return;
        }
        self.clarity = (1.0 - b).clamp(0.0, 1.0);
        self.sibilance *= 1.0 - self.clarity;
        self.pitch_hz = Some(hz);
        let raw = 69.0 + 12.0 * (hz / self.params.reference_hz).log2();
        self.recent[self.pitch_count % 3] = raw;
        self.pitch_count = self.pitch_count.saturating_add(1);
        let mut pitches = self.recent;
        pitches.sort_by(f32::total_cmp);
        let midi = if self.pitch_count >= 3 {
            pitches[1]
        } else {
            raw
        };
        if self.pitch_count < 3 || (midi - self.base_midi).abs() > 0.75 {
            self.base_midi = midi;
        } else {
            self.base_midi += (midi - self.base_midi) * 0.12;
        }
        let mut next = nearest_note_with_mask(
            midi,
            self.params.root,
            self.params.scale,
            self.params.custom_notes,
        );
        if let (Some(n), Some(old)) = (next, self.note)
            && allowed(
                old,
                self.params.root,
                self.params.scale,
                self.params.custom_notes,
            )
            && n != old
            && (midi - old as f32).abs() < (midi - n as f32).abs() + 0.08
        {
            next = Some(old);
        }
        if next == self.note {
            self.sustain = self.sustain.saturating_add(self.hop);
        } else {
            self.sustain = 0;
        }
        self.note = next;
        let cents = next.map_or(0.0, |n| {
            let mut correction = (n as f32 - midi) * 100.0;
            if !self.params.hard_tune {
                correction += (midi - self.base_midi) * 100.0 * self.params.vibrato;
                if correction.abs() < self.params.deadband_cents {
                    correction = 0.0;
                } else if self.params.flex_cents > 0.0 {
                    correction *= (correction.abs() / self.params.flex_cents).min(1.0);
                }
            }
            correction * self.params.correction + self.params.detune_cents
        });
        self.target_ratio = 2.0_f32.powf(cents / 1200.0).clamp(0.5, 2.0);
    }
    fn unvoiced(&mut self) {
        self.pitch_hz = None;
        self.note = None;
        self.clarity = 0.0;
        self.target_ratio = 1.0;
        self.sustain = 0;
        self.pitch_count = 0;
    }
}
fn clean(x: f32, low: f32, high: f32, default: f32) -> f32 {
    if x.is_finite() {
        x.clamp(low, high)
    } else {
        default
    }
}
/// The absolute C..B mask of a built-in or user scale.
pub fn scale_mask(root: u8, scale: Scale, custom: u16) -> u16 {
    (0..12).fold(0, |mask, n| {
        if allowed(n, root, scale, custom) {
            mask | (1 << n)
        } else {
            mask
        }
    })
}
fn allowed(m: i32, root: u8, scale: Scale, custom_notes: u16) -> bool {
    match scale {
        Scale::Chromatic => true,
        Scale::Major => matches!((m - root as i32).rem_euclid(12), 0 | 2 | 4 | 5 | 7 | 9 | 11),
        Scale::Minor => matches!((m - root as i32).rem_euclid(12), 0 | 2 | 3 | 5 | 7 | 8 | 10),
        Scale::Pentatonic => matches!((m - root as i32).rem_euclid(12), 0 | 2 | 4 | 7 | 9),
        Scale::Blues => matches!((m - root as i32).rem_euclid(12), 0 | 3 | 5 | 6 | 7 | 10),
        Scale::Custom => custom_notes & (1u16 << m.rem_euclid(12)) != 0,
    }
}
pub fn nearest_note(midi: f32, root: u8, scale: Scale) -> Option<i32> {
    nearest_note_with_mask(midi, root, scale, 0x0fff)
}
pub fn nearest_note_with_mask(midi: f32, root: u8, scale: Scale, custom_notes: u16) -> Option<i32> {
    if !midi.is_finite() {
        return None;
    }
    let around = midi.round() as i32;
    // Any custom scale can skip several semitones. Search a full octave both ways.
    (-12..=12)
        .map(|i| around + i)
        .filter(|&n| allowed(n, root % 12, scale, custom_notes))
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
    fn custom_note_map_only_uses_enabled_notes() {
        let mask = (1 << 0) | (1 << 7); // C and G in any octave
        assert_eq!(
            nearest_note_with_mask(61.0, 0, Scale::Custom, mask),
            Some(60)
        );
        assert_eq!(
            nearest_note_with_mask(66.6, 0, Scale::Custom, mask),
            Some(67)
        );
        assert_eq!(nearest_note_with_mask(60.0, 0, Scale::Custom, 0), None);
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
