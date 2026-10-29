//! Gen: six-voice, four-operator Genesis-inspired FM synth with SN76489-style PSG.
//! This musical instrument is not a cycle-accurate YM2612/SN76489 emulator.
//! No allocations, mutexes, filesystem accesses, or sample loading in the audio callback.
use std::f32::consts::TAU;
use crate::{ProcessContext, instrument::Instrument};

pub const NUM_PARAMS: usize = 56;
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
        p[i + 1] = 0.5;  // +/- 100 cents detune
        p[i + 2] = if op == 3 { 0.82 } else { 0.55 };
        p[i + 3] = 0.06; // attack
        p[i + 4] = 0.32; // decay
        p[i + 5] = 0.7;  // sustain
        p[i + 6] = 0.32; // release
        p[i + 7] = 0.0;  // LFO AM
        op += 1;
    }
    p[44] = 0.55; // PSG tone A
    p[45] = 0.28; // PSG tone B
    p[46] = 0.16; // PSG tone C
    p[47] = 0.0;  // PSG noise
    p[48] = 0.0;  // white / periodic
    p[49] = 0.4;  // noise frequency
    p[50] = 0.5;  // PSG stereo
    p[51] = 0.25; // pitch bend range
    p[52] = 0.0;  // glide
    p[53] = 0.0;  // release tail
    p[54] = 0.0;  // FM+PSG/FM/PSG
    p[55] = 0.5;  // trim
    p
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Stage { Off, Attack, Decay, Sustain, Release }
#[derive(Clone, Copy)]
struct Operator {
    phase: f32,
    env: f32,
    stage: Stage,
}
impl Default for Operator {
    fn default() -> Self { Self { phase: 0.0, env: 0.0, stage: Stage::Off } }
}
impl Operator {
    fn on(&mut self) { self.phase = 0.0; self.env = 0.0; self.stage = Stage::Attack; }
    fn off(&mut self) { if self.stage != Stage::Off { self.stage = Stage::Release; } }
    fn advance(&mut self, params: &[f32; NUM_PARAMS], index: usize, rate: f32) -> f32 {
        let i = OP_START + OP_STRIDE * index;
        let attack = 0.002 + params[i + 3] * params[i + 3] * 3.0;
        let decay = 0.006 + params[i + 4] * params[i + 4] * 5.0;
        let sustain = params[i + 5];
        let release = 0.008 + params[i + 6] * params[i + 6] * 7.0;
        match self.stage {
            Stage::Attack => {
                self.env = (self.env + 1.0 / (rate * attack)).min(1.0);
                if self.env >= 1.0 { self.stage = Stage::Decay; }
            }
            Stage::Decay => {
                self.env = (self.env - (1.0 - sustain) / (rate * decay)).max(sustain);
                if self.env <= sustain { self.stage = Stage::Sustain; }
            }
            Stage::Sustain => {}
            Stage::Release => {
                self.env = (self.env - 1.0 / (rate * release)).max(0.0);
                if self.env <= 0.0 { self.stage = Stage::Off; }
            }
            Stage::Off => self.env = 0.0,
        }
        self.env
    }
}
#[derive(Clone, Copy)]
struct Voice {
    key: Option<u8>, midi: u8, velocity: f32, pan: f32,
    age: u64, base_hz: f32, glide_hz: f32, feedback: f32,
    ops: [Operator; 4],
}
impl Default for Voice {
    fn default() -> Self {
        Self { key: None, midi: 0, velocity: 0.0, pan: 0.5,
            age: 0, base_hz: 440.0, glide_hz: 440.0,
            feedback: 0.0, ops: [Operator::default(); 4] }
    }
}
impl Voice {
    fn playing(&self) -> bool { self.ops.iter().any(|o| o.stage != Stage::Off) }
    fn on(&mut self, midi: u8, key: u8, velocity: f32, pan: f32, age: u64, glide: bool) {
        let next = midi_frequency(key);
        self.glide_hz = if glide && self.playing() { self.glide_hz } else { next };
        self.base_hz = next;
        self.midi = midi;
        self.key = Some(key);
        self.velocity = velocity.clamp(0.0, 1.0);
        self.pan = pan.clamp(0.0, 1.0);
        self.age = age;
        self.feedback = 0.0;
        for op in &mut self.ops { op.on(); }
    }
    fn off(&mut self) { self.key = None; for op in &mut self.ops { op.off(); } }
    fn render(&mut self, params: &[f32; NUM_PARAMS], rate: f32, lfo: f32, bend: f32) -> f32 {
        if !self.playing() { return 0.0; }
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
            const RATIOS: [f32; 12] = [0.5, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 12.0];
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
                (1, 1) => output[0], (1, 2) => output[0], (1, 3) => output[1] + output[2],
                (2, 1) => output[0], (2, 3) => output[1] + output[2],
                (3, 2) => output[0] + output[1], (3, 3) => output[2],
                (4, 1) => output[0], (4, 3) => output[2],
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
            0 | 1 | 2 | 3 => output[3],
            4 => (output[1] + output[3]) * 0.65,
            5 => (output[1] + output[2] + output[3]) * 0.48,
            6 => (output[1] + output[2] + output[3]) * 0.48,
            _ => output.iter().sum::<f32>() * 0.3,
        };
        (out * self.velocity).clamp(-4.0, 4.0)
    }
}
fn midi_frequency(key: u8) -> f32 { 440.0 * 2.0f32.powf((key as f32 - 69.0) / 12.0) }

