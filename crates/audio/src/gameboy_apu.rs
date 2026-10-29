//! DMG register/timer APU. Digital timing and DAC routing follow Pan Docs.
//! Band-limited delta resampling; analog DAC fades and model-specific zombie
//! envelope behavior remain approximations. This is not a complete GB emulator.
use blip_buf::BlipBuf;
pub const DMG_CLOCK: u32 = 4_194_304;
const FRAME_CLOCK: u32 = 8192;
const DUTY: [u8; 4] = [0b0000_0001, 0b1000_0001, 0b1000_0111, 0b0111_1110];
const READ_MASK: [u8; 23] = [
    0x80, 0x3f, 0, 0xff, 0xbf, 0xff, 0x3f, 0, 0xff, 0xbf, 0x7f, 0xff, 0x9f, 0xff, 0xbf, 0xff, 0xff,
    0, 0, 0xbf, 0, 0, 0x70,
];

#[derive(Clone, Copy, Default)]
struct Envelope {
    initial: u8,
    volume: u8,
    pace: u8,
    timer: u8,
    rise: bool,
    running: bool,
}
impl Envelope {
    fn write(&mut self, data: u8) {
        // The consistent manual-increment case; other zombie cases vary by DMG revision.
        if self.running && self.pace == 0 && self.rise && data == 8 {
            self.volume = (self.volume + 1) & 15;
        }
        self.initial = data >> 4;
        self.rise = data & 8 != 0;
        self.pace = data & 7;
    }
    fn trigger(&mut self) {
        self.volume = self.initial;
        self.timer = if self.pace == 0 { 8 } else { self.pace };
        self.running = true;
    }
    fn clock(&mut self) {
        if !self.running {
            return;
        }
        self.timer = self.timer.saturating_sub(1);
        if self.timer > 0 {
            return;
        }
        self.timer = if self.pace == 0 { 8 } else { self.pace };
        if self.pace == 0 {
            return;
        }
        if self.rise && self.volume < 15 {
            self.volume += 1;
        } else if !self.rise && self.volume > 0 {
            self.volume -= 1;
        } else {
            self.running = false;
        }
    }
}
#[derive(Clone, Copy, Default)]
struct Pulse {
    enabled: bool,
    dac: bool,
    length_enable: bool,
    clocking: bool,
    first_step: bool,
    duty: u8,
    pos: u8,
    length: u16,
    frequency: u16,
    timer: i32,
    envelope: Envelope,
    sweep_pace: u8,
    sweep_shift: u8,
    sweep_down: bool,
    sweep_timer: u8,
    sweep_shadow: u16,
    sweep_enabled: bool,
    sweep_negated: bool,
}
impl Pulse {
    fn period(&self) -> i32 {
        (2048 - self.frequency as i32) * 4
    }
    fn trigger(&mut self, sweep: bool) {
        self.enabled = self.dac;
        if self.length == 0 {
            self.length = 64;
        }
        self.timer = self.period() + (self.timer & 3);
        if !self.clocking {
            self.first_step = true;
        }
        self.clocking = true;
        self.envelope.trigger();
        if sweep {
            self.sweep_shadow = self.frequency;
            self.sweep_timer = if self.sweep_pace == 0 {
                8
            } else {
                self.sweep_pace
            };
            self.sweep_enabled = self.sweep_pace != 0 || self.sweep_shift != 0;
            self.sweep_negated = false;
            if self.sweep_shift != 0 && self.calc_sweep().is_none() {
                self.enabled = false;
            }
        }
    }
    fn calc_sweep(&mut self) -> Option<u16> {
        let delta = self.sweep_shadow >> self.sweep_shift;
        let next = if self.sweep_down {
            self.sweep_negated = true;
            self.sweep_shadow.checked_sub(delta)?
        } else {
            self.sweep_shadow.checked_add(delta)?
        };
        (next <= 2047).then_some(next)
    }
    fn clock_sweep(&mut self) {
        self.sweep_timer = self.sweep_timer.saturating_sub(1);
        if self.sweep_timer != 0 {
            return;
        }
        self.sweep_timer = if self.sweep_pace == 0 {
            8
        } else {
            self.sweep_pace
        };
        if !self.sweep_enabled || self.sweep_pace == 0 {
            return;
        }
        if let Some(next) = self.calc_sweep() {
            if self.sweep_shift != 0 {
                self.sweep_shadow = next;
                self.frequency = next;
                if self.calc_sweep().is_none() {
                    self.enabled = false;
                }
            }
        } else {
            self.enabled = false;
        }
    }
    fn clocks(&mut self, n: i32) {
        if !self.clocking {
            return;
        }
        self.timer -= n;
        while self.timer <= 0 {
            self.timer += self.period();
            self.pos = (self.pos + 1) & 7;
            self.first_step = false;
        }
    }
    fn output(&self) -> Option<u8> {
        if !self.dac {
            return None;
        }
        Some(
            if self.enabled
                && !self.first_step
                && (DUTY[self.duty as usize] >> (7 - self.pos)) & 1 != 0
            {
                self.envelope.volume
            } else {
                0
            },
        )
    }
    fn clock_length(&mut self) {
        length_clock(self.length_enable, &mut self.length, &mut self.enabled);
    }
}
#[derive(Clone, Copy, Default)]
struct Wave {
    enabled: bool,
    dac: bool,
    length_enable: bool,
    length: u16,
    frequency: u16,
    timer: i32,
    index: u8,
    level: u8,
    buffer: u8,
    access: i32,
}
impl Wave {
    fn period(&self) -> i32 {
        (2048 - self.frequency as i32) * 2
    }
    fn trigger(&mut self) {
        self.enabled = self.dac;
        if self.length == 0 {
            self.length = 256;
        }
        self.timer = self.period() + 6;
        self.index = 0;
        self.access = 0;
    }
    fn clocks(&mut self, n: i32, ram: &[u8; 16]) {
        self.access = (self.access - n).max(0);
        if !self.enabled {
            return;
        }
        self.timer -= n;
        while self.timer <= 0 {
            self.timer += self.period();
            self.index = (self.index + 1) & 31;
            self.buffer = ram[(self.index / 2) as usize];
            self.access = 2;
        }
    }
    fn output(&self) -> Option<u8> {
        if !self.dac {
            return None;
        }
        if !self.enabled {
            return Some(0);
        }
        let sample = if self.index & 1 == 0 {
            self.buffer >> 4
        } else {
            self.buffer & 15
        };
        Some(match self.level {
            0 => 0,
            1 => sample,
            2 => sample >> 1,
            _ => sample >> 2,
        })
    }
    fn clock_length(&mut self) {
        length_clock(self.length_enable, &mut self.length, &mut self.enabled);
    }
}
#[derive(Clone, Copy)]
struct Noise {
    enabled: bool,
    dac: bool,
    length_enable: bool,
    length: u16,
    timer: i32,
    divisor: u8,
    shift: u8,
    short: bool,
    lfsr: u16,
    envelope: Envelope,
}
impl Default for Noise {
    fn default() -> Self {
        Self {
            enabled: false,
            dac: false,
            length_enable: false,
            length: 0,
            timer: 8,
            divisor: 0,
            shift: 0,
            short: false,
            lfsr: 0x7fff,
            envelope: Envelope::default(),
        }
    }
}
impl Noise {
    fn period(&self) -> i32 {
        (if self.divisor == 0 {
            8
        } else {
            16 * self.divisor as i32
        }) << self.shift
    }
    fn trigger(&mut self) {
        self.enabled = self.dac;
        if self.length == 0 {
            self.length = 64;
        }
        self.envelope.trigger();
        self.lfsr = 0x7fff;
        self.timer = self.period();
    }
    fn clocks(&mut self, n: i32) {
        if !self.enabled || self.shift >= 14 {
            return;
        }
        self.timer -= n;
        while self.timer <= 0 {
            self.timer += self.period();
            let xor = (self.lfsr ^ (self.lfsr >> 1)) & 1;
            self.lfsr = (self.lfsr >> 1) | (xor << 14);
            if self.short {
                self.lfsr = (self.lfsr & !(1 << 6)) | (xor << 6);
            }
        }
    }
    fn output(&self) -> Option<u8> {
        if !self.dac {
            return None;
        }
        Some(if self.enabled && self.lfsr & 1 == 0 {
            self.envelope.volume
        } else {
            0
        })
    }
    fn clock_length(&mut self) {
        length_clock(self.length_enable, &mut self.length, &mut self.enabled);
    }
}
fn length_clock(on: bool, length: &mut u16, enabled: &mut bool) {
    if on && *length > 0 {
        *length -= 1;
        if *length == 0 {
            *enabled = false;
        }
    }
}
fn length_write(
    old: bool,
    new: bool,
    trigger: bool,
    extra: bool,
    max: u16,
    length: &mut u16,
    enabled: &mut bool,
) {
    if !old && new && extra {
        length_clock(true, length, enabled);
    }
    if trigger && *length == 0 {
        *length = max - u16::from(new && extra);
    }
}

