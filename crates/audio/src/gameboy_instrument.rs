//! MIDI instrument wrapper for a DMG APU: one hardware-like voice (four GB channels).
//! No allocations or locks in the audio callback.
use crate::gameboy_apu::DmgApu;
use crate::instrument::Instrument;
use crate::ProcessContext;

const NUM_PARAMS: usize = 22;
pub const DEFAULTS: [f32; NUM_PARAMS] = [
    1.0, // Enable
    0.87, // pulse 1 level
    2.0 / 3.0, // pulse 1 duty 50%
    0.0, // sweep pace
    0.0, // sweep shift
    0.0, // sweep direction (add)
    0.6, // pulse 2 level
    1.0 / 3.0, // pulse 2 duty 25%
    1.0 / 3.0, // wave level 100%
    0.0, // wave shape triangle
    0.0, // noise level (off by default)
    0.0, // noise 15-bit
    0.0, // noise divisor 0
    0.0, // noise shift 0
    0.0, // envelope pace 0 (sustain)
    1.0, // master volume
    0.0, // stereo: original dual mono
    0.75, // output gain
    1.0, // pulse 1 on
    1.0, // pulse 2 on
    1.0, // wave on
    0.0, // noise off
];
pub struct GameBoyInstrument {
    apu: DmgApu,
    params: [f32; NUM_PARAMS],
    note: Option<u8>,
    velocity: f32,
    note_pan: f32,
    bend_semitones: f32,
}
impl GameBoyInstrument {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            apu: DmgApu::new(sample_rate),
            params: DEFAULTS, note: None,
            velocity: 1.0, note_pan: 0.5, bend_semitones: 0.0,
        }
    }
    fn step(&self, i: usize, max: u8) -> u8 {
        (self.params[i] * max as f32).round() as u8
    }
    fn volume(&self, i: usize) -> u8 {
        self.step(i, 15)
    }
    fn waveform(&mut self) {
        // 32 x 4-bit samples held in the original 16-byte wave RAM.
        let shape = self.step(9, 3);
        self.apu.write(0xff1a, 0);
        for byte in 0..16_usize {
            let pixel = |n: usize| -> u8 {
                match shape {
                    0 => if n < 16 { n as u8 } else { (31 - n) as u8 },
                    1 => (n / 2) as u8,
                    2 => if n < 16 { 15 } else { 0 },
                    _ => {
                        let phase = std::f32::consts::TAU * n as f32 / 32.0;
                        (7.5 * (phase.sin() + 1.0)).round().clamp(0.0, 15.0) as u8
                    }
                }
            };
            self.apu.write(0xff30 + byte as u16,
                (pixel(byte * 2) << 4) | pixel(byte * 2 + 1));
        }
    }
    fn trigger(&mut self, key: u8) {
        let hz = 440.0_f32 * 2.0_f32.powf(
            (key as f32 - 69.0 + self.bend_semitones) / 12.0,
        );
        let pulse = (2048.0 - (131_072.0 / hz)).round().clamp(0.0, 2047.0) as u16;
        let wave = (2048.0 - (65_536.0 / hz)).round().clamp(0.0, 2047.0) as u16;
        self.apu.stop_midi();
        self.apu.write(0xff26, 0x80);
        let master = self.step(15, 7);
        self.apu.write(0xff24, (master << 4) | master);
        let stereo = match self.step(16, 2) {
            0 => 0xff, // channels sent to both sides, like stock NR51
            1 => 0x5a, // hard-left CH1+CH3, hard-right CH2+CH4
            _ => 0xf5, // preserve channel 1 & 3 on both, others on left
        };
        self.apu.write(0xff25, stereo);
        let pace = self.step(14, 7);
        let sweep = (self.step(3, 7) << 4)
            | ((self.params[5] >= 0.5) as u8) << 3 | self.step(4, 7);
        self.apu.write(0xff10, sweep);
        let p1 = if self.params[18] >= 0.5 { self.volume(1) } else { 0 };
        let p2 = if self.params[19] >= 0.5 { self.volume(6) } else { 0 };
        let wave_on = self.params[20] >= 0.5;
        let noise = if self.params[21] >= 0.5 { self.volume(10) } else { 0 };
        let env = |v: u8| (v << 4) | pace;
        self.apu.write(0xff11, self.step(2, 3) << 6);
        self.apu.write(0xff12, env(p1));
        self.apu.write(0xff13, pulse as u8);
        self.apu.write(0xff14, 0x80 | (pulse >> 8) as u8);
        self.apu.write(0xff16, self.step(7, 3) << 6);
        self.apu.write(0xff17, env(p2));
        self.apu.write(0xff18, pulse as u8);
        self.apu.write(0xff19, 0x80 | (pulse >> 8) as u8);
        self.waveform();
        self.apu.write(0xff1a, if wave_on { 0x80 } else { 0 });
        self.apu.write(0xff1b, 0);
        self.apu.write(0xff1c, self.step(8, 3) << 5);
        self.apu.write(0xff1d, wave as u8);
        self.apu.write(0xff1e, 0x80 | (wave >> 8) as u8);
        self.apu.write(0xff20, 0);
        self.apu.write(0xff21, env(noise));
        self.apu.write(0xff22, (self.step(13, 13) << 4)
            | ((self.params[11] >= 0.5) as u8) << 3 | self.step(12, 7));
        self.apu.write(0xff23, 0x80);
    }
}
impl Instrument for GameBoyInstrument {
    fn set_parameter(&mut self, index: usize, value: f32) {
        if let Some(param) = self.params.get_mut(index) {
            if value.is_finite() {
                *param = value.clamp(0.0, 1.0);
                if let Some(key) = self.note { self.trigger(key); }
            }
        }
    }
    fn note_on(&mut self, key: u8, velocity: f32, pan: f32) {
        self.note = Some(key);
        self.velocity = velocity.clamp(0.0, 1.0);
        self.note_pan = pan.clamp(0.0, 1.0);
        self.trigger(key);
    }
    fn note_off(&mut self, key: u8) {
        if self.note == Some(key) { self.note = None; self.apu.stop_midi(); }
    }
    fn all_notes_off(&mut self) { self.note = None; self.apu.stop_midi(); }
    fn reset(&mut self) {
        self.note = None;
        self.apu.stop_midi();
    }
    fn process(&mut self, left: &mut [f32], right: &mut [f32], _: ProcessContext) {
        let volume = self.params[17] * self.velocity;
        let pan = self.note_pan;
        for (l,r) in left.iter_mut().zip(right.iter_mut()) {
            let (a,b) = self.apu.render();
            *l += a * volume * (1.0 - pan).sqrt();
            *r += b * volume * pan.sqrt();
        }
    }
    fn set_controller(&mut self, index: usize, value: f32) {
        if index == 4 && value.is_finite() {
            self.bend_semitones = 4.0 * value.clamp(0.0, 1.0) - 2.0;
            if let Some(key) = self.note { self.trigger(key); }
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn piano_note_renders_sound() {
        let mut g = GameBoyInstrument::new(48_000.0);
        g.note_on(69, 1.0, 0.5);
        let mut l = vec![0.0; 2048]; let mut r = l.clone();
        g.process(&mut l, &mut r, ProcessContext {
            sample_rate: 48_000.0, tempo_bpm: 120.0,
            start_beat: 0.0, playing: true,
        });
        assert!(l.iter().any(|x| x.abs() > 0.00001));
        assert!(l.iter().all(|x| x.is_finite()));
    }
}