#[derive(Clone, Copy)]
struct Tone { key: Option<u8>, midi: u8, phase: f32, pan: f32, velocity: f32, age: u64 }
impl Default for Tone {
    fn default() -> Self { Self { key: None, midi: 0, phase: 0.0, pan: 0.5, velocity: 0.0, age: 0 } }
}
pub struct Gen {
    rate: f32, params: [f32; NUM_PARAMS], voices: [Voice; 6],
    tones: [Tone; 3], pedals: [bool; 16],
    age: u64, bend: [f32; 16], lfo_phase: f32, noise: u16, noise_phase: f32,
    noise_on: bool, smooth_gain: f32, telemetry: [f32; 10],
}
impl Gen {
    pub fn new(rate: f32) -> Self {
        let rate = if rate.is_finite() && (8000.0..=192000.0).contains(&rate) { rate } else { 48000.0 };
        Self { rate, params: DEFAULTS, voices: [Voice::default(); 6],
            tones: [Tone::default(); 3], pedals: [false; 16],
            age: 0, bend: [0.0; 16], lfo_phase: 0.0, noise: 0x4000,
            noise_phase: 0.0, noise_on: false, smooth_gain: 0.0,
            telemetry: [0.0; 10] }
    }
    fn start(&mut self, midi: u8, key: u8, velocity: f32, pan: f32) {
        if midi >= 16 || key >= 128 || !velocity.is_finite() || velocity <= 0.0 { return; }
        self.age = self.age.wrapping_add(1);
        let idx = self.voices.iter().position(|v| v.key.is_none() && !v.playing())
            .or_else(|| self.voices.iter().position(|v| v.key == Some(key) && v.midi == midi))
            .unwrap_or_else(|| self.voices.iter().enumerate().min_by_key(|(_,v)| v.age).map_or(0, |(i,_)| i));
        self.voices[idx].on(midi, key, velocity, pan, self.age, self.params[52] > 0.0);
        let idx = self.tones.iter().position(|t| t.key.is_none())
            .unwrap_or_else(|| self.tones.iter().enumerate().min_by_key(|(_,t)| t.age).map_or(0, |(i,_)| i));
        self.tones[idx] = Tone { key: Some(key), midi, phase: 0.0, pan, velocity, age: self.age };
        self.noise_on = true;
    }
    fn release(&mut self, midi: Option<u8>, key: u8) {
        for v in &mut self.voices {
            if v.key == Some(key) && midi.is_none_or(|m| v.midi == m) && !self.pedals[v.midi as usize] {
                v.off();
            }
        }
        for tone in &mut self.tones {
            if tone.key == Some(key) && midi.is_none_or(|m| tone.midi == m) && !self.pedals[tone.midi as usize] {
                tone.key = None;
            }
        }
        self.noise_on = self.tones.iter().any(|t| t.key.is_some());
    }
    fn render_psg(&mut self) -> (f32, f32) {
        let mut lr = [0.0f32; 2];
        for i in 0..3 {
            let tone = &mut self.tones[i];
            if let Some(key) = tone.key {
                let bend = self.bend[tone.midi as usize] * (1.0 + 12.0 * self.params[51]);
                let hz = midi_frequency(key) * 2.0f32.powf(bend / 12.0);
                tone.phase = (tone.phase + hz / self.rate).fract();
                let signal = if tone.phase < 0.5 { 1.0 } else { -1.0 };
                let gain = self.params[44+i] * tone.velocity * 0.2;
                let pos = (tone.pan - 0.5) * self.params[50] + 0.5;
                lr[0] += signal * gain * (1.0 - pos);
                lr[1] += signal * gain * pos;
            }
        }
        if self.noise_on && self.params[47] > 0.0 {
            let hz = 160.0 * 2.0f32.powf(8.0 * self.params[49]);
            self.noise_phase += hz / self.rate;
            if self.noise_phase >= 1.0 {
                self.noise_phase -= 1.0;
                let bit = if self.params[48] >= 0.5 { self.noise & 1 } else { (self.noise ^ (self.noise >> 1)) & 1 };
                self.noise = (self.noise >> 1) | (bit << 14);
            }
            let value = (if self.noise & 1 != 0 { 1.0 } else { -1.0 }) * self.params[47] * 0.15;
            lr[0] += value; lr[1] += value;
        }
        (lr[0],lr[1])
    }
}
impl Instrument for Gen {
    fn set_parameter(&mut self, i: usize, value: f32) {
        if let Some(p) = self.params.get_mut(i) {
            if value.is_finite() { *p = value.clamp(0.0,1.0); }
        }
    }
    fn note_on(&mut self, key: u8, velocity: f32, pan: f32) { self.start(0,key,velocity,pan); }
    fn note_off(&mut self, key: u8) { self.release(None,key); }
    fn note_on_midi(&mut self, midi: u8, key: u8, velocity: f32, pan: f32) { self.start(midi,key,velocity,pan); }
    fn note_off_midi(&mut self, midi: u8, key: u8) { self.release(Some(midi),key); }
    fn set_midi_controller(&mut self, midi: u8, index: usize, value: f32) {
        if midi >= 16 || !value.is_finite() { return; }
        if index == 4 { self.bend[midi as usize] = value.clamp(0.0,1.0) * 2.0 - 1.0; }
        if index == 71 {
            self.pedals[midi as usize] = value >= 0.5;
            if value < 0.5 {
                for v in &mut self.voices { if v.midi == midi { v.off(); } }
                for t in &mut self.tones { if t.midi == midi { t.key = None; } }
            }
        }
    }
    fn analysis(&self) -> &[f32] { &self.telemetry }
    fn all_notes_off(&mut self) {
        for v in &mut self.voices { v.off(); }
        for t in &mut self.tones { t.key = None; }
        self.noise_on = false;
        self.pedals.fill(false);
    }
    fn reset(&mut self) {
        self.voices.fill(Voice::default());
        self.tones.fill(Tone::default());
        self.noise = 0x4000; self.noise_phase = 0.0;
        self.lfo_phase = 0.0; self.noise_on = false;
        self.telemetry.fill(0.0); self.pedals.fill(false);
        self.smooth_gain = 0.0;
    }
    fn process(&mut self, left: &mut [f32], right: &mut [f32], _: ProcessContext) {
        let lfo_rate = 0.1 + self.params[6] * 19.9;
        let fm_on = self.params[54] < 0.75;
        let psg_on = self.params[54] < 0.25 || self.params[54] >= 0.75;
        for (l,r) in left.iter_mut().zip(right.iter_mut()) {
            self.lfo_phase = (self.lfo_phase + lfo_rate / self.rate).fract();
            let lfo = (self.lfo_phase * TAU).sin();
            let mut fm = [0.0f32; 2];
            for v in &mut self.voices {
                let bend = self.bend[v.midi as usize] * (1.0 + 12.0 * self.params[51]);
                let signal = v.render(&self.params, self.rate, lfo, bend);
                let pos = ((v.pan - 0.5) * self.params[11] + 0.5).clamp(0.0,1.0);
                fm[0] += signal * (1.0 - pos);
                fm[1] += signal * pos;
            }
            let psg = self.render_psg();
            self.smooth_gain += (self.params[3] - self.smooth_gain) * 0.002;
            let trim = 2.0f32.powf((self.params[55] - 0.5) * 4.0);
            let drive = 1.0 + self.params[9] * 12.0;
            let quant = (1 << (16 - (self.params[10] * 7.0).round() as u32)) as f32;
            for (dst, i) in [(l,0usize), (r,1usize)] {
                let mixed = if fm_on { fm[i] * self.params[4] } else { 0.0 }
                    + if psg_on { [psg.0, psg.1][i] * self.params[5] } else { 0.0 };
                let saturated = (mixed * drive).tanh() / drive.tanh();
                let dac = (saturated * quant).round() / quant;
                *dst += (dac * trim * self.smooth_gain * 0.42).clamp(-1.0,1.0);
            }
        }
        for (i,v) in self.voices.iter().enumerate() { self.telemetry[i] = f32::from(v.playing()); }
        for i in 0..3 { self.telemetry[6+i] = f32::from(self.tones[i].key.is_some()); }
        self.telemetry[9] = f32::from(self.noise_on);
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    const CTX: ProcessContext = ProcessContext { sample_rate: 48000.0, tempo_bpm: 120.0, start_beat: 0.0, playing: false };
    #[test] fn fm_plays_without_nan_and_releases() {
        let mut g = Gen::new(48000.0);
        g.note_on(69,1.0,0.5);
        let (mut l,mut r) = (vec![0.0;4096],vec![0.0;4096]);
        g.process(&mut l,&mut r,CTX);
        assert!(l.iter().any(|x| x.abs()>1e-4));
        assert!(l.iter().all(|x| x.is_finite()));
        g.note_off(69);
        assert!(g.voices.iter().all(|v| v.key.is_none()));
    }
    #[test] fn every_fm_algorithm_renders_audio() {
        for i in 0..8 {
            let mut g = Gen::new(48000.0);
            g.set_parameter(1,i as f32 / 7.0);
            g.note_on(60,1.0,0.5);
            let (mut l,mut r) = (vec![0.0;4096],vec![0.0;4096]);
            g.process(&mut l,&mut r,CTX);
            assert!(l.iter().any(|x| x.abs()>0.00001), "algorithm {i}");
        }
    }
    #[test] fn psg_only_produces_square_wave() {
        let mut g = Gen::new(48000.0);
        g.set_parameter(54,1.0);
        g.note_on(60,1.0,0.5);
        let (mut l,mut r) = (vec![0.0;1024],vec![0.0;1024]);
        g.process(&mut l,&mut r,CTX);
        assert!(l.iter().any(|x| x.abs()>0.01));
    }
}
