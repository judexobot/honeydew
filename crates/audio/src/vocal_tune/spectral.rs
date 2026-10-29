//! Stereo phase-locked STFT shifter with cepstral source/filter separation.
use rustfft::{Fft, FftPlanner, num_complex::Complex32};
use std::f32::consts::{PI, TAU};
use std::sync::Arc;

struct Channel {
    input: Vec<f32>,
    output: Vec<f32>,
    spectrum: Vec<Complex32>,
    shifted: Vec<Complex32>,
    cepstrum: Vec<Complex32>,
    scratch: Vec<Complex32>,
    magnitude: Vec<f32>,
    phase: Vec<f32>,
    previous: Vec<f32>,
    synthesis: Vec<f32>,
    envelope: Vec<f32>,
    peaks: Vec<usize>,
    fresh: bool,
}
impl Channel {
    fn new(n: usize, scratch: usize) -> Self {
        Self {
            input: vec![0.0; n],
            output: vec![0.0; n * 2],
            spectrum: vec![Complex32::default(); n],
            shifted: vec![Complex32::default(); n],
            cepstrum: vec![Complex32::default(); n],
            scratch: vec![Complex32::default(); scratch],
            magnitude: vec![0.0; n / 2 + 1],
            phase: vec![0.0; n / 2 + 1],
            previous: vec![0.0; n / 2 + 1],
            synthesis: vec![0.0; n / 2 + 1],
            envelope: vec![1.0; n / 2 + 1],
            peaks: Vec::with_capacity(n / 2 + 1),
            fresh: true,
        }
    }
    fn clear(&mut self) {
        self.input.fill(0.0);
        self.output.fill(0.0);
        self.previous.fill(0.0);
        self.synthesis.fill(0.0);
        self.fresh = true;
    }
}