pub struct DmgApu {
    regs: [u8; 0x30],
    wave_ram: [u8; 16],
    first: Pulse,
    second: Pulse,
    wave: Wave,
    noise: Noise,
    powered: bool,
    frame_clock: u32,
    frame_step: u8,
    clock_frac: f64,
    rate: f64,
    pan_weights: [[i32; 4]; 2],
    blip: [BlipBuf; 2],
    last: [i32; 2],
    capacitor: [f32; 2],
    hp_coeff: f32,
}
impl DmgApu {
    pub fn new(rate: f32) -> Self {
        let rate = if rate.is_finite() && (8000.0..=192000.0).contains(&rate) {
            rate as f64
        } else {
            48000.0
        };
        let mut blip = [BlipBuf::new(4096), BlipBuf::new(4096)];
        for b in &mut blip {
            b.set_rates(DMG_CLOCK as f64, rate);
        }
        let mut regs = [0; 0x30];
        regs[0x14] = 0x77;
        regs[0x15] = 0xff;
        Self {
            regs,
            wave_ram: [0; 16],
            first: Pulse::default(),
            second: Pulse::default(),
            wave: Wave::default(),
            noise: Noise::default(),
            pan_weights: [[4096; 4]; 2],
            powered: true,
            frame_clock: 0,
            frame_step: 0,
            clock_frac: 0.0,
            rate,
            blip,
            last: [0; 2],
            capacitor: [0.0; 2],
            hp_coeff: 0.999958_f64.powf(DMG_CLOCK as f64 / rate) as f32,
        }
    }
    /// Additional production mixer, bypassed in Authentic. Weights are applied after each DAC.
    pub fn set_extended_pan(&mut self, pan: Option<[f32; 4]>) {
        self.pan_weights = [[4096; 4]; 2];
        if let Some(pan) = pan {
            for (ch, p) in pan.into_iter().enumerate() {
                let p = p.clamp(0.0, 1.0);
                self.pan_weights[0][ch] = ((2.0 * (1.0 - p)).min(1.0) * 4096.0).round() as i32;
                self.pan_weights[1][ch] = ((2.0 * p).min(1.0) * 4096.0).round() as i32;
            }
        }
    }
    pub fn wave_ram(&self) -> [u8; 16] {
        self.wave_ram
    }
    pub fn channel_levels(&self) -> [u8; 4] {
        [
            self.first.output(),
            self.second.output(),
            self.wave.output(),
            self.noise.output(),
        ]
        .map(|v| v.unwrap_or(0))
    }
    pub fn channel_status(&self) -> [bool; 4] {
        [
            self.first.enabled,
            self.second.enabled,
            self.wave.enabled,
            self.noise.enabled,
        ]
    }
    pub fn register(&self, addr: u16) -> u8 {
        if !(0xff10..=0xff3f).contains(&addr) {
            return 0xff;
        }
        if addr >= 0xff30 {
            return if self.wave.enabled {
                if self.wave.access > 0 {
                    self.wave_ram[(self.wave.index / 2) as usize]
                } else {
                    0xff
                }
            } else {
                self.wave_ram[(addr - 0xff30) as usize]
            };
        }
        if addr == 0xff26 {
            return 0x70
                | if self.powered { 0x80 } else { 0 }
                | self
                    .channel_status()
                    .iter()
                    .enumerate()
                    .fold(0, |m, (i, on)| m | (u8::from(*on) << i));
        }
        if addr > 0xff26 {
            return 0xff;
        }
        self.regs[(addr - 0xff10) as usize] | READ_MASK[(addr - 0xff10) as usize]
    }
    pub fn write(&mut self, addr: u16, data: u8) {
        if !(0xff10..=0xff3f).contains(&addr) {
            return;
        }
        if addr >= 0xff30 {
            if !self.wave.enabled {
                self.wave_ram[(addr - 0xff30) as usize] = data;
            } else if self.wave.access > 0 {
                self.wave_ram[(self.wave.index / 2) as usize] = data;
            }
            return;
        }
        if addr == 0xff26 {
            if data & 0x80 == 0 && self.powered {
                let lengths = [
                    self.first.length,
                    self.second.length,
                    self.wave.length,
                    self.noise.length,
                ];
                self.first = Pulse::default();
                self.second = Pulse::default();
                self.wave = Wave::default();
                self.noise = Noise::default();
                self.first.length = lengths[0];
                self.second.length = lengths[1];
                self.wave.length = lengths[2];
                self.noise.length = lengths[3];
                self.regs.fill(0);
                self.frame_step = 0;
            }
            self.powered = data & 0x80 != 0;
            return;
        }
        if !self.powered {
            match addr {
                0xff11 => self.first.length = 64 - (data & 63) as u16,
                0xff16 => self.second.length = 64 - (data & 63) as u16,
                0xff1b => self.wave.length = 256 - data as u16,
                0xff20 => self.noise.length = 64 - (data & 63) as u16,
                _ => {}
            }
            return;
        }
        if addr > 0xff25 || addr == 0xff15 || addr == 0xff1f {
            return;
        }
        self.regs[(addr - 0xff10) as usize] = data;
        let extra = self.frame_step & 1 != 0;
        match addr {
            0xff10 => {
                if self.first.sweep_down && data & 8 == 0 && self.first.sweep_negated {
                    self.first.enabled = false;
                }
                self.first.sweep_pace = (data >> 4) & 7;
                self.first.sweep_down = data & 8 != 0;
                self.first.sweep_shift = data & 7;
            }
            0xff11 => {
                self.first.duty = data >> 6;
                self.first.length = 64 - (data & 63) as u16;
            }
            0xff12 => {
                self.first.envelope.write(data);
                self.first.dac = data & 0xf8 != 0;
                if !self.first.dac {
                    self.first.enabled = false;
                }
            }
            0xff13 => self.first.frequency = (self.first.frequency & 0x700) | data as u16,
            0xff14 => {
                self.first.frequency = (self.first.frequency & 0xff) | (((data & 7) as u16) << 8);
                length_write(
                    self.first.length_enable,
                    data & 0x40 != 0,
                    data & 0x80 != 0,
                    extra,
                    64,
                    &mut self.first.length,
                    &mut self.first.enabled,
                );
                self.first.length_enable = data & 0x40 != 0;
                if data & 0x80 != 0 {
                    self.first.trigger(true);
                    if self.frame_step == 7 {
                        self.first.envelope.timer += 1;
                    }
                }
            }
            0xff16 => {
                self.second.duty = data >> 6;
                self.second.length = 64 - (data & 63) as u16;
            }
            0xff17 => {
                self.second.envelope.write(data);
                self.second.dac = data & 0xf8 != 0;
                if !self.second.dac {
                    self.second.enabled = false;
                }
            }
            0xff18 => self.second.frequency = (self.second.frequency & 0x700) | data as u16,
            0xff19 => {
                self.second.frequency = (self.second.frequency & 0xff) | (((data & 7) as u16) << 8);
                length_write(
                    self.second.length_enable,
                    data & 0x40 != 0,
                    data & 0x80 != 0,
                    extra,
                    64,
                    &mut self.second.length,
                    &mut self.second.enabled,
                );
                self.second.length_enable = data & 0x40 != 0;
                if data & 0x80 != 0 {
                    self.second.trigger(false);
                    if self.frame_step == 7 {
                        self.second.envelope.timer += 1;
                    }
                }
            }
            0xff1a => {
                self.wave.dac = data & 0x80 != 0;
                if !self.wave.dac {
                    self.wave.enabled = false;
                }
            }
            0xff1b => self.wave.length = 256 - data as u16,
            0xff1c => self.wave.level = (data >> 5) & 3,
            0xff1d => self.wave.frequency = (self.wave.frequency & 0x700) | data as u16,
            0xff1e => {
                self.wave.frequency = (self.wave.frequency & 0xff) | (((data & 7) as u16) << 8);
                length_write(
                    self.wave.length_enable,
                    data & 0x40 != 0,
                    data & 0x80 != 0,
                    extra,
                    256,
                    &mut self.wave.length,
                    &mut self.wave.enabled,
                );
                self.wave.length_enable = data & 0x40 != 0;
                if data & 0x80 != 0 {
                    if self.wave.enabled && self.wave.access > 0 {
                        let byte = (self.wave.index / 2) as usize;
                        if byte < 4 {
                            self.wave_ram[0] = self.wave_ram[byte];
                        } else {
                            let block = byte & !3;
                            self.wave_ram.copy_within(block..block + 4, 0);
                        }
                    }
                    self.wave.trigger();
                }
            }
            0xff20 => self.noise.length = 64 - (data & 63) as u16,
            0xff21 => {
                self.noise.envelope.write(data);
                self.noise.dac = data & 0xf8 != 0;
                if !self.noise.dac {
                    self.noise.enabled = false;
                }
            }
            0xff22 => {
                self.noise.shift = data >> 4;
                self.noise.short = data & 8 != 0;
                self.noise.divisor = data & 7;
            }
            0xff23 => {
                length_write(
                    self.noise.length_enable,
                    data & 0x40 != 0,
                    data & 0x80 != 0,
                    extra,
                    64,
                    &mut self.noise.length,
                    &mut self.noise.enabled,
                );
                self.noise.length_enable = data & 0x40 != 0;
                if data & 0x80 != 0 {
                    self.noise.trigger();
                    if self.frame_step == 7 {
                        self.noise.envelope.timer += 1;
                    }
                }
            }
            _ => {}
        }
    }
    fn clock_frame(&mut self) {
        if self.frame_step & 1 == 0 {
            self.first.clock_length();
            self.second.clock_length();
            self.wave.clock_length();
            self.noise.clock_length();
        }
        if self.frame_step == 2 || self.frame_step == 6 {
            self.first.clock_sweep();
        }
        if self.frame_step == 7 {
            self.first.envelope.clock();
            self.second.envelope.clock();
            self.noise.envelope.clock();
        }
        self.frame_step = (self.frame_step + 1) & 7;
    }
    /// CPU DIV reset seam, including the extra falling edge when bit 12 was high.
    pub fn divider_reset(&mut self) {
        if self.frame_clock >= FRAME_CLOCK / 2 && self.powered {
            self.clock_frame();
        }
        self.frame_clock = 0;
    }
    fn delta(&mut self, time: u32) {
        let signals = [
            self.first.output(),
            self.second.output(),
            self.wave.output(),
            self.noise.output(),
        ];
        for side in 0..2 {
            let volume = if side == 0 {
                ((self.regs[0x14] >> 4) & 7) + 1
            } else {
                (self.regs[0x14] & 7) + 1
            };
            let mut mixed = 0;
            if self.powered {
                for (ch, signal) in signals.iter().enumerate() {
                    if self.regs[0x15] & (1 << (ch + if side == 0 { 4 } else { 0 })) != 0
                        && let Some(level) = signal
                    {
                        mixed += (15 - 2 * (*level as i32)) * self.pan_weights[side][ch];
                    }
                }
            }
            let level = mixed * volume as i32 * 48 / 4096;
            if level != self.last[side] {
                self.blip[side].add_delta(time, level - self.last[side]);
                self.last[side] = level;
            }
        }
    }
    pub fn step_cycles(&mut self, mut cycles: u32) {
        // Bound blip frames even when a register test advances whole seconds.
        while cycles > 0 {
            let frame = cycles.min(DMG_CLOCK / 1000);
            let mut elapsed = 0;
            self.delta(0);
            while elapsed < frame {
                let mut n = (frame - elapsed).min(FRAME_CLOCK - self.frame_clock);
                if self.powered {
                    for timer in [
                        if self.first.clocking {
                            self.first.timer
                        } else {
                            i32::MAX
                        },
                        if self.second.clocking {
                            self.second.timer
                        } else {
                            i32::MAX
                        },
                        if self.wave.enabled {
                            self.wave.timer
                        } else {
                            i32::MAX
                        },
                        if self.noise.enabled && self.noise.shift < 14 {
                            self.noise.timer
                        } else {
                            i32::MAX
                        },
                    ] {
                        n = n.min(timer.max(1) as u32);
                    }
                    self.first.clocks(n as i32);
                    self.second.clocks(n as i32);
                    self.wave.clocks(n as i32, &self.wave_ram);
                    self.noise.clocks(n as i32);
                }
                elapsed += n;
                self.frame_clock += n;
                if self.frame_clock == FRAME_CLOCK {
                    self.frame_clock = 0;
                    if self.powered {
                        self.clock_frame();
                    }
                }
                self.delta(elapsed);
            }
            for b in &mut self.blip {
                b.end_frame(frame);
            }
            // Public long clock advances are register tests, not audio rendering.
            // Retain a bounded tail; ordinary render() never reaches this condition.
            if self.blip[0].samples_avail() > 2048 {
                let mut discard = [0; 1024];
                for b in &mut self.blip {
                    b.read_samples(&mut discard, false);
                }
            }
            cycles -= frame;
        }
    }
    pub fn render(&mut self) -> (f32, f32) {
        while self.blip[0].samples_avail() == 0 {
            self.clock_frac += DMG_CLOCK as f64 / self.rate;
            let cycles = self.clock_frac.floor() as u32;
            self.clock_frac -= cycles as f64;
            self.step_cycles(cycles.max(1));
        }
        let mut output = [0.0; 2];
        let connected = self.first.dac || self.second.dac || self.wave.dac || self.noise.dac;
        for (side, out) in output.iter_mut().enumerate() {
            let mut sample = [0];
            self.blip[side].read_samples(&mut sample, false);
            let input = sample[0] as f32 / 32768.0;
            if connected && self.powered {
                *out = input - self.capacitor[side];
                self.capacitor[side] = input - *out * self.hp_coeff;
                if self.capacitor[side].abs() < 1e-20 {
                    self.capacitor[side] = 0.0;
                }
            }
        }
        (output[0], output[1])
    }
    /// MIDI release is translated to a real DAC-disable register write; the next trigger restores it.
    pub fn stop_channel(&mut self, ch: usize) {
        let register = match ch {
            0 => 0xff12,
            1 => 0xff17,
            2 => 0xff1a,
            3 => 0xff21,
            _ => return,
        };
        self.write(register, 0);
    }
    pub fn stop_midi(&mut self) {
        for ch in 0..4 {
            self.stop_channel(ch);
        }
    }
    pub fn reset(&mut self) {
        let rate = self.rate;
        self.first = Pulse::default();
        self.second = Pulse::default();
        self.wave = Wave::default();
        self.noise = Noise::default();
        self.regs.fill(0);
        self.powered = true;
        self.frame_clock = 0;
        self.frame_step = 0;
        self.clock_frac = 0.0;
        self.capacitor = [0.0; 2];
        self.last = [0; 2];
        for b in &mut self.blip {
            b.clear();
            b.set_rates(DMG_CLOCK as f64, rate);
        }
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
        let mut a = Noise {
            dac: true,
            ..Noise::default()
        };
        a.trigger();
        a.clocks(8 * 16);
        let long = a.lfsr;
        a.short = true;
        a.trigger();
        a.clocks(8 * 16);
        assert_ne!(long, a.lfsr);
    }
    #[test]
    fn audio_finite_across_common_rates() {
        for rate in [8_000.0, 32_768.0, 44_100.0, 48_000.0, 96_000.0] {
            let mut a = DmgApu::new(rate);
            a.write(0xff12, 0xf0);
            a.write(0xff11, 0x80);
            a.write(0xff13, 0x00);
            a.write(0xff14, 0x87);
            let samples = (0..4096).map(|_| a.render()).collect::<Vec<_>>();
            assert!(samples.iter().all(|(l, r)| l.is_finite() && r.is_finite()));
            assert!(samples.iter().any(|(l, _)| l.abs() > 0.001));
        }
    }
}

