//! Game Boy DMG APU model (experimental): four register-driven channels.
//! Independent of Honeydew's existing oscillators. Clocked at 4_194_304 Hz.
//! References: gbdev Pan Docs Audio Registers / Audio Details.
//! This is not yet cycle-accurate for register edge cases or analog circuitry.
pub const DMG_CLOCK: u32 = 4_194_304;
const FRAME_CLOCK: u32 = 8_192;
const DUTY: [u8; 4] = [0b0000_0001, 0b1000_0001, 0b1000_0111, 0b0111_1110];

#[derive(Clone, Copy)]
struct Envelope {
    initial: u8, volume: u8, pace: u8, timer: u8, rise: bool,
}
impl Default for Envelope {
    fn default() -> Self {
        Self { initial: 0, volume: 0, pace: 0, timer: 0, rise: false }
    }
}
impl Envelope {
    fn write(&mut self, data: u8) {
        self.initial = data >> 4;
        self.rise = data & 8 != 0;
        self.pace = data & 7;
    }
    fn trigger(&mut self) {
        self.volume = self.initial;
        self.timer = if self.pace == 0 { 8 } else { self.pace };
    }
    fn clock(&mut self) {
        if self.pace == 0 { return; }
        self.timer = self.timer.saturating_sub(1);
        if self.timer != 0 { return; }
        self.timer = self.pace;
        if self.rise && self.volume < 15 { self.volume += 1; }
        if !self.rise && self.volume > 0 { self.volume -= 1; }
    }
}
#[derive(Clone, Copy, Default)]
struct Pulse {
    enabled: bool, dac: bool, length_enable: bool,
    duty: u8, pos: u8, length: u16, frequency: u16,
    timer: i32, envelope: Envelope,
    sweep_pace: u8, sweep_shift: u8, sweep_down: bool,
    sweep_timer: u8, sweep_shadow: u16, sweep_enabled: bool,
}
impl Pulse {
    fn period(&self) -> i32 { (2048 - self.frequency as i32).max(1) * 4 }
    fn trigger(&mut self, sweep: bool) {
        self.enabled = self.dac;
        if self.length == 0 { self.length = 64; }
        self.timer = self.period();
        self.pos = 0;
        self.envelope.trigger();
        if sweep {
            self.sweep_shadow = self.frequency;
            self.sweep_timer = if self.sweep_pace == 0 { 8 } else { self.sweep_pace };
            self.sweep_enabled = self.sweep_pace != 0 || self.sweep_shift != 0;
            if self.sweep_shift != 0 && self.calc_sweep().is_none() { self.enabled = false; }
        }
    }
    fn calc_sweep(&self) -> Option<u16> {
        let d = self.sweep_shadow >> self.sweep_shift;
        let next = if self.sweep_down {
            self.sweep_shadow.checked_sub(d)?
        } else {
            self.sweep_shadow.checked_add(d)?
        };
        (next <= 2047).then_some(next)
    }
    fn clock_sweep(&mut self) {
        if !self.sweep_enabled { return; }
        self.sweep_timer = self.sweep_timer.saturating_sub(1);
        if self.sweep_timer != 0 { return; }
        self.sweep_timer = if self.sweep_pace == 0 { 8 } else { self.sweep_pace };
        if self.sweep_pace == 0 { return; }
        if let Some(new_freq) = self.calc_sweep() {
            if self.sweep_shift != 0 {
                self.sweep_shadow = new_freq;
                self.frequency = new_freq;
                if self.calc_sweep().is_none() { self.enabled = false; }
            }
        } else { self.enabled = false; }
    }
    fn clocks(&mut self, n: i32) {
        if !self.enabled { return; }
        self.timer -= n;
        while self.timer <= 0 {
            self.timer += self.period();
            self.pos = (self.pos + 1) & 7;
        }
    }
    fn output(&self) -> Option<u8> {
        if !self.enabled || !self.dac { return None; }
        Some(if (DUTY[self.duty as usize] >> (7 - self.pos)) & 1 != 0 { self.envelope.volume } else { 0 })
    }
    fn clock_length(&mut self) {
        if self.length_enable && self.length > 0 {
            self.length -= 1;
            if self.length == 0 { self.enabled = false; }
        }
    }
}
#[derive(Clone, Copy, Default)]
struct Wave {
    enabled: bool, dac: bool, length_enable: bool,
    length: u16, frequency: u16, timer: i32, index: u8, level: u8,
}
impl Wave {
    fn period(&self) -> i32 { (2048 - self.frequency as i32).max(1) * 2 }
    fn trigger(&mut self) {
        self.enabled = self.dac;
        if self.length == 0 { self.length = 256; }
        self.timer = self.period();
        self.index = 0;
    }
    fn clocks(&mut self, n: i32) {
        if !self.enabled { return; }
        self.timer -= n;
        while self.timer <= 0 {
            self.timer += self.period();
            self.index = (self.index + 1) & 31;
        }
    }
    fn output(&self, ram: &[u8; 16]) -> Option<u8> {
        if !self.enabled || !self.dac { return None; }
        let b = ram[(self.index / 2) as usize];
        let sample = if self.index & 1 == 0 { b >> 4 } else { b & 15 };
        Some(match self.level { 0 => 0, 1 => sample, 2 => sample >> 1, _ => sample >> 2 })
    }
    fn clock_length(&mut self) {
        if self.length_enable && self.length > 0 {
            self.length -= 1;
            if self.length == 0 { self.enabled = false; }
        }
    }
}
#[derive(Clone, Copy)]
struct Noise {
    enabled: bool, dac: bool, length_enable: bool,
    length: u16, timer: i32, divisor: u8, shift: u8, short: bool,
    lfsr: u16, envelope: Envelope,
}
impl Default for Noise {
    fn default() -> Self {
        Self { enabled: false, dac: false, length_enable: false, length: 0,
            timer: 8, divisor: 0, shift: 0, short: false, lfsr: 0x7fff,
            envelope: Envelope::default() }
    }
}
impl Noise {
    fn period(&self) -> i32 {
        let base = if self.divisor == 0 { 8 } else { 16 * self.divisor as i32 };
        base << self.shift.min(13)
    }
    fn trigger(&mut self) {
        self.enabled = self.dac;
        if self.length == 0 { self.length = 64; }
        self.envelope.trigger();
        self.lfsr = 0x7fff;
        self.timer = self.period();
    }
    fn clocks(&mut self, n: i32) {
        if !self.enabled { return; }
        self.timer -= n;
        while self.timer <= 0 {
            self.timer += self.period();
            let xor = (self.lfsr ^ (self.lfsr >> 1)) & 1;
            self.lfsr = (self.lfsr >> 1) | (xor << 14);
            if self.short { self.lfsr = (self.lfsr & !(1 << 6)) | (xor << 6); }
        }
    }
    fn output(&self) -> Option<u8> {
        if !self.enabled || !self.dac { return None; }
        Some(if self.lfsr & 1 == 0 { self.envelope.volume } else { 0 })
    }
    fn clock_length(&mut self) {
        if self.length_enable && self.length > 0 {
            self.length -= 1;
            if self.length == 0 { self.enabled = false; }
        }
    }
}
pub struct DmgApu {
    regs: [u8; 0x30],
    wave_ram: [u8; 16],
    first: Pulse, second: Pulse, wave: Wave, noise: Noise,
    powered: bool, frame_clock: u32, frame_step: u8,
    clock_frac: f64, rate: f64,
    hp_prev: [f32; 2], hp_last: [f32; 2], hp_coeff: f32,
}
impl DmgApu {
    pub fn new(rate: f32) -> Self {
        let rate = if rate.is_finite() && rate > 1.0 { rate as f64 } else { 48_000.0 };
        let mut a = Self { regs: [0; 0x30], wave_ram: [0; 16],
            first: Pulse::default(), second: Pulse::default(), wave: Wave::default(),
            noise: Noise::default(), powered: true, frame_clock: 0, frame_step: 0,
            clock_frac: 0.0, rate, hp_prev: [0.0; 2], hp_last: [0.0; 2],
            hp_coeff: 0.999958_f64.powf(DMG_CLOCK as f64 / rate) as f32 };
        a.write(0xff24, 0x77); a.write(0xff25, 0xff); a.write(0xff26, 0x80);
        a
    }
    pub fn wave_ram_mut(&mut self) -> &mut [u8; 16] { &mut self.wave_ram }
    pub fn register(&self, addr: u16) -> u8 {
        if !(0xff10..=0xff3f).contains(&addr) { return 0xff; }
        if addr >= 0xff30 { return self.wave_ram[(addr - 0xff30) as usize]; }
        if addr == 0xff26 {
            let flags = (self.first.enabled as u8) | ((self.second.enabled as u8) << 1)
                | ((self.wave.enabled as u8) << 2) | ((self.noise.enabled as u8) << 3);
            return 0x70 | if self.powered { 0x80 } else { 0 } | flags;
        }
        self.regs[(addr - 0xff10) as usize]
    }
    pub fn write(&mut self, addr: u16, data: u8) {
        if !(0xff10..=0xff3f).contains(&addr) { return; }
        if addr >= 0xff30 {
            self.wave_ram[(addr - 0xff30) as usize] = data;
            return;
        }
        if addr == 0xff26 {
            if data & 0x80 == 0 {
                self.powered = false;
                self.first = Pulse::default(); self.second = Pulse::default();
                self.wave = Wave::default(); self.noise = Noise::default();
                self.regs.fill(0);
            } else { self.powered = true; }
            return;
        }
        if !self.powered { return; }
        let at = (addr - 0xff10) as usize;
        self.regs[at] = data;
        match addr {
            0xff10 => {
                self.first.sweep_pace = (data >> 4) & 7;
                self.first.sweep_down = data & 8 != 0;
                self.first.sweep_shift = data & 7;
            }
            0xff11 => { self.first.duty = data >> 6; self.first.length = 64 - (data & 63) as u16; }
            0xff12 => { self.first.envelope.write(data); self.first.dac = data & 0xf8 != 0; if !self.first.dac { self.first.enabled = false; } }
            0xff13 => self.first.frequency = (self.first.frequency & 0x700) | data as u16,
            0xff14 => {
                self.first.frequency = (self.first.frequency & 0xff) | (((data & 7) as u16) << 8);
                self.first.length_enable = data & 0x40 != 0;
                if data & 0x80 != 0 { self.first.trigger(true); }
            }
            0xff16 => { self.second.duty = data >> 6; self.second.length = 64 - (data & 63) as u16; }
            0xff17 => { self.second.envelope.write(data); self.second.dac = data & 0xf8 != 0; if !self.second.dac { self.second.enabled = false; } }
            0xff18 => self.second.frequency = (self.second.frequency & 0x700) | data as u16,
            0xff19 => {
                self.second.frequency = (self.second.frequency & 0xff) | (((data & 7) as u16) << 8);
                self.second.length_enable = data & 0x40 != 0;
                if data & 0x80 != 0 { self.second.trigger(false); }
            }
            0xff1a => { self.wave.dac = data & 0x80 != 0; if !self.wave.dac { self.wave.enabled = false; } }
            0xff1b => self.wave.length = 256 - data as u16,
            0xff1c => self.wave.level = (data >> 5) & 3,
            0xff1d => self.wave.frequency = (self.wave.frequency & 0x700) | data as u16,
            0xff1e => {
                self.wave.frequency = (self.wave.frequency & 0xff) | (((data & 7) as u16) << 8);
                self.wave.length_enable = data & 0x40 != 0;
                if data & 0x80 != 0 { self.wave.trigger(); }
            }
            0xff20 => self.noise.length = 64 - (data & 63) as u16,
            0xff21 => { self.noise.envelope.write(data); self.noise.dac = data & 0xf8 != 0; if !self.noise.dac { self.noise.enabled = false; } }
            0xff22 => { self.noise.shift = data >> 4; self.noise.short = data & 8 != 0; self.noise.divisor = data & 7; }
            0xff23 => {
                self.noise.length_enable = data & 0x40 != 0;
                if data & 0x80 != 0 { self.noise.trigger(); }
            }
            _ => {}
        }
    }
    fn clock_frame(&mut self) {
        if self.frame_step & 1 == 0 {
            self.first.clock_length(); self.second.clock_length();
            self.wave.clock_length(); self.noise.clock_length();
        }
        if self.frame_step == 2 || self.frame_step == 6 { self.first.clock_sweep(); }
        if self.frame_step == 7 {
            self.first.envelope.clock(); self.second.envelope.clock(); self.noise.envelope.clock();
        }
        self.frame_step = (self.frame_step + 1) & 7;
    }
    pub fn step_cycles(&mut self, mut cycles: u32) {
        if !self.powered { return; }
        while cycles != 0 {
            let until = FRAME_CLOCK - self.frame_clock;
            let n = cycles.min(until);
            self.first.clocks(n as i32); self.second.clocks(n as i32);
            self.wave.clocks(n as i32); self.noise.clocks(n as i32);
            self.frame_clock += n; cycles -= n;
            if self.frame_clock == FRAME_CLOCK {
                self.frame_clock = 0;
                self.clock_frame();
            }
        }
    }
    pub fn render(&mut self) -> (f32, f32) {
        self.clock_frac += DMG_CLOCK as f64 / self.rate;
        let cycles = self.clock_frac.floor() as u32;
        self.clock_frac -= cycles as f64;
        self.step_cycles(cycles);
        let signals = [self.first.output(), self.second.output(),
            self.wave.output(&self.wave_ram), self.noise.output()];
        let nr51 = self.regs[0x15]; // NR51 FF25
        let nr50 = self.regs[0x14]; // NR50 FF24
        let volumes = [((nr50 >> 4) & 7) + 1, (nr50 & 7) + 1];
        let mut output = [0.0_f32; 2];
        for side in 0..2 {
            let mut mixed = 0.0;
            for (channel, signal) in signals.iter().enumerate() {
                if nr51 & (1 << (channel + if side == 0 { 4 } else { 0 })) != 0 {
                    if let Some(digital) = signal {
                        mixed += (*digital as f32 / 7.5 - 1.0) / 4.0;
                    }
                }
            }
            let input = mixed * volumes[side] as f32 / 8.0;
            let filtered = input - self.hp_prev[side] + self.hp_coeff * self.hp_last[side];
            self.hp_prev[side] = input; self.hp_last[side] = filtered;
            output[side] = filtered.clamp(-1.0, 1.0);
        }
        (output[0], output[1])
    }
    pub fn stop_midi(&mut self) {
        // MIDI note-off is not an original DMG register event; silence the active generators.
        self.first.enabled = false; self.second.enabled = false;
        self.wave.enabled = false; self.noise.enabled = false;
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn clock_rate_and_frame_tick() {
        let mut a = DmgApu::new(48_000.0);
        a.step_cycles(DMG_CLOCK / 512);
        assert_eq!(a.frame_step, 1);
        a.step_cycles(DMG_CLOCK / 512 * 7);
        assert_eq!(a.frame_step, 0);
    }
    #[test]
    fn wave_ram_survives_power_off() {
        let mut a = DmgApu::new(44_100.0);
        a.write(0xff30, 0xab);
        a.write(0xff26, 0);
        assert_eq!(a.register(0xff30), 0xab);
        assert_eq!(a.register(0xff26) & 0x80, 0);
    }
    #[test]
    fn channel_1_trigger_and_dac_disable() {
        let mut a = DmgApu::new(48_000.0);
        a.write(0xff12, 0xf3);
        a.write(0xff11, 0x80);
        a.write(0xff13, 0);
        a.write(0xff14, 0x87);
        assert_eq!(a.register(0xff26) & 1, 1);
        a.write(0xff12, 0);
        assert_eq!(a.register(0xff26) & 1, 0);
    }
    #[test]
    fn volume_envelope_ticks_on_frame_step_seven() {
        let mut a = DmgApu::new(48_000.0);
        a.write(0xff12, 0x91);
        a.write(0xff14, 0x80);
        assert_eq!(a.first.envelope.volume, 9);
        a.step_cycles(FRAME_CLOCK * 8);
        assert_eq!(a.first.envelope.volume, 8);
    }
    #[test]
    fn noise_lfsr_widths_differ() {
        let mut a = Noise::default();
        a.dac = true; a.divisor = 0; a.shift = 0; a.short = false; a.trigger();
        a.clocks(8 * 16); let long = a.lfsr;
        a.short = true; a.trigger(); a.clocks(8 * 16);
        assert_ne!(long, a.lfsr);
    }
    #[test]
    fn audio_finite_across_common_rates() {
        for rate in [8_000.0, 32_768.0, 44_100.0, 48_000.0, 96_000.0] {
            let mut a = DmgApu::new(rate);
            a.write(0xff12, 0xf0); a.write(0xff11, 0x80);
            a.write(0xff13, 0x00); a.write(0xff14, 0x87);
            let samples = (0..4096).map(|_| a.render()).collect::<Vec<_>>();
            assert!(samples.iter().all(|(l,r)| l.is_finite() && r.is_finite()));
            assert!(samples.iter().any(|(l,_)| l.abs() > 0.001));
        }
    }
}
