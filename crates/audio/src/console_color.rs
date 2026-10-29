//! Console Color: console-output coloration for arbitrary audio, ID 56.
//! A *post-processing approximation*, not an emulation of game-console sample players.
//! SNES BRR, PS1 SPU ADPCM, N64 RSP microcode and GBA FIFO behavior require source-aware emulators.
//! Callback-safe: every buffer is allocated in new(), never in process_frame/refresh/reset.
pub const PARAM_COUNT: usize = 11;
pub const DEFAULTS: [f32; PARAM_COUNT] = [
    1.0, 0.0, 0.0, 0.0, 0.5, 0.3, 0.35, 0.5, 0.12, 1.0, 0.5,
];
const ECHO_CAP: usize = 32_768;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Console {
    Snes, Gba, N64, Ps1,
}
impl Console {
    fn from_param(value: f32) -> Self {
        match (value.clamp(0.0, 1.0) * 3.0).round() as u8 {
            1 => Self::Gba,
            2 => Self::N64,
            3 => Self::Ps1,
            _ => Self::Snes,
        }
    }
    fn native_rate(self) -> f32 {
        match self {
            Self::Snes => 32_000.0,
            Self::Gba => 32_768.0,
            Self::N64 => 32_000.0, // common game rate, not a universal N64 sample rate
            Self::Ps1 => 44_100.0,
        }
    }
    fn native_bits(self) -> i32 {
        match self {
            Self::Snes => 15,
            Self::Gba => 8, // GBA Direct Sound sample input resolution
            Self::N64 | Self::Ps1 => 16,
        }
    }
    fn cutoff(self) -> f32 {
        match self {
            Self::Snes => 13_000.0,
            Self::Gba => 9_000.0,
            Self::N64 => 14_500.0,
            Self::Ps1 => 17_500.0,
        }
    }
}

/// Precalculated, automation-updated settings: no exponentials or sample-rate changes per frame.
#[derive(Clone, Copy)]
struct Coeffs {
    mode: Console,
    step: f64,
    pole: f32,
    steps: f32,
    delay: usize,
    spacing: usize,
    fir: [f32; 8],
    feedback: f32,
    echo_level: f32,
    width: f32,
    drive: f32,
    drive_inv: f32,
    gain: f32,
    mix_alpha: f32,
}
impl Coeffs {
    fn calculate(p: &[f32; PARAM_COUNT], rate: f32) -> Self {
        let mode = Console::from_param(p[1]);
        let internal_rate = (mode.native_rate() * (1.0 - p[3] * 0.75)).min(rate);
        let cutoff = (mode.cutoff() * (1.0 - p[4] * 0.70))
            .min(internal_rate * 0.45)
            .min(rate * 0.45)
            .max(90.0);
        let delay_ms: f32 = match mode {
            Console::Snes => 150.0,
            Console::Gba => 40.0,
            Console::N64 => 85.0,
            Console::Ps1 => 115.0,
        };
        // FIR taps span internal sample intervals, not host-sample intervals.
        let spacing = ((rate / internal_rate).round() as usize).max(1);
        let longest = (7 * spacing).min(ECHO_CAP / 4);
        let delay = ((rate * delay_ms * 0.001) as usize)
            .clamp(8, ECHO_CAP - longest - 1);
        const SNES: [f32; 8] = [0.48, 0.28, 0.13, 0.06, 0.03, 0.01, 0.005, 0.005];
        const PS1: [f32; 8] = [0.39, 0.0, 0.22, 0.0, 0.17, 0.0, 0.12, 0.10];
        const OTHER: [f32; 8] = [0.75, 0.16, 0.06, 0.02, 0.01, 0.0, 0.0, 0.0];
        let fir = match mode {
            Console::Snes => SNES,
            Console::Ps1 => PS1,
            Console::Gba | Console::N64 => OTHER,
        };
        let drive = 1.0 + 4.0 * p[8];
        Self {
            mode,
            step: f64::from(internal_rate / rate),
            pole: 1.0 - (-std::f32::consts::TAU * cutoff / rate).exp(),
            steps: 2.0_f32.powf(
                (mode.native_bits() as f32 - p[2] * 7.0).round().clamp(5.0, 16.0) - 1.0
            ),
            delay,
            spacing,
            fir,
            feedback: p[6] * 0.65,
            echo_level: p[5] * 0.50,
            width: p[7] * 2.0,
            drive,
            drive_inv: 1.0 / drive.tanh(),
            gain: 10.0_f32.powf((p[10] - 0.5) * 24.0 / 20.0),
            mix_alpha: 1.0 - (-1.0 / (0.008 * rate)).exp(),
        }
    }
}
/// Causal cubic B-spline: an intentionally soft four-sample approximation.
/// NOT a substitute for the voice-level Gaussian interpolation in the SNES/PS1.
fn cubic_smooth(h: [f32; 4], t: f32) -> f32 {
    let u = 1.0 - t;
    (u * u * u * h[0]
        + (3.0 * t * t * t - 6.0 * t * t + 4.0) * h[1]
        + (-3.0 * t * t * t + 3.0 * t * t + 3.0 * t + 1.0) * h[2]
        + t * t * t * h[3]) / 6.0
}

