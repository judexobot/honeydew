//! One four-channel DMG in Authentic, eight independent APUs in Extended.
//! Parameter indices 0..21 retain the original fork's normalization and meaning.
use crate::{ProcessContext, gameboy_apu::DmgApu, instrument::Instrument};
pub const NUM_PARAMS: usize = 87;
pub const MODE: usize = 22;
pub const ROUTING: usize = 23;
pub const DESTINATION: usize = 24;
pub const CUSTOM_WAVE: usize = 26;
pub const WAVE_START: usize = 27;
pub const DEFAULTS: [f32; NUM_PARAMS] = defaults();
const fn defaults() -> [f32; NUM_PARAMS] {
    let mut p = [0.0; NUM_PARAMS];
    p[0] = 1.0;
    p[1] = 0.87;
    p[2] = 2.0 / 3.0;
    p[6] = 0.6;
    p[7] = 1.0 / 3.0;
    p[8] = 1.0 / 3.0;
    p[15] = 1.0;
    p[17] = 0.75;
    p[18] = 1.0;
    p[19] = 1.0;
    p[20] = 1.0;
    p[23] = 1.0;
    p[25] = 2.0 / 24.0;
    let mut i = 0;
    while i < 32 {
        p[WAVE_START + i] = if i < 16 {
            i as f32 / 15.0
        } else {
            (31 - i) as f32 / 15.0
        };
        i += 1;
    }
    p[63] = 0.5;
    p[68] = 0.5;
    p[71] = 0.5;
    p[78] = 1.0;
    p[79] = 1.0;
    p[80] = 1.0;
    p[81] = 1.0;
    p[82] = 0.02;
    p[83] = 0.5;
    p[84] = 0.5;
    p[85] = 0.5;
    p[86] = 0.5;
    p
}
#[derive(Clone, Copy, Default)]
struct Held {
    age: u64,
    velocity: f32,
    midi: u8,
}
struct Voice {
    apu: DmgApu,
    key: Option<u8>,
    midi: u8,
    age: u64,
    velocity: f32,
    pan: f32,
    gain: f32,
    release: bool,
    pending: bool,
}
impl Voice {
    fn new(rate: f32) -> Self {
        Self {
            apu: DmgApu::new(rate),
            key: None,
            midi: 0,
            age: 0,
            velocity: 1.0,
            pan: 0.5,
            gain: 0.0,
            release: false,
            pending: false,
        }
    }
}
pub struct GameBoyInstrument {
    voices: [Voice; 8],
    params: [f32; NUM_PARAMS],
    dirty: [bool; NUM_PARAMS],
    held: [[Held; 128]; 4],
    deferred: [[bool; 128]; 4],
    pedals: [bool; 16],
    active: [Option<u8>; 4],
    bend: [f32; 16],
    age: u64,
    rate: f32,
    telemetry: [f32; 8],
    output_gain: f32,
}
impl GameBoyInstrument {
    pub fn new(rate: f32) -> Self {
        let rate = if rate.is_finite() && (8000.0..=192000.0).contains(&rate) {
            rate
        } else {
            48000.0
        };
        Self {
            voices: std::array::from_fn(|_| Voice::new(rate)),
            params: DEFAULTS,
            dirty: [true; NUM_PARAMS],
            held: [[Held::default(); 128]; 4],
            deferred: [[false; 128]; 4],
            pedals: [false; 16],
            active: [None; 4],
            bend: [0.0; 16],
            age: 0,
            rate: rate.clamp(8000.0, 192000.0),
            telemetry: [0.0; 8],
            output_gain: 0.75,
        }
    }
    fn step(&self, i: usize, max: u8) -> u8 {
        (self.params[i] * max as f32).round() as u8
    }
    fn extended(&self) -> bool {
        self.params[MODE] >= 0.5
    }
    fn targets(&self, midi: Option<u8>) -> u8 {
        match self.step(ROUTING, 2) {
            0 => 15,
            1 => 1 << self.step(DESTINATION, 3),
            _ => match midi {
                Some(ch) if ch < 4 => 1 << ch,
                Some(_) => 0,
                None => 1 << self.step(DESTINATION, 3),
            },
        }
    }
    fn wave(&self) -> [u8; 16] {
        let shape = self.step(9, 3);
        let pixel = |n: usize| -> u8 {
            if self.params[CUSTOM_WAVE] >= 0.5 {
                self.step(WAVE_START + n, 15)
            } else {
                wave_sample(shape, n)
            }
        };
        std::array::from_fn(|i| (pixel(i * 2) << 4) | pixel(i * 2 + 1))
    }
    fn write_wave(&mut self, voice: usize) {
        let wave = self.wave();
        let apu = &mut self.voices[voice].apu;
        let on = apu.channel_status()[2];
        apu.write(0xff1a, 0);
        for (i, value) in wave.into_iter().enumerate() {
            apu.write(0xff30 + i as u16, value);
        }
        if self.params[20] >= 0.5 {
            apu.write(0xff1a, 0x80);
            if on {
                let key = self.voices[voice].key.or(self.active[2]).unwrap_or(69);
                self.frequency(voice, 2, key, true);
            }
        }
    }
    fn mix(&mut self, voice: usize) {
        let pans = if self.extended() {
            Some([
                self.params[83],
                self.params[84],
                self.params[85],
                self.params[86],
            ])
        } else {
            None
        };
        self.voices[voice].apu.set_extended_pan(pans);
        let master = self.step(15, 7);
        let mut routing = match self.step(16, 2) {
            0 => 0xff,
            1 => 0x5a,
            _ => 0xf5,
        };
        if self.params[77] >= 0.5 {
            routing = 0;
            for ch in 0..4 {
                let route = self.step(78 + ch, 3);
                if route & 1 != 0 {
                    routing |= 1 << (ch + 4);
                }
                if route & 2 != 0 {
                    routing |= 1 << ch;
                }
            }
        }
        let apu = &mut self.voices[voice].apu;
        apu.write(0xff24, (master << 4) | master);
        apu.write(0xff25, routing);
    }
    fn envelope(&self, ch: usize, velocity: f32) -> u8 {
        let (level, direction, pace) = match ch {
            0 => (1, 59, 60),
            1 => (6, 64, 65),
            _ => (10, 72, 73),
        };
        let initial = if self.params[18 + ch] >= 0.5 {
            (self.params[level] * velocity * 15.0).round() as u8
        } else {
            0
        };
        let override_pace = self.step(pace, 8);
        let pace = if override_pace == 0 {
            self.step(14, 7)
        } else {
            override_pace - 1
        };
        (initial << 4) | (u8::from(self.params[direction] >= 0.5) << 3) | pace
    }
    fn length(&self, ch: usize) -> (u8, bool) {
        let (on, value, max) = match ch {
            0 => (61, 62, 63),
            1 => (66, 67, 63),
            2 => (69, 70, 255),
            _ => (74, 75, 63),
        };
        (self.step(value, max), self.params[on] >= 0.5)
    }
    fn frequency(&mut self, voice: usize, ch: usize, key: u8, trigger: bool) {
        let transpose = match ch {
            0 => 48.0 * self.params[63] - 24.0,
            1 => 48.0 * self.params[68] - 24.0,
            2 => 48.0 * self.params[71] - 24.0,
            _ => 0.0,
        };
        let midi = if self.extended() {
            self.voices[voice].midi
        } else {
            self.held[ch][key as usize].midi
        };
        let bend = self.bend[midi.min(15) as usize] * self.params[25] * 24.0;
        let hz = 440.0 * 2.0_f32.powf((key as f32 - 69.0 + transpose + bend) / 12.0);
        let period = (2048.0 - if ch == 2 { 65536.0 / hz } else { 131072.0 / hz })
            .round()
            .clamp(0.0, 2047.0) as u16;
        let (_, length) = self.length(ch);
        let high = match ch {
            0 => 0xff14,
            1 => 0xff19,
            2 => 0xff1e,
            _ => 0xff23,
        };
        let apu = &mut self.voices[voice].apu;
        if ch < 3 {
            apu.write(high - 1, period as u8);
        }
        apu.write(
            high,
            (if trigger { 0x80 } else { 0 })
                | (u8::from(length) << 6)
                | if ch < 3 { (period >> 8) as u8 } else { 0 },
        );
    }
    fn configure(&mut self, voice: usize, ch: usize, velocity: f32) {
        let (length, _) = self.length(ch);
        match ch {
            0 => {
                let sweep = (self.step(3, 7) << 4)
                    | (u8::from(self.params[5] >= 0.5) << 3)
                    | self.step(4, 7);
                let duty = self.step(2, 3);
                let env = self.envelope(0, velocity);
                let a = &mut self.voices[voice].apu;
                a.write(0xff10, sweep);
                a.write(0xff11, (duty << 6) | length);
                a.write(0xff12, env);
            }
            1 => {
                let duty = self.step(7, 3);
                let env = self.envelope(1, velocity);
                let a = &mut self.voices[voice].apu;
                a.write(0xff16, (duty << 6) | length);
                a.write(0xff17, env);
            }
            2 => {
                let level = self.step(8, 3);
                let a = &mut self.voices[voice].apu;
                a.write(0xff1a, if self.params[20] >= 0.5 { 0x80 } else { 0 });
                a.write(0xff1b, length);
                a.write(0xff1c, level << 5);
            }
            _ => {
                let env = self.envelope(3, velocity);
                let shift = match self.step(76, 2) {
                    0 => self.step(13, 13),
                    1 => 14,
                    _ => 15,
                };
                let control =
                    (shift << 4) | (u8::from(self.params[11] >= 0.5) << 3) | self.step(12, 7);
                let a = &mut self.voices[voice].apu;
                a.write(0xff20, length);
                a.write(0xff21, env);
                a.write(0xff22, control);
            }
        }
    }
    fn trigger(&mut self, voice: usize, ch: usize, key: u8, velocity: f32) {
        self.configure(voice, ch, velocity);
        self.frequency(voice, ch, key, true);
    }
    fn synchronize(&mut self) {
        if !self.dirty.iter().any(|v| *v) {
            return;
        }
        let wave = self.dirty[9]
            || self.dirty[CUSTOM_WAVE]
            || self.dirty[WAVE_START..WAVE_START + 32].iter().any(|v| *v);
        let voices = if self.extended() { 8 } else { 1 };
        for voice in 0..voices {
            self.mix(voice);
            if wave {
                self.write_wave(voice);
            }
            for ch in 0..4 {
                let active = if self.extended() {
                    self.voices[voice].key
                } else {
                    self.active[ch]
                };
                let Some(key) = active else {
                    continue;
                };
                let velocity = if self.extended() {
                    self.voices[voice].velocity
                } else {
                    self.held[ch][key as usize].velocity
                };
                let changed = match ch {
                    0 => {
                        self.dirty[1..=5].iter().any(|v| *v)
                            || self.dirty[59..=63].iter().any(|v| *v)
                    }
                    1 => self.dirty[6] || self.dirty[7] || self.dirty[64..=68].iter().any(|v| *v),
                    2 => self.dirty[8] || self.dirty[69..=71].iter().any(|v| *v),
                    _ => {
                        self.dirty[10..=13].iter().any(|v| *v)
                            || self.dirty[72..=76].iter().any(|v| *v)
                    }
                } || self.dirty[14]
                    || self.dirty[18 + ch];
                if changed {
                    self.configure(voice, ch, velocity);
                    self.frequency(voice, ch, key, false);
                    if self.dirty[18 + ch] {
                        if self.params[18 + ch] >= 0.5 {
                            self.frequency(voice, ch, key, true);
                        } else {
                            self.voices[voice].apu.stop_channel(ch);
                        }
                    }
                    // Initial volume is a hardware envelope setting: apply it with a trigger on this channel alone.
                    if self.dirty[match ch {
                        0 => 1,
                        1 => 6,
                        3 => 10,
                        _ => 8,
                    }] && ch != 2
                    {
                        self.frequency(voice, ch, key, true);
                    }
                }
            }
        }
        self.dirty.fill(false);
    }
    fn on(&mut self, midi: Option<u8>, key: u8, velocity: f32, pan: f32) {
        if key > 127 || !velocity.is_finite() || !pan.is_finite() {
            return;
        }
        if velocity <= 0.0 {
            self.off(midi, key);
            return;
        }
        self.synchronize();
        self.age = self.age.wrapping_add(1).max(1);
        let targets = self.targets(midi);
        if self.extended() {
            let voice = self
                .voices
                .iter()
                .position(|v| v.key.is_none())
                .unwrap_or_else(|| {
                    self.voices
                        .iter()
                        .enumerate()
                        .min_by_key(|(_, v)| v.age)
                        .map_or(0, |(i, _)| i)
                });
            let source = midi.unwrap_or(self.step(DESTINATION, 3));
            let v = &mut self.voices[voice];
            v.apu.reset();
            v.key = Some(key);
            v.midi = source;
            v.age = self.age;
            v.velocity = velocity.clamp(0.0, 1.0);
            v.pan = pan.clamp(0.0, 1.0);
            v.gain = 0.0;
            v.release = false;
            v.pending = false;
            self.mix(voice);
            self.write_wave(voice);
            for ch in 0..4 {
                if targets & (1 << ch) != 0 && self.params[18 + ch] >= 0.5 {
                    self.trigger(voice, ch, key, velocity.clamp(0.0, 1.0));
                }
            }
        } else {
            for ch in 0..4 {
                if targets & (1 << ch) != 0 {
                    self.held[ch][key as usize] = Held {
                        age: self.age,
                        velocity: velocity.clamp(0.0, 1.0),
                        midi: midi.unwrap_or(ch as u8),
                    };
                    self.deferred[ch][key as usize] = false;
                    self.active[ch] = Some(key);
                    if self.params[18 + ch] >= 0.5 {
                        self.trigger(0, ch, key, velocity.clamp(0.0, 1.0));
                    }
                }
            }
        }
    }
    fn off(&mut self, midi: Option<u8>, key: u8) {
        if key > 127 {
            return;
        }
        if self.extended() {
            for v in &mut self.voices {
                if v.key == Some(key) && midi.is_none_or(|m| v.midi == m) {
                    if self.pedals[v.midi as usize] {
                        v.pending = true;
                    } else {
                        v.release = true;
                    }
                }
            }
            return;
        }
        for ch in 0..4 {
            let held = &mut self.held[ch][key as usize];
            if midi.is_some_and(|m| held.midi != m) {
                continue;
            }
            if held.age != 0 && self.pedals[held.midi as usize] {
                self.deferred[ch][key as usize] = true;
                continue;
            }
            held.age = 0;
            self.deferred[ch][key as usize] = false;
            if self.active[ch] == Some(key) {
                let next = self.held[ch]
                    .iter()
                    .enumerate()
                    .filter(|(_, h)| h.age != 0)
                    .max_by_key(|(_, h)| h.age)
                    .map(|(k, h)| (k as u8, h.velocity));
                self.active[ch] = next.map(|(k, _)| k);
                if let Some((key, velocity)) = next {
                    self.trigger(0, ch, key, velocity);
                } else {
                    self.voices[0].apu.stop_channel(ch);
                }
            }
        }
    }
    fn bend(&mut self, midi: u8, value: f32) {
        if midi >= 16 || !value.is_finite() {
            return;
        }
        self.bend[midi as usize] = 2.0 * value.clamp(0.0, 1.0) - 1.0;
        if self.extended() {
            for voice in 0..8 {
                if self.voices[voice].midi == midi
                    && let Some(key) = self.voices[voice].key
                {
                    for ch in 0..4 {
                        self.frequency(voice, ch, key, false);
                    }
                }
            }
        } else {
            for ch in 0..4 {
                if let Some(key) = self.active[ch]
                    && self.held[ch][key as usize].midi == midi
                {
                    self.frequency(0, ch, key, false);
                }
            }
        }
    }
}
impl Instrument for GameBoyInstrument {
    fn set_parameter(&mut self, index: usize, value: f32) {
        if index >= NUM_PARAMS || !value.is_finite() {
            return;
        }
        let value = value.clamp(0.0, 1.0);
        if self.params[index] == value {
            return;
        }
        if index == MODE || index == ROUTING {
            self.reset();
        }
        self.params[index] = value;
        self.dirty[index] = true;
    }
    fn note_on(&mut self, key: u8, velocity: f32, pan: f32) {
        self.on(None, key, velocity, pan);
    }
    fn note_off(&mut self, key: u8) {
        self.off(None, key);
    }
    fn note_on_midi(&mut self, midi: u8, key: u8, velocity: f32, pan: f32) {
        self.on(Some(midi), key, velocity, pan);
    }
    fn note_off_midi(&mut self, midi: u8, key: u8) {
        self.off(Some(midi), key);
    }
    fn set_midi_controller(&mut self, midi: u8, index: usize, value: f32) {
        if midi >= 16 || !value.is_finite() {
            return;
        }
        if index == 4 {
            self.bend(midi, value);
        }
        if index == 71 {
            self.pedals[midi as usize] = value >= 0.5;
            if value < 0.5 {
                for ch in 0..4 {
                    for key in 0..128 {
                        if self.deferred[ch][key] && self.held[ch][key].midi == midi {
                            self.off(Some(midi), key as u8);
                        }
                    }
                }
                for v in &mut self.voices {
                    if v.midi == midi && v.pending {
                        v.release = true;
                        v.pending = false;
                    }
                }
            }
        }
        if index == 127 || index == 130 {
            // CC120 is immediate; CC123 releases keys while retaining the sustain pedal.
            if index == 127 {
                self.pedals[midi as usize] = false;
            }
            for key in 0..128 {
                self.off(Some(midi), key as u8);
            }
            if index == 127 && self.extended() {
                // All Sound Off bypasses the musical release, including Extended voice tails.
                for voice in &mut self.voices {
                    if voice.midi == midi {
                        voice.apu.stop_midi();
                        voice.key = None;
                        voice.gain = 0.0;
                        voice.pending = false;
                    }
                }
            }
        }
    }