#[cfg(test)]
mod regression_tests {
    use super::*;
    #[test]
    fn read_masks_and_powered_off_dmg_lengths() {
        let mut a = DmgApu::new(48000.0);
        a.write(0xff13, 0x12);
        assert_eq!(a.register(0xff13), 0xff);
        a.write(0xff11, 0x3f);
        a.write(0xff26, 0);
        a.write(0xff11, 0x3e);
        a.write(0xff12, 0xf0);
        assert_eq!(a.first.length, 2);
        assert!(!a.first.dac);
        a.write(0xff26, 0x80);
        assert_eq!(a.first.length, 2);
    }
    #[test]
    fn retrigger_keeps_pulse_duty_position() {
        let mut a = DmgApu::new(48000.0);
        a.write(0xff12, 0xf0);
        a.write(0xff13, 0xf0);
        a.write(0xff14, 0x87);
        a.step_cycles(64 * 3);
        let pos = a.first.pos;
        a.write(0xff14, 0x87);
        assert_eq!(a.first.pos, pos);
    }
    #[test]
    fn sweep_negate_clear_disables_channel() {
        let mut a = DmgApu::new(48000.0);
        a.write(0xff12, 0xf0);
        a.write(0xff10, 0x19);
        a.write(0xff13, 0);
        a.write(0xff14, 0x84);
        assert!(a.first.sweep_negated);
        a.write(0xff10, 0x11);
        assert!(!a.first.enabled);
    }
    #[test]
    fn length_enable_has_an_extra_clock_on_odd_step() {
        let mut a = DmgApu::new(48000.0);
        a.write(0xff12, 0xf0);
        a.write(0xff11, 63);
        a.write(0xff14, 0x80);
        a.step_cycles(FRAME_CLOCK);
        assert_eq!(a.first.length, 1);
        a.write(0xff14, 0x40);
        assert!(!a.first.enabled);
        assert_eq!(a.first.length, 0);
    }
    #[test]
    fn shifts_fourteen_and_fifteen_do_not_clock_noise() {
        for shift in [14, 15] {
            let mut n = Noise {
                dac: true,
                shift,
                ..Noise::default()
            };
            n.trigger();
            let initial = n.lfsr;
            n.clocks(4_194_304);
            assert_eq!(n.lfsr, initial);
        }
    }
    #[test]
    fn disabled_generator_keeps_its_enabled_dac_connected() {
        let mut a = DmgApu::new(48000.0);
        a.write(0xff12, 0x80);
        assert_eq!(a.first.output(), Some(0));
        a.delta(0);
        assert!(a.last[0] > 0);
        a.write(0xff12, 0);
        assert_eq!(a.first.output(), None);
    }
    #[test]
    fn wave_trigger_keeps_the_previous_sample_buffer() {
        let mut a = DmgApu::new(48000.0);
        a.write(0xff30, 0xab);
        a.write(0xff1a, 0x80);
        a.write(0xff1c, 0x20);
        a.write(0xff1d, 0xff);
        a.write(0xff1e, 0x87);
        assert_eq!(a.wave.output(), Some(0));
        a.step_cycles(8);
        assert_eq!(a.wave.output(), Some(11));
    }
    #[test]
    fn band_limited_pulse_rejects_folded_fifth_harmonic() {
        fn energy(samples: &[f32], hz: f32) -> f32 {
            let coefficient = 2.0 * (std::f32::consts::TAU * hz / 48000.0).cos();
            let (mut a, mut b) = (0.0, 0.0);
            for &s in samples {
                let q = s + coefficient * a - b;
                b = a;
                a = q;
            }
            (a * a + b * b - coefficient * a * b).max(0.0)
        }
        let mut a = DmgApu::new(48000.0);
        a.write(0xff11, 0x80);
        a.write(0xff12, 0xf0);
        a.write(0xff13, 0xef);
        a.write(0xff14, 0x87);
        let samples: Vec<_> = (0..16384).map(|_| a.render().0).collect();
        let f = DMG_CLOCK as f32 / (68.0 * 8.0);
        assert!(
            energy(&samples[1024..], (5.0 * f - 48000.0).abs())
                < energy(&samples[1024..], f) * 0.0036
        );
    }
}