pub struct ConsoleColor {
    rate: f32,
    params: [f32; PARAM_COUNT],
    low: [f32; 2],
    history: [[f32; 4]; 2],
    coeffs: Coeffs,
    phase: f64,
    echo: Box<[[f32; 2]]>,
    echo_at: usize,
    mix: f32,
}
impl ConsoleColor {
    pub fn new(sample_rate: f32) -> Self {
        let rate = sample_rate.max(1.0);
        Self {
            rate,
            params: DEFAULTS,
            low: [0.0; 2],
            history: [[0.0; 4]; 2],
            coeffs: Coeffs::calculate(&DEFAULTS, rate),
            phase: 1.0,
            echo: vec![[0.0; 2]; ECHO_CAP].into_boxed_slice(),
            echo_at: 0,
            mix: DEFAULTS[9],
        }
    }
    pub fn refresh(&mut self, params: &[f32], sample_rate: f32) {
        let old_mode = self.console();
        let rate = sample_rate.max(1.0);
        let changed_rate = self.rate != rate;
        self.rate = rate;
        for (dst, value) in self.params.iter_mut().zip(params) {
            if value.is_finite() {
                *dst = value.clamp(0.0, 1.0);
            }
        }
        self.coeffs = Coeffs::calculate(&self.params, self.rate);
        // Avoid tails from an old console profile contaminating the new one.
        // Transition from dry audio over 8 ms, avoiding a discontinuous switch.
        if changed_rate || old_mode != self.console() {
            self.reset();
            self.mix = 0.0;
        }
    }
    pub fn reset(&mut self) {
        self.low = [0.0; 2];
        self.history = [[0.0; 4]; 2];
        self.phase = 1.0;
        self.echo.fill([0.0; 2]);
        self.echo_at = 0;
        self.mix = self.params[9];
    }
    pub fn console(&self) -> Console { Console::from_param(self.params[1]) }