    fn all_notes_off(&mut self) {
        self.held.fill([Held::default(); 128]);
        self.deferred.fill([false; 128]);
        self.active.fill(None);
        for v in &mut self.voices {
            if self.params[MODE] >= 0.5 {
                v.release = true;
            } else {
                v.apu.stop_midi();
                v.key = None;
            }
        }
    }
    fn reset(&mut self) {
        self.held.fill([Held::default(); 128]);
        self.deferred.fill([false; 128]);
        self.active.fill(None);
        for v in &mut self.voices {
            v.apu.reset();
            v.key = None;
            v.gain = 0.0;
            v.release = false;
            v.pending = false;
        }
        self.telemetry.fill(0.0);
        self.dirty.fill(true);
    }
    fn set_controller(&mut self, index: usize, value: f32) {
        if index == 4 {
            for m in 0..16 {
                self.bend(m, value);
            }
        }
    }
    fn analysis(&self) -> &[f32] {
        &self.telemetry
    }
    fn process(&mut self, left: &mut [f32], right: &mut [f32], _: ProcessContext) {
        self.synchronize();
        let extended = self.extended();
        let release = 1.0 / (self.rate * (0.002 + self.params[82] * 0.5));
        for (l, r) in left.iter_mut().zip(right.iter_mut()) {
            self.output_gain += (self.params[17] - self.output_gain) * 0.002;
            for voice in 0..if extended { 8 } else { 1 } {
                let v = &mut self.voices[voice];
                if extended && v.key.is_none() {
                    continue;
                }
                let (a, b) = v.apu.render();
                let gain = if extended {
                    if v.release {
                        v.gain = (v.gain - release).max(0.0);
                    } else {
                        v.gain = (v.gain + 1.0 / (self.rate * 0.002)).min(1.0);
                    }
                    v.gain * 0.35
                } else {
                    1.0
                };
                let (pan_l, pan_r) = if extended {
                    ((2.0 * (1.0 - v.pan)).min(1.0), (2.0 * v.pan).min(1.0))
                } else {
                    (1.0, 1.0)
                };
                *l += a * gain * self.output_gain * pan_l;
                *r += b * gain * self.output_gain * pan_r;
                if extended && v.release && v.gain == 0.0 {
                    v.key = None;
                    v.apu.stop_midi();
                }
            }
        }
        self.telemetry.fill(0.0);
        for v in &self.voices[..if extended { 8 } else { 1 }] {
            for ch in 0..4 {
                self.telemetry[ch] = self.telemetry[ch].max(f32::from(v.apu.channel_status()[ch]));
                self.telemetry[ch + 4] =
                    self.telemetry[ch + 4].max(v.apu.channel_levels()[ch] as f32 / 15.0);
            }
        }
    }
}
/// Four factory tables shared by the editor and register loader.
pub fn wave_sample(shape: u8, n: usize) -> u8 {
    let n = n % 32;
    match shape {
        0 => {
            if n < 16 {
                n as u8
            } else {
                (31 - n) as u8
            }
        }
        1 => (n / 2) as u8,
        2 => {
            if n < 16 {
                15
            } else {
                0
            }
        }
        _ => (7.5 * (1.0 + (std::f32::consts::TAU * n as f32 / 32.0).sin())).round() as u8,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn render(g: &mut GameBoyInstrument) -> Vec<f32> {
        let mut l = vec![0.0; 4096];
        let mut r = l.clone();
        g.process(
            &mut l,
            &mut r,
            ProcessContext {
                sample_rate: 48000.0,
                tempo_bpm: 120.0,
                start_beat: 0.0,
                playing: true,
            },
        );
        l
    }
    #[test]
    fn piano_note_renders_sound() {
        let mut g = GameBoyInstrument::new(48000.0);
        g.note_on(69, 1.0, 0.5);
        let l = render(&mut g);
        assert!(l.iter().all(|v| v.is_finite()));
        assert!(l.iter().any(|v| v.abs() > 1e-5));
    }
    #[test]
    fn authentic_channels_are_independent() {
        let mut g = GameBoyInstrument::new(48000.0);
        g.note_on_midi(0, 60, 1.0, 0.5);
        g.note_on_midi(1, 60, 1.0, 0.5);
        assert_eq!(g.voices[0].apu.register(0xff26) & 3, 3);
        g.note_off_midi(0, 60);
        assert_eq!(g.voices[0].apu.register(0xff26) & 3, 2);
    }
    #[test]
    fn gain_and_bend_do_not_restart_envelopes() {
        let mut g = GameBoyInstrument::new(48000.0);
        g.set_parameter(60, 2.0 / 8.0);
        g.note_on_midi(0, 69, 1.0, 0.5);
        render(&mut g);
        let before = g.voices[0].apu.channel_levels();
        g.set_parameter(17, 0.5);
        g.set_midi_controller(0, 4, 0.8);
        g.synchronize();
        assert_eq!(g.voices[0].apu.channel_levels(), before);
    }
    #[test]
    fn extended_has_eight_independent_apus() {
        let mut g = GameBoyInstrument::new(48000.0);
        g.set_parameter(MODE, 1.0);
        for key in 60..70 {
            g.note_on(key, 1.0, 0.5);
        }
        assert_eq!(g.voices.iter().filter(|v| v.key.is_some()).count(), 8);
        assert!(render(&mut g).iter().any(|v| v.abs() > 1e-5));
    }
}

#[cfg(test)]
mod midi_tests {
    use super::*;
    #[test]
    fn sustain_and_panic_respect_midi_channel_identity() {
        let mut apu = GameBoyInstrument::new(48_000.0);
        apu.note_on_midi(0, 60, 0.8, 0.5);
        apu.note_on_midi(1, 60, 0.8, 0.5);
        apu.set_midi_controller(0, 71, 1.0);
        apu.note_off_midi(0, 60);
        apu.note_off_midi(1, 60);
        assert_eq!(&apu.voices[0].apu.channel_status()[..2], &[true, false]);
        apu.set_midi_controller(0, 71, 0.0);
        assert_eq!(&apu.voices[0].apu.channel_status()[..2], &[false, false]);
        apu.set_parameter(MODE, 1.0);
        apu.note_on_midi(0, 60, 0.8, 0.5);
        apu.note_on_midi(1, 60, 0.8, 0.5);
        apu.set_midi_controller(0, 127, 0.0);
        assert!(!apu.voices.iter().any(|v| v.midi == 0 && v.key.is_some()));
        assert!(apu.voices.iter().any(|v| v.midi == 1 && v.key == Some(60)));
    }
}

#[cfg(test)]
mod midi_release_tests {
    use super::*;
    #[test]
    fn all_notes_off_retains_sustain_but_all_sound_off_clears_it() {
        for mode in [0.0, 1.0] {
            let mut instrument = GameBoyInstrument::new(48000.0);
            instrument.set_parameter(MODE, mode);
            instrument.note_on_midi(1, 60, 1.0, 0.5);
            instrument.set_midi_controller(1, 71, 1.0);
            instrument.set_midi_controller(1, 130, 0.0);
            assert!(instrument.pedals[1]);
            if mode >= 0.5 {
                assert!(instrument.voices.iter().any(|v| v.pending));
            } else {
                assert!(instrument.deferred[1][60]);
            }
            instrument.set_midi_controller(1, 127, 0.0);
            assert!(!instrument.pedals[1]);
            assert!(!instrument.deferred[1][60]);
        }
    }
}