pub(super) struct Spectral {
    n: usize,
    hop: usize,
    at: usize,
    output_at: usize,
    count: usize,
    rate: f32,
    fft: Arc<dyn Fft<f32>>,
    inverse: Arc<dyn Fft<f32>>,
    window: Vec<f32>,
    channels: [Channel; 2],
}
impl Spectral {
    pub fn new(rate: u32, studio: bool) -> Self {
        let n = ((rate as f32 * if studio { 0.040 } else { 0.020 }).ceil() as usize)
            .next_power_of_two()
            .max(256);
        let mut planner = FftPlanner::new();
        let fft = planner.plan_fft_forward(n);
        let inverse = planner.plan_fft_inverse(n);
        let scratch = fft
            .get_inplace_scratch_len()
            .max(inverse.get_inplace_scratch_len());
        Self {
            n,
            hop: n / 4,
            at: 0,
            output_at: 0,
            count: 0,
            rate: rate as f32,
            fft,
            inverse,
            window: (0..n)
                .map(|i| 0.5 - 0.5 * (TAU * i as f32 / n as f32).cos())
                .collect(),
            channels: [Channel::new(n, scratch), Channel::new(n, scratch)],
        }
    }
    pub fn latency(&self) -> usize {
        self.n
    }
    pub fn reset(&mut self) {
        self.at = 0;
        self.output_at = 0;
        self.count = 0;
        for c in &mut self.channels {
            c.clear();
        }
    }
    pub fn frame(
        &mut self,
        input: [f32; 2],
        ratio: f32,
        fundamental: f32,
        preserve: bool,
        formant: f32,
        protection: f32,
    ) -> [f32; 2] {
        let mut out = [0.0; 2];
        for (i, c) in self.channels.iter_mut().enumerate() {
            c.input[self.at] = input[i];
            out[i] = c.output[self.output_at];
            c.output[self.output_at] = 0.0;
        }
        self.at = (self.at + 1) % self.n;
        self.output_at = (self.output_at + 1) % (self.n * 2);
        self.count += 1;
        if self.count.is_multiple_of(self.hop) {
            self.transform(ratio, fundamental, preserve, formant, protection);
        }
        out
    }
    fn transform(
        &mut self,
        ratio: f32,
        fundamental: f32,
        preserve: bool,
        formant: f32,
        protection: f32,
    ) {
        let n = self.n;
        let half = n / 2;
        for c in &mut self.channels {
            for i in 0..n {
                c.spectrum[i] = Complex32::new(c.input[(self.at + i) % n] * self.window[i], 0.0);
            }
            self.fft
                .process_with_scratch(&mut c.spectrum, &mut c.scratch);
            for k in 0..=half {
                c.magnitude[k] = c.spectrum[k].norm();
                c.phase[k] = c.spectrum[k].arg();
            }
            if (ratio - 1.0).abs() < 1e-5 && formant.abs() < 1e-5 {
                c.shifted.copy_from_slice(&c.spectrum);
                c.previous.copy_from_slice(&c.phase);
                c.synthesis.copy_from_slice(&c.phase);
            } else {
                // Smooth log magnitude in the cepstral domain. Quefrency is below the
                // current pitch period, removing harmonic ripple while retaining formants.
                for k in 0..=half {
                    c.cepstrum[k] = Complex32::new(c.magnitude[k].max(1e-5).ln(), 0.0);
                }
                for k in 1..half {
                    c.cepstrum[n - k] = c.cepstrum[k];
                }
                self.inverse
                    .process_with_scratch(&mut c.cepstrum, &mut c.scratch);
                let cutoff = (self.rate * 0.5 / fundamental.max(70.0))
                    .round()
                    .clamp(8.0, (n / 8) as f32) as usize;
                for q in 0..n {
                    let distance = q.min(n - q);
                    let lifter = if distance >= cutoff {
                        0.0
                    } else {
                        0.5 + 0.5 * (PI * distance as f32 / cutoff as f32).cos()
                    };
                    c.cepstrum[q] *= lifter / n as f32;
                }
                self.fft
                    .process_with_scratch(&mut c.cepstrum, &mut c.scratch);
                for k in 0..=half {
                    c.envelope[k] = c.cepstrum[k].re.clamp(-16.0, 16.0).exp();
                }
                c.peaks.clear();
                for k in 1..half {
                    if c.magnitude[k] >= c.magnitude[k - 1] && c.magnitude[k] > c.magnitude[k + 1] {
                        c.peaks.push(k);
                    }
                }
                if c.peaks.is_empty() {
                    c.peaks.push(1);
                }
                let maximum = c.magnitude.iter().copied().fold(0.0, f32::max);
                let has_formants = c
                    .peaks
                    .iter()
                    .filter(|&&k| c.magnitude[k] > maximum * 0.05)
                    .count()
                    >= 3;
                for &peak in &c.peaks {
                    let expected = TAU * peak as f32 * self.hop as f32 / n as f32;
                    let delta = wrap(c.phase[peak] - c.previous[peak] - expected);
                    if c.fresh {
                        c.synthesis[peak] = c.phase[peak];
                    } else {
                        c.synthesis[peak] = wrap(c.synthesis[peak] + ratio * (expected + delta));
                    }
                }
                c.shifted.fill(Complex32::default());
                let shift = 2.0_f32.powf(formant / 12.0);
                let mut p = 0;
                for k in 1..half {
                    while p + 1 < c.peaks.len() && k > (c.peaks[p] + c.peaks[p + 1]) / 2 {
                        p += 1;
                    }
                    let peak = c.peaks[p];
                    let target = k as f32 * ratio;
                    if target >= (half - 1) as f32 {
                        continue;
                    }
                    let destination = target as usize;
                    let frac = target - destination as f32;
                    let envelope_position = if preserve && has_formants {
                        target / shift
                    } else {
                        k as f32 / shift
                    };
                    let gain = (interpolate(&c.envelope, envelope_position)
                        / c.envelope[k].max(1e-5))
                    .clamp(0.125, 8.0);
                    let phase = c.synthesis[peak] + wrap(c.phase[k] - c.phase[peak]);
                    let value = Complex32::from_polar(c.magnitude[k] * gain, phase);
                    c.shifted[destination] += value * (1.0 - frac);
                    c.shifted[destination + 1] += value * frac;
                }
                // Sibilant protection leaves the high-frequency stochastic component
                // at its original frequency, using a smooth frequency crossover.
                for k in 0..=half {
                    let hz = k as f32 * self.rate / n as f32;
                    let keep = ((hz - 3500.0) / 2500.0).clamp(0.0, 1.0) * protection;
                    c.shifted[k] = c.shifted[k] * (1.0 - keep) + c.spectrum[k] * keep;
                }
                c.shifted[0] = c.spectrum[0];
                c.shifted[half].im = 0.0;
                for k in 1..half {
                    c.shifted[n - k] = c.shifted[k].conj();
                }
                c.previous.copy_from_slice(&c.phase);
            }
            c.fresh = false;
            self.inverse
                .process_with_scratch(&mut c.shifted, &mut c.scratch);
            // Four overlapping periodic Hann windows sum to 1.5 after squaring.
            for i in 0..n {
                let position = (self.output_at + i) % (n * 2);
                let value = c.shifted[i].re * self.window[i] / (1.5 * n as f32);
                c.output[position] += if value.is_normal() { value } else { 0.0 };
            }
        }
    }
}
fn wrap(v: f32) -> f32 {
    (v + PI).rem_euclid(TAU) - PI
}
fn interpolate(v: &[f32], index: f32) -> f32 {
    let index = index.clamp(0.0, (v.len() - 2) as f32);
    let i = index as usize;
    v[i] + (v[i + 1] - v[i]) * (index - i as f32)
}