    pub fn process_frame(&mut self, l: f32, r: f32) -> (f32, f32) {
        let c = self.coeffs;
        // Defend the effect chain against an invalid upstream sample.
        let input = [
            if l.is_finite() { l } else { 0.0 },
            if r.is_finite() { r } else { 0.0 },
        ];
        for (filtered, sample) in self.low.iter_mut().zip(input) {
            *filtered += (sample.clamp(-4.0, 4.0) - *filtered) * c.pole;
        }
        self.phase += c.step;
        if self.phase >= 1.0 {
            self.phase -= self.phase.floor();
            for (history, filtered) in self.history.iter_mut().zip(self.low) {
                history.copy_within(1..4, 0);
                history[3] = (filtered * c.steps).round() / c.steps;
            }
        }
        let t = self.phase as f32;
        let mut held = [0.0; 2];
        for (out, history) in held.iter_mut().zip(self.history) {
            *out = match c.mode {
                // Direct Sound character: signed 8-bit samples and a held DAC output.
                Console::Gba => history[3],
                // N64 software audio often uses RSP resampling, but its exact filter
                // varies by microcode/game. This is intentionally only a linear color.
                Console::N64 => history[2] + (history[3] - history[2]) * t,
                // Softer cubic response for the SNES and PS1; not an SPC/SPU emulator.
                Console::Snes | Console::Ps1 => cubic_smooth(history, t),
            };
        }
        // The original SNES DSP's FIR runs at 32 kHz: keep taps separated
        // by approximately a single internal output sample, not a host frame.
        let mut tail = [0.0_f32; 2];
        for (i, gain) in c.fir.iter().enumerate() {
            let offset = c.delay + i * c.spacing;
            let at = (self.echo_at + ECHO_CAP - offset) % ECHO_CAP;
            tail[0] += self.echo[at][0] * gain;
            tail[1] += self.echo[at][1] * gain;
        }
        let mono = 0.5 * (held[0] + held[1]);
        let wet = [
            mono + (held[0] - mono) * c.width,
            mono + (held[1] - mono) * c.width,
        ];
        let mut result = [0.0; 2];
        self.mix += (self.params[9] - self.mix) * c.mix_alpha;
        for channel in 0..2 {
            // Saturate the feedback path to bound the energy under automation.
            self.echo[self.echo_at][channel] =
                (wet[channel] + tail[channel] * c.feedback).clamp(-1.2, 1.2);
            let colored = ((wet[channel] + tail[channel] * c.echo_level) * c.drive).tanh()
                * c.drive_inv;
            result[channel] =
                (input[channel] + (colored - input[channel]) * self.mix) * c.gain;
        }
        self.echo_at = (self.echo_at + 1) % ECHO_CAP;
        (result[0], result[1])
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn all_consoles_process_finite_stereo_at_common_rates() {
        for rate in [8_000.0, 32_000.0, 44_100.0, 48_000.0, 96_000.0] {
            let mut effect = ConsoleColor::new(rate);
            for mode in 0..4 {
                let mut p = DEFAULTS;
                p[1] = mode as f32 / 3.0;
                effect.refresh(&p, rate);
                effect.reset();
                for i in 0..2000 {
                    let (l,r) = effect.process_frame(if i == 0 { 0.8 } else { 0.0 }, 0.0);
                    assert!(l.is_finite() && r.is_finite(), "rate {rate} mode {mode}");
                }
            }
        }
    }
    #[test]
    fn cubic_resampler_is_constant_for_dc_input_and_continuous_at_tick() {
        for t in [0.0, 0.25, 0.5, 0.75, 1.0] {
            assert!((cubic_smooth([0.25; 4], t) - 0.25).abs() < 1e-6);
        }
        let before = cubic_smooth([0.1, 0.2, 0.3, 0.4], 1.0);
        let after = cubic_smooth([0.2, 0.3, 0.4, 0.5], 0.0);
        assert!((before-after).abs() < 1e-6);
    }
    #[test]
    fn preset_switch_clears_older_tails_and_restarts_wet_crossfade() {
        let mut fx = ConsoleColor::new(48_000.0);
        for _ in 0..1000 { fx.process_frame(0.6, 0.4); }
        let mut params = DEFAULTS;
        params[1] = 1.0;
        fx.refresh(&params, 48_000.0);
        assert!(fx.echo.iter().all(|frame| *frame == [0.0; 2]));
        assert_eq!(fx.mix, 0.0);
        let (a,b) = fx.process_frame(0.0,0.0);
        assert!(a.abs() < 0.001 && b.abs() < 0.001);
    }
    #[test]
    fn profiles_produce_distinct_impulse_responses() {
        let run = |mode: usize| {
            let mut fx = ConsoleColor::new(48_000.0);
            let mut p = DEFAULTS;
            p[1] = mode as f32 / 3.0;
            p[5] = 0.0;
            fx.refresh(&p, 48_000.0);
            (0..256).map(|i| fx.process_frame(if i==0 { 1.0 } else {0.0}, 0.0).0).collect::<Vec<_>>()
        };
        let snes = run(0);
        let gba = run(1);
        assert_ne!(snes, gba);
    }
    #[test]
    fn bypass_mix_is_identity_at_unity_gain() {
        let mut fx = ConsoleColor::new(48_000.0);
        let mut p = DEFAULTS; p[9] = 0.0;
        fx.refresh(&p, 48_000.0); fx.reset();
        for _ in 0..100 {
            let (l,r) = fx.process_frame(0.25, -0.6);
            assert!((l-0.25).abs() < 1e-6);
            assert!((r+0.6).abs() < 1e-6);
        }
    }
}
