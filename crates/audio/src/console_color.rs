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

pub struct ConsoleColor {
    rate: f32,
    params: [f32; PARAM_COUNT],
    low: [f32; 2],
    held: [f32; 2],
    phase: f64,
    echo: Box<[[f32; 2]]>,
    echo_at: usize,
    mix: f32,
}
impl ConsoleColor {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            rate: sample_rate.max(1.0),
            params: DEFAULTS,
            low: [0.0; 2],
            held: [0.0; 2],
            phase: 1.0,
            echo: vec![[0.0; 2]; ECHO_CAP].into_boxed_slice(),
            echo_at: 0,
            mix: DEFAULTS[9],
        }
    }
    pub fn refresh(&mut self, params: &[f32], sample_rate: f32) {
        self.rate = sample_rate.max(1.0);
        for (dst, value) in self.params.iter_mut().zip(params) {
            if value.is_finite() { *dst = value.clamp(0.0, 1.0); }
        }
    }
    pub fn reset(&mut self) {
        self.low = [0.0; 2];
        self.held = [0.0; 2];
        self.phase = 1.0;
        self.echo.fill([0.0; 2]);
        self.echo_at = 0;
        self.mix = self.params[9];
    }
    pub fn console(&self) -> Console { Console::from_param(self.params[1]) }
    pub fn process_frame(&mut self, l: f32, r: f32) -> (f32, f32) {
        let mode = self.console();
        let native_rate = mode.native_rate();
        let internal_rate = (native_rate * (1.0 - self.params[3] * 0.75)).min(self.rate);
        let cutoff = (mode.cutoff() * (1.0 - self.params[4] * 0.70))
            .min(internal_rate * 0.45).min(self.rate * 0.45).max(90.0);
        let pole = 1.0 - (-std::f32::consts::TAU * cutoff / self.rate).exp();
        let x = [l, r];
        for (low, input) in self.low.iter_mut().zip(x) {
            *low += (input.clamp(-4.0, 4.0) - *low) * pole;
        }
        self.phase += f64::from(internal_rate / self.rate);
        if self.phase >= 1.0 {
            self.phase -= self.phase.floor();
            let bits = (mode.native_bits() as f32 - self.params[2] * 7.0).round().clamp(5.0, 16.0);
            let steps = 2.0f32.powf(bits - 1.0);
            for (held, low) in self.held.iter_mut().zip(self.low) {
                *held = (low * steps).round() / steps;
            }
        }
        // Feed an 8-tap echo FIR at the host rate. Presets intentionally differ:
        // SNES: dark FIR feedback echo; GBA: dry-ish; N64: shorter/clearer space;
        // PS1: multi-tap metallic reverb approximation (not the SPU's actual registers).
        let delay_ms: f32 = match mode {
            Console::Snes => 150.0,
            Console::Gba => 40.0,
            Console::N64 => 85.0,
            Console::Ps1 => 115.0,
        };
        let delay = ((self.rate * delay_ms * 0.001) as usize).clamp(8, ECHO_CAP - 9);
        const SNES: [f32; 8] = [0.48, 0.28, 0.13, 0.06, 0.03, 0.01, 0.005, 0.005];
        const PS1: [f32; 8] = [0.39, 0.0, 0.22, 0.0, 0.17, 0.0, 0.12, 0.10];
        const OTHER: [f32; 8] = [0.75, 0.16, 0.06, 0.02, 0.01, 0.0, 0.0, 0.0];
        let taps = match mode { Console::Snes => &SNES, Console::Ps1 => &PS1, _ => &OTHER };
        let mut tail = [0.0_f32; 2];
        for (i, gain) in taps.iter().enumerate() {
            let at = (self.echo_at + ECHO_CAP - delay - i) % ECHO_CAP;
            tail[0] += self.echo[at][0] * gain;
            tail[1] += self.echo[at][1] * gain;
        }
        let feedback = self.params[6] * 0.65;
        let echo_level = self.params[5] * 0.50;
        let drive = 1.0 + 4.0 * self.params[8];
        let width = self.params[7] * 2.0;
        let mono = 0.5 * (self.held[0] + self.held[1]);
        let wet_l = mono + (self.held[0] - mono) * width;
        let wet_r = mono + (self.held[1] - mono) * width;
        let wet = [wet_l, wet_r];
        let mut output = [0.0_f32; 2];
        let out_gain = 10.0_f32.powf((self.params[10] - 0.5) * 24.0 / 20.0);
        self.mix += (self.params[9] - self.mix) * (1.0 - (-1.0 / (0.008 * self.rate)).exp());
        for ch in 0..2 {
            // Do not feed dry bypass gain into the echo. Protect against runaway feedback.
            self.echo[self.echo_at][ch] = (wet[ch] + tail[ch] * feedback).clamp(-1.2, 1.2);
            let colored = ((wet[ch] + tail[ch] * echo_level) * drive).tanh() / drive.tanh();
            output[ch] = (x[ch] + (colored - x[ch]) * self.mix) * out_gain;
        }
        self.echo_at = (self.echo_at + 1) % ECHO_CAP;
        (output[0], output[1])
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
