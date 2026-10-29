//! Gen: six-voice, four-operator Genesis-inspired FM synth with SN76489-style PSG.
//! This musical instrument is not a cycle-accurate YM2612/SN76489 emulator.
//! No allocations, mutexes, filesystem accesses, or sample loading in the audio callback.
mod psg;
mod ym2612;
use crate::{ProcessContext, instrument::Instrument};
use psg::Psg;
use std::f32::consts::TAU;
use ym2612::Ym2612;

pub const NUM_PARAMS: usize = 106;
pub const MODEL: usize = 56;
pub const REGION: usize = 57;
pub const PSG_MODEL: usize = 58;
pub const EXTRA_START: usize = 62;
pub const EXTRA_STRIDE: usize = 10;
pub const OP_START: usize = 12;
pub const OP_STRIDE: usize = 8;
pub const DEFAULTS: [f32; NUM_PARAMS] = defaults();
const fn defaults() -> [f32; NUM_PARAMS] {
    let mut p = [0.0; NUM_PARAMS];
    p[0] = 1.0; // Enable
    p[1] = 0.0; // Algorithm (serial four-op)
    p[2] = 0.30; // Feedback
    p[3] = 0.65; // Master output
    p[4] = 0.88; // FM blend
    p[5] = 0.24; // PSG blend
    p[6] = 0.25; // LFO rate
    p[7] = 0.0; // Pitch LFO
    p[8] = 0.0; // Amplitude LFO
    p[9] = 0.0; // Saturation
    p[10] = 0.0; // DAC resolution
    p[11] = 0.5; // Width
    let mut op = 0;
    while op < 4 {
        let i = OP_START + op * OP_STRIDE;
        p[i] = 1.0 / 11.0; // frequency ratio selector, 1x default
        p[i + 1] = 0.5; // +/- 100 cents detune
        p[i + 2] = if op == 3 { 0.82 } else { 0.55 };
        p[i + 3] = 0.06; // attack
        p[i + 4] = 0.32; // decay
        p[i + 5] = 0.7; // sustain
        p[i + 6] = 0.32; // release
        p[i + 7] = 0.0; // LFO AM
        op += 1;
    }
    p[44] = 0.55; // PSG tone A
    p[45] = 0.28; // PSG tone B
    p[46] = 0.16; // PSG tone C
    p[47] = 0.0; // PSG noise
    p[48] = 0.0; // white / periodic
    p[49] = 0.4; // noise frequency
    p[50] = 0.5; // PSG stereo
    p[51] = 0.25; // pitch bend range
    p[52] = 0.0; // glide
    p[53] = 0.0; // release tail
    p[54] = 0.0; // FM+PSG/FM/PSG
    p[55] = 0.5; // trim
    p[61] = 1.0; // chip velocity sensitivity
    let mut op = 0;
    while op < 4 {
        let i = EXTRA_START + op * EXTRA_STRIDE;
        p[i] = 28.0 / 31.0;
        p[i + 1] = 8.0 / 31.0;
        p[i + 3] = 6.0 / 15.0;
        p[i + 4] = 6.0 / 15.0;
        p[i + 6] = if op == 3 { 0.0 } else { 32.0 / 127.0 };
        p[i + 9] = 1.0 / 15.0;
        op += 1;
    }
    p
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Stage {
    Off,
    Attack,
    Decay,
    Sustain,
    Release,
}
#[derive(Clone, Copy)]
struct Operator {
    phase: f32,
    env: f32,
    stage: Stage,
}
impl Default for Operator {
    fn default() -> Self {
        Self {
            phase: 0.0,
            env: 0.0,
            stage: Stage::Off,
        }
    }
}
impl Operator {
    fn on(&mut self) {
        self.phase = 0.0;
        self.env = 0.0;
        self.stage = Stage::Attack;
    }
    fn off(&mut self) {
        if self.stage != Stage::Off {
            self.stage = Stage::Release;
        }
    }
    fn advance(&mut self, params: &[f32; NUM_PARAMS], index: usize, rate: f32) -> f32 {
        let i = OP_START + OP_STRIDE * index;
        let attack = 0.002 + params[i + 3] * params[i + 3] * 3.0;
        let decay = 0.006 + params[i + 4] * params[i + 4] * 5.0;
        let sustain = params[i + 5];
        let release = (0.008 + params[i + 6] * params[i + 6] * 7.0) * (1.0 + params[53] * 3.0);
        match self.stage {
            Stage::Attack => {
                self.env = (self.env + 1.0 / (rate * attack)).min(1.0);
                if self.env >= 1.0 {
                    self.stage = Stage::Decay;
                }
            }
            Stage::Decay => {
                self.env = (self.env - (1.0 - sustain) / (rate * decay)).max(sustain);
                if self.env <= sustain {
                    self.stage = Stage::Sustain;
                }
            }
            Stage::Sustain => {}
            Stage::Release => {
                self.env = (self.env - 1.0 / (rate * release)).max(0.0);
                if self.env <= 0.0 {
                    self.stage = Stage::Off;
                }
            }
            Stage::Off => self.env = 0.0,
        }
        self.env
    }
}
#[derive(Clone, Copy)]
struct Voice {
    key: Option<u8>,
    midi: u8,
    velocity: f32,
    pan: f32,
    age: u64,
    base_hz: f32,
    glide_hz: f32,
    feedback: f32,
    deferred: bool,
    ops: [Operator; 4],
    last_sample: f32,
    retrigger_sample: f32,
    fade_left: usize,
    fade_total: usize,
}
impl Default for Voice {
    fn default() -> Self {
        Self {
            key: None,
            midi: 0,
            velocity: 0.0,
            pan: 0.5,
            age: 0,
            base_hz: 440.0,
            glide_hz: 440.0,
            feedback: 0.0,
            deferred: false,
            ops: [Operator::default(); 4],
            last_sample: 0.0,
            retrigger_sample: 0.0,
            fade_left: 0,
            fade_total: 1,
        }
    }
}
impl Voice {
    fn playing(&self) -> bool {
        self.ops.iter().any(|o| o.stage != Stage::Off)
    }
    fn on(&mut self, midi: u8, key: u8, velocity: f32, pan: f32, age: u64, glide: bool) {
        let next = midi_frequency(key);
        self.glide_hz = if glide && self.playing() {
            self.glide_hz
        } else {
            next
        };
        self.base_hz = next;
        self.midi = midi;
        self.key = Some(key);
        self.velocity = velocity.clamp(0.0, 1.0);
        self.pan = pan.clamp(0.0, 1.0);
        self.age = age;
        self.feedback = 0.0;
        self.deferred = false;
        for op in &mut self.ops {
            op.on();
        }
    }
    fn off(&mut self) {
        self.key = None;
        self.deferred = false;
        for op in &mut self.ops {
            op.off();
        }
    }
    fn render(&mut self, params: &[f32; NUM_PARAMS], rate: f32, lfo: f32, bend: f32) -> f32 {
        if !self.playing() {
            return 0.0;
        }
        let glide = params[52] * params[52] * 0.4;
        if glide < 0.00001 {
            self.glide_hz = self.base_hz;
        } else {
            self.glide_hz += (self.base_hz - self.glide_hz) / (glide * rate).max(1.0);
        }
        let pitch_lfo = lfo * params[7] * 0.7;
        let fundamental = self.glide_hz * 2.0f32.powf((bend + pitch_lfo) / 12.0);
        let mut output = [0.0_f32; 4];
        let feedback = self.feedback * params[2] * 6.0;
        let algorithm = (params[1] * 7.0).round() as u8;
        for i in 0..4 {
            let offset = OP_START + i * OP_STRIDE;
            // Musical multiple from 0.5x to 12x, quantized to preserve recognizable FM ratios.
            const RATIOS: [f32; 12] =
                [0.5, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 12.0];
            let ratio = RATIOS[((params[offset] * 11.0).round() as usize).min(11)];
            let cents = (params[offset + 1] - 0.5) * 200.0;
            let freq = (fundamental * ratio * 2.0f32.powf(cents / 1200.0)).min(rate * 0.48);
            let env = self.ops[i].advance(params, i, rate);
            let amp_lfo = 1.0 - params[offset + 7] * params[8] * (lfo + 1.0) * 0.5;
            // Eight routings approximate the YM2612's musical algorithm families.
            let modulation = match (algorithm, i) {
                (_, 0) => feedback,
                (0, 1) => output[0],
                (0, 2) => output[1],
                (0, 3) => output[2],
                (1, 1) => output[0],
                (1, 2) => output[0],
                (1, 3) => output[1] + output[2],
                (2, 1) => output[0],
                (2, 3) => output[1] + output[2],
                (3, 2) => output[0] + output[1],
                (3, 3) => output[2],
                (4, 1) => output[0],
                (4, 3) => output[2],
                (5, 1..=3) => output[0],
                (6, 1) => output[0],
                _ => 0.0,
            };
            let intensity = (3.0 * modulation).clamp(-25.0, 25.0);
            let phase = self.ops[i].phase + intensity;
            let sample = phase.sin() * env * params[offset + 2] * amp_lfo;
            output[i] = sample;
            self.ops[i].phase = (self.ops[i].phase + TAU * freq / rate).rem_euclid(TAU);
        }
        self.feedback = output[0] * 0.7 + self.feedback * 0.3;
        let out = match algorithm {
            0..=3 => output[3],
            4 => (output[1] + output[3]) * 0.65,
            5 => (output[1] + output[2] + output[3]) * 0.48,
            6 => (output[1] + output[2] + output[3]) * 0.48,
            _ => output.iter().sum::<f32>() * 0.3,
        };
        let mut sample = (out * self.velocity).clamp(-4.0, 4.0);
        if self.fade_left > 0 {
            let weight = self.fade_left as f32 / self.fade_total as f32;
            sample += (self.retrigger_sample - sample) * weight;
            self.fade_left -= 1;
        }
        self.last_sample = sample;
        sample
    }
}
fn midi_frequency(key: u8) -> f32 {
    440.0 * 2.0f32.powf((key as f32 - 69.0) / 12.0)
}
/// SN76489-compatible Sega 16-bit LFSR; white=true uses the Genesis XOR taps.
fn psg_noise_step(lfsr: u16, white: bool) -> u16 {
    let feedback = if white {
        (lfsr ^ (lfsr >> 3)) & 1
    } else {
        lfsr & 1
    };
    (lfsr >> 1) | (feedback << 15)
}

/// Bandlimiting step correction for PSG square-wave discontinuities.
fn poly_blep(t: f32, dt: f32) -> f32 {
    if t < dt {
        let x = t / dt;
        x + x - x * x - 1.0
    } else if t > 1.0 - dt {
        let x = (t - 1.0) / dt;
        x * x + x + x + 1.0
    } else {
        0.0
    }
}

#[derive(Clone, Copy)]
struct Tone {
    key: Option<u8>,
    midi: u8,
    phase: f32,
    pan: f32,
    velocity: f32,
    age: u64,
    deferred: bool,
}
impl Default for Tone {
    fn default() -> Self {
        Self {
            key: None,
            midi: 0,
            phase: 0.0,
            pan: 0.5,
            velocity: 0.0,
            age: 0,
            deferred: false,
        }
    }
}
pub struct Gen {
    rate: f32,
    params: [f32; NUM_PARAMS],
    voices: [Voice; 6],
    tones: [Tone; 3],
    pedals: [bool; 16],
    age: u64,
    bend: [f32; 16],
    lfo_phase: f32,
    noise: u16,
    noise_phase: f32,
    noise_on: bool,
    smooth_gain: f32,
    telemetry: [f32; 27],
    chip: Ym2612,
    clocked_psg: Psg,
    dirty: bool,
    control_tick: usize,
    last_hz: [Option<f32>; 16],
    mix_weights: [f32; 2],
    rendered: bool,
    last_output: [f32; 2],
    switch_output: [f32; 2],
    switch_left: usize,
}
impl Gen {
    pub fn new(rate: f32) -> Self {
        let rate = if rate.is_finite() && (8000.0..=192000.0).contains(&rate) {
            rate
        } else {
            48000.0
        };
        Self {
            rate,
            params: DEFAULTS,
            voices: [Voice::default(); 6],
            tones: [Tone::default(); 3],
            pedals: [false; 16],
            age: 0,
            bend: [0.0; 16],
            lfo_phase: 0.0,
            noise: 0x8000,
            noise_phase: 0.0,
            noise_on: false,
            smooth_gain: 0.0,
            telemetry: [0.0; 27],
            chip: Ym2612::new(rate),
            clocked_psg: Psg::new(rate),
            dirty: true,
            control_tick: 0,
            last_hz: [None; 16],
            mix_weights: [1.0; 2],
            rendered: false,
            last_output: [0.0; 2],
            switch_output: [0.0; 2],
            switch_left: 0,
        }
    }
    fn start(&mut self, midi: u8, key: u8, velocity: f32, pan: f32) {
        if midi >= 16 || key >= 128 || !velocity.is_finite() || !pan.is_finite() {
            return;
        }
        if velocity <= 0.0 {
            self.release(Some(midi), key);
            return;
        }
        let pan = pan.clamp(0.0, 1.0);
        self.age = self.age.wrapping_add(1);
        let idx = self
            .voices
            .iter()
            .position(|v| v.key == Some(key) && v.midi == midi)
            .or_else(|| {
                self.voices.iter().enumerate().position(|(i, v)| {
                    v.key.is_none()
                        && if self.params[MODEL] >= 0.5 {
                            !self.chip.active(i)
                        } else {
                            !v.playing()
                        }
                })
            })
            .unwrap_or_else(|| {
                self.voices
                    .iter()
                    .enumerate()
                    .min_by_key(|(_, v)| v.age)
                    .map_or(0, |(i, _)| i)
            });
        let old_sample = self.voices[idx].last_sample;
        let retrigger = self.voices[idx].playing() && self.params[MODEL] < 0.5;
        self.voices[idx].on(midi, key, velocity, pan, self.age, self.params[52] > 0.0);
        if retrigger {
            let v = &mut self.voices[idx];
            v.retrigger_sample = old_sample;
            v.fade_total = (self.rate * 0.004) as usize;
            v.fade_left = v.fade_total;
        }
        if self.params[52] > 0.0
            && let Some(hz) = self.last_hz[midi as usize]
        {
            self.voices[idx].glide_hz = hz;
        }
        self.last_hz[midi as usize] = Some(midi_frequency(key));
        if self.params[MODEL] >= 0.5 {
            self.configure_chip(idx);
            self.chip.key(idx, true);
        }
        let idx = self
            .tones
            .iter()
            .position(|t| t.key == Some(key) && t.midi == midi)
            .or_else(|| self.tones.iter().position(|t| t.key.is_none()))
            .unwrap_or_else(|| {
                self.tones
                    .iter()
                    .enumerate()
                    .min_by_key(|(_, t)| t.age)
                    .map_or(0, |(i, _)| i)
            });
        self.tones[idx] = Tone {
            key: Some(key),
            midi,
            phase: 0.0,
            pan,
            velocity,
            age: self.age,
            deferred: false,
        };
        self.noise_on = true;
    }
    fn release(&mut self, midi: Option<u8>, key: u8) {
        for (i, v) in self.voices.iter_mut().enumerate() {
            if v.key == Some(key) && midi.is_none_or(|m| v.midi == m) {
                if self.pedals[v.midi as usize] {
                    v.deferred = true;
                } else {
                    v.off();
                    self.chip.key(i, false);
                }
            }
        }
        for tone in &mut self.tones {
            if tone.key == Some(key) && midi.is_none_or(|m| tone.midi == m) {
                if self.pedals[tone.midi as usize] {
                    tone.deferred = true;
                } else {
                    tone.key = None;
                    tone.deferred = false;
                }
            }
        }
        self.noise_on = self.tones.iter().any(|t| t.key.is_some());
    }
    fn configure_chip(&mut self, ch: usize) {
        self.chip.set_pal(self.params[REGION] >= 0.5, self.rate);
        let bank = if ch >= 3 { 0x100 } else { 0 };
        let channel = (ch % 3) as u16;
        for op in 0..4 {
            let e = EXTRA_START + op * EXTRA_STRIDE;
            let register = bank + channel + [0, 8, 4, 12][op];
            let q = |p: usize, max: f32| (self.params[p] * max).round() as u8;
            let velocity = (1.0 - self.params[61] + self.params[61] * self.voices[ch].velocity)
                .max(1.0 / 127.0);
            let tl = (f32::from(q(e + 6, 127.0)) - 20.0 * velocity.log10() / 0.75)
                .round()
                .clamp(0.0, 127.0) as u8;
            self.chip
                .write(0x30 + register, (q(e + 7, 7.0) << 4) | q(e + 9, 15.0));
            self.chip.write(0x40 + register, tl);
            self.chip
                .write(0x50 + register, (q(e + 5, 3.0) << 6) | q(e, 31.0));
            self.chip.write(
                0x60 + register,
                (u8::from(self.params[102 + op] >= 0.5) << 7) | q(e + 1, 31.0),
            );
            self.chip.write(0x70 + register, q(e + 2, 31.0));
            let release = (f32::from(q(e + 4, 15.0)) / (1.0 + 3.0 * self.params[53])).round() as u8;
            self.chip
                .write(0x80 + register, (q(e + 3, 15.0) << 4) | release);
            let ssg = q(e + 8, 8.0);
            self.chip
                .write(0x90 + register, if ssg == 0 { 0 } else { 8 | (ssg - 1) });
        }
        let q = |p: usize, max: f32| (self.params[p] * max).round() as u8;
        self.chip
            .write(bank + 0xb0 + channel, (q(2, 7.0) << 3) | q(1, 7.0));
        let pan = (self.voices[ch].pan - 0.5) * self.params[11] + 0.5;
        self.chip.pan(ch, pan);
        self.chip
            .write(bank + 0xb4 + channel, 0xc0 | (q(8, 3.0) << 4) | q(7, 7.0));
        const LFO: [f32; 8] = [3.82, 5.33, 5.77, 6.11, 6.60, 9.23, 46.11, 69.22];
        let pick = q(60, 8.0);
        let lfo = if pick > 0 {
            pick - 1
        } else {
            let target = 0.1 + self.params[6] * 19.9;
            LFO.iter()
                .enumerate()
                .min_by(|a, b| (*a.1 - target).abs().total_cmp(&(*b.1 - target).abs()))
                .map_or(0, |(i, _)| i as u8)
        };
        self.chip.write(
            0x22,
            if self.params[7] > 0.0 || self.params[8] > 0.0 {
                8 | lfo
            } else {
                0
            },
        );
        self.chip_frequency(ch);
    }
    fn chip_frequency(&mut self, ch: usize) {
        let v = &self.voices[ch];
        let bend = self.bend[v.midi as usize] * (1.0 + 12.0 * self.params[51]);
        let hz = v.glide_hz * 2.0f32.powf(bend / 12.0);
        let mut block = 0u8;
        let clock = self.chip.clock() / 144.0;
        while block < 7 && f64::from(hz) * 2.0f64.powi(21 - i32::from(block)) / clock > 2047.0 {
            block += 1;
        }
        let fnum = (f64::from(hz) * 2.0f64.powi(21 - i32::from(block)) / clock)
            .round()
            .clamp(0.0, 2047.0) as u16;
        let bank = if ch >= 3 { 0x100 } else { 0 };
        let channel = (ch % 3) as u16;
        self.chip
            .write(bank + 0xa4 + channel, (block << 3) | ((fnum >> 8) as u8));
        self.chip.write(bank + 0xa0 + channel, fnum as u8);
    }
    fn sync_registers(&mut self) {
        if self.dirty {
            self.chip.set_pal(self.params[REGION] >= 0.5, self.rate);
            self.clocked_psg.set_pal(self.params[REGION] >= 0.5);
            if self.params[MODEL] >= 0.5 {
                for ch in 0..6 {
                    if self.voices[ch].key.is_some() || self.chip.active(ch) {
                        self.configure_chip(ch);
                    }
                }
            }
            self.dirty = false;
        }
        if self.params[PSG_MODEL] >= 0.5 {
            for (i, t) in self.tones.iter().enumerate() {
                if let Some(key) = t.key {
                    let bend = self.bend[t.midi as usize] * (1.0 + 12.0 * self.params[51]);
                    let hz = midi_frequency(key) * 2.0f32.powf(bend / 12.0);
                    let pan = ((t.pan - 0.5) * self.params[50] + 0.5).clamp(0.0, 1.0);
                    self.clocked_psg
                        .tone(i, hz, self.params[44 + i] * t.velocity, pan);
                } else {
                    self.clocked_psg.volume(i, 0.0);
                }
            }
            let kind =
                (self.params[59] * 3.0).round() as u8 | if self.params[48] < 0.5 { 4 } else { 0 };
            self.clocked_psg
                .noise(kind, if self.noise_on { self.params[47] } else { 0.0 });
        }
    }
    fn render_psg(&mut self) -> (f32, f32) {
        let mut lr = [0.0f32; 2];
        for i in 0..3 {
            let tone = &mut self.tones[i];
            if let Some(key) = tone.key {
                let bend = self.bend[tone.midi as usize] * (1.0 + 12.0 * self.params[51]);
                let hz = midi_frequency(key) * 2.0f32.powf(bend / 12.0);
                // PolyBLEP suppresses the most severe stair-step aliasing at PSG square edges.
                let dt = (hz / self.rate).clamp(0.000001, 0.49);
                tone.phase = (tone.phase + dt).fract();
                let signal = (if tone.phase < 0.5 { 1.0 } else { -1.0 })
                    + poly_blep(tone.phase, dt)
                    - poly_blep((tone.phase + 0.5).fract(), dt);
                let gain = self.params[44 + i] * tone.velocity * 0.65;
                let pos = (tone.pan - 0.5) * self.params[50] + 0.5;
                lr[0] += signal * gain * (1.0 - pos);
                lr[1] += signal * gain * pos;
            }
        }
        if self.noise_on && self.params[47] > 0.0 {
            let hz = 160.0 * 2.0f32.powf(8.0 * self.params[49]);
            self.noise_phase += hz / self.rate;
            // Genesis PSG: 16-bit shift register, white noise taps 0 and 3 (0x0009)
            // and periodic noise reinserts bit 0 at bit 15.
            // Advance every elapsed tick, including at low host sample rates.
            while self.noise_phase >= 1.0 {
                self.noise_phase -= 1.0;
                self.noise = psg_noise_step(self.noise, self.params[48] < 0.5);
            }
            let value = (if self.noise & 1 != 0 { 1.0 } else { -1.0 }) * self.params[47] * 0.15;
            lr[0] += value;
            lr[1] += value;
        }
        (lr[0], lr[1])
    }
}
impl Instrument for Gen {
    fn set_parameter(&mut self, i: usize, value: f32) {
        if i < NUM_PARAMS && value.is_finite() {
            let value = value.clamp(0.0, 1.0);
            if self.params[i] != value {
                if i == MODEL || i == PSG_MODEL {
                    let previous = self.last_output;
                    self.reset();
                    self.switch_output = previous;
                    self.switch_left = (self.rate * 0.005) as usize;
                }
                self.params[i] = value;
                self.dirty = true;
            }
        }
    }
    fn note_on(&mut self, key: u8, velocity: f32, pan: f32) {
        self.start(0, key, velocity, pan);
    }
    fn note_off(&mut self, key: u8) {
        self.release(None, key);
    }
    fn note_on_midi(&mut self, midi: u8, key: u8, velocity: f32, pan: f32) {
        self.start(midi, key, velocity, pan);
    }
    fn note_off_midi(&mut self, midi: u8, key: u8) {
        self.release(Some(midi), key);
    }
    fn set_midi_controller(&mut self, midi: u8, index: usize, value: f32) {
        if midi >= 16 || !value.is_finite() {
            return;
        }
        if index == 4 {
            self.bend[midi as usize] = value.clamp(0.0, 1.0) * 2.0 - 1.0;
            self.dirty = true;
        }
        if index == 71 {
            self.pedals[midi as usize] = value >= 0.5;
            if value < 0.5 {
                // Pedal-up releases only deferred note-offs; keys still held keep sounding.
                for (i, v) in self.voices.iter_mut().enumerate() {
                    if v.midi == midi && v.deferred {
                        v.off();
                        self.chip.key(i, false);
                    }
                }
                for t in &mut self.tones {
                    if t.midi == midi && t.deferred {
                        t.key = None;
                        t.deferred = false;
                    }
                }
                self.noise_on = self.tones.iter().any(|t| t.key.is_some());
            }
        }
        if index == 127 || index == 130 {
            if index == 127 {
                self.pedals[midi as usize] = false;
            }
            for key in 0..128 {
                self.release(Some(midi), key);
            }
            if index == 127 {
                for (i, v) in self.voices.iter_mut().enumerate() {
                    if v.midi == midi {
                        *v = Voice::default();
                        self.chip.key(i, false);
                        self.chip.write(
                            if i >= 3 {
                                0x1b4 + (i - 3) as u16
                            } else {
                                0xb4 + i as u16
                            },
                            0,
                        );
                    }
                }
            }
        }
    }
    fn analysis(&self) -> &[f32] {
        &self.telemetry
    }
    fn all_notes_off(&mut self) {
        for (i, v) in self.voices.iter_mut().enumerate() {
            v.off();
            self.chip.key(i, false);
        }
        for t in &mut self.tones {
            t.key = None;
        }
        self.noise_on = false;
        self.pedals.fill(false);
    }
    fn reset(&mut self) {
        self.voices.fill(Voice::default());
        self.tones.fill(Tone::default());
        self.noise = 0x8000;
        self.noise_phase = 0.0;
        self.lfo_phase = 0.0;
        self.noise_on = false;
        self.telemetry.fill(0.0);
        self.pedals.fill(false);
        self.smooth_gain = 0.0;
        self.chip.reset();
        self.clocked_psg.reset();
        self.last_hz.fill(None);
        self.dirty = true;
        self.rendered = false;
        self.last_output = [0.0; 2];
        self.switch_left = 0;
    }
    fn set_controller(&mut self, index: usize, value: f32) {
        if index == 4 && value.is_finite() {
            for midi in 0..16 {
                self.set_midi_controller(midi, index, value);
            }
        }
    }
    fn process(&mut self, left: &mut [f32], right: &mut [f32], _: ProcessContext) {
        self.sync_registers();
        let chip_mode = self.params[MODEL] >= 0.5;
        let clocked_psg = self.params[PSG_MODEL] >= 0.5;
        let trim = 10.0f32.powf((self.params[55] - 0.5) * 24.0 / 20.0);
        let drive = 1.0 + self.params[9] * 12.0;
        let drive_norm = drive.tanh();
        let quant = (1u32 << (16 - (self.params[10] * 7.0).round() as u32)) as f32;
        let mut output_peak = [0.0f32; 2];
        let lfo_rate = 0.1 + self.params[6] * 19.9;
        let fm_on = self.params[54] < 0.75;
        let psg_on = self.params[54] < 0.25 || self.params[54] >= 0.75;
        if !self.rendered {
            self.mix_weights = [f32::from(fm_on), f32::from(psg_on)];
            self.rendered = true;
        }
        for (l, r) in left.iter_mut().zip(right.iter_mut()) {
            self.lfo_phase = (self.lfo_phase + lfo_rate / self.rate).fract();
            let lfo = (self.lfo_phase * TAU).sin();
            let mut fm = [0.0f32; 2];
            if chip_mode {
                let glide = self.params[52] * self.params[52] * 0.4;
                for v in &mut self.voices {
                    if v.key.is_some() {
                        if glide < 0.00001 {
                            v.glide_hz = v.base_hz;
                        } else {
                            v.glide_hz += (v.base_hz - v.glide_hz) / (glide * self.rate).max(1.0);
                        }
                    }
                }
                if self.control_tick == 0 && glide > 0.0 {
                    for i in 0..6 {
                        if self.voices[i].key.is_some() {
                            self.chip_frequency(i);
                        }
                    }
                }
                self.control_tick = (self.control_tick + 1) % (self.rate as usize / 750).max(1);
                let sample = self.chip.render();
                if self.voices.iter().any(|v| v.key.is_some())
                    || (0..6).any(|i| self.chip.active(i))
                {
                    fm = sample;
                }
            } else {
                for v in &mut self.voices {
                    let bend = self.bend[v.midi as usize] * (1.0 + 12.0 * self.params[51]);
                    let signal = v.render(&self.params, self.rate, lfo, bend);
                    let pos = ((v.pan - 0.5) * self.params[11] + 0.5).clamp(0.0, 1.0);
                    fm[0] += signal * (1.0 - pos);
                    fm[1] += signal * pos;
                }
            }
            let psg = if clocked_psg {
                let v = self.clocked_psg.render();
                (v[0], v[1])
            } else {
                self.render_psg()
            };
            let targets = [f32::from(fm_on), f32::from(psg_on)];
            for (i, w) in self.mix_weights.iter_mut().enumerate() {
                *w += (targets[i] - *w) * 0.004;
            }
            self.smooth_gain += (self.params[3] - self.smooth_gain) * 0.002;
            for (dst, i) in [(l, 0usize), (r, 1usize)] {
                let mixed = fm[i] * self.params[4] * self.mix_weights[0]
                    + [psg.0, psg.1][i] * self.params[5] * self.mix_weights[1];
                let saturated = (mixed * drive).tanh() / drive_norm;
                let dac = (saturated * quant).round() / quant;
                let mut sample = (dac * trim * self.smooth_gain * 0.42).clamp(-1.0, 1.0);
                if self.switch_left > 0 {
                    let weight = self.switch_left as f32 / (self.rate * 0.005).max(1.0);
                    sample += (self.switch_output[i] - sample) * weight;
                }
                self.last_output[i] = sample;
                output_peak[i] = output_peak[i].max(sample.abs());
                *dst += sample;
            }
            self.switch_left = self.switch_left.saturating_sub(1);
        }
        self.telemetry[24] = if chip_mode {
            self.chip.lfo_phase()
        } else {
            self.lfo_phase
        };
        self.telemetry[25..27].copy_from_slice(&output_peak);
        if chip_mode {
            self.chip.update_levels();
        }
        for (i, v) in self.voices.iter().enumerate() {
            self.telemetry[i] = f32::from(if chip_mode {
                self.chip.active(i)
            } else {
                v.playing()
            });
            self.telemetry[10 + i] = if chip_mode {
                (0..4).map(|op| self.chip.level(i, op)).fold(0.0, f32::max)
            } else {
                v.ops.iter().map(|o| o.env).fold(0.0, f32::max)
            };
        }
        let last = self
            .voices
            .iter()
            .enumerate()
            .max_by_key(|(_, v)| v.age)
            .map_or(0, |(i, _)| i);
        for op in 0..4 {
            self.telemetry[20 + op] = if chip_mode {
                self.chip.level(last, op)
            } else {
                self.voices[last].ops[op].env
            };
        }
        for i in 0..3 {
            self.telemetry[6 + i] = f32::from(self.tones[i].key.is_some());
            self.telemetry[16 + i] =
                self.telemetry[6 + i] * self.params[44 + i] * self.tones[i].velocity;
        }
        self.telemetry[9] = f32::from(self.noise_on);
        self.telemetry[19] = self.telemetry[9] * self.params[47];
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    const CTX: ProcessContext = ProcessContext {
        sample_rate: 48000.0,
        tempo_bpm: 120.0,
        start_beat: 0.0,
        playing: false,
    };
    #[test]
    fn fm_plays_without_nan_and_releases() {
        let mut g = Gen::new(48000.0);
        g.note_on(69, 1.0, 0.5);
        let (mut l, mut r) = (vec![0.0; 4096], vec![0.0; 4096]);
        g.process(&mut l, &mut r, CTX);
        assert!(l.iter().any(|x| x.abs() > 1e-4));
        assert!(l.iter().all(|x| x.is_finite()));
        g.note_off(69);
        assert!(g.voices.iter().all(|v| v.key.is_none()));
    }
    #[test]
    fn every_fm_algorithm_renders_audio() {
        for i in 0..8 {
            let mut g = Gen::new(48000.0);
            g.set_parameter(1, i as f32 / 7.0);
            g.note_on(60, 1.0, 0.5);
            let (mut l, mut r) = (vec![0.0; 4096], vec![0.0; 4096]);
            g.process(&mut l, &mut r, CTX);
            assert!(l.iter().any(|x| x.abs() > 0.00001), "algorithm {i}");
        }
    }
    #[test]
    fn pedal_does_not_cut_keys_that_are_still_held() {
        let mut g = Gen::new(48000.0);
        g.note_on_midi(1, 60, 0.8, 0.5);
        g.note_on_midi(1, 64, 0.8, 0.5);
        g.set_midi_controller(1, 71, 1.0); // CC64 sustain down
        g.note_off_midi(1, 60);
        assert!(g.voices.iter().any(|v| v.key == Some(60) && v.deferred));
        assert!(g.voices.iter().any(|v| v.key == Some(64) && !v.deferred));
        g.set_midi_controller(1, 71, 0.0); // pedal up
        assert!(!g.voices.iter().any(|v| v.key == Some(60)));
        assert!(g.voices.iter().any(|v| v.key == Some(64)));
        assert!(g.tones.iter().any(|t| t.key == Some(64)));
    }
    #[test]
    fn high_psg_notes_remain_finite() {
        let mut g = Gen::new(48000.0);
        g.set_parameter(54, 1.0);
        g.note_on(108, 1.0, 0.5);
        let (mut l, mut r) = (vec![0.0; 4096], vec![0.0; 4096]);
        g.process(&mut l, &mut r, CTX);
        assert!(l.iter().all(|x| x.is_finite()));
        assert!(l.iter().any(|x| x.abs() > 0.001));
    }
    #[test]
    fn genesis_noise_has_sixteen_bit_feedback_and_periodic_cycle() {
        let mut periodic = 0x8000u16;
        for _ in 0..16 {
            periodic = psg_noise_step(periodic, false);
        }
        assert_eq!(periodic, 0x8000, "periodic noise repeats every 16 shifts");
        let mut white = 0x8000u16;
        for _ in 0..40 {
            white = psg_noise_step(white, true);
        }
        assert_ne!(white, 0x8000, "white noise feedback uses taps 0 and 3");
    }
    #[test]
    fn psg_only_produces_square_wave() {
        let mut g = Gen::new(48000.0);
        g.set_parameter(54, 1.0);
        g.note_on(60, 1.0, 0.5);
        let (mut l, mut r) = (vec![0.0; 1024], vec![0.0; 1024]);
        g.process(&mut l, &mut r, CTX);
        assert!(l.iter().any(|x| x.abs() > 0.01));
    }
}
