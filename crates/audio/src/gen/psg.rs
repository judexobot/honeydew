//! Sega PSG: latched 10-bit tone/4-bit attenuation registers, clocked LFSR and BLEP output.
//! Independent implementation; timing facts cross-checked against MAME's BSD reference.
use blip_buf::BlipBuf;
pub(super) struct Psg {
    registers: [u16; 8],
    latch: usize,
    timers: [u32; 4],
    outputs: [bool; 3],
    noise: u16,
    blip: [BlipBuf; 2],
    previous: [i32; 2],
    gains: [f32; 16],
    pan: [[f32; 2]; 4],
    clock: f64,
    rate: f64,
    fraction: f64,
}
impl Psg {
    pub fn new(rate: f32) -> Self {
        let mut blip = [BlipBuf::new(4096), BlipBuf::new(4096)];
        for b in &mut blip {
            b.set_rates(3_579_545.0, f64::from(rate));
        }
        let mut gains = [0.0; 16];
        for (i, g) in gains.iter_mut().enumerate().take(15) {
            *g = 10.0f32.powf(-(i as f32) * 2.0 / 20.0);
        }
        Self {
            registers: [0, 15, 0, 15, 0, 15, 0, 15],
            latch: 0,
            timers: [16; 4],
            outputs: [false; 3],
            noise: 0x8000,
            blip,
            previous: [0; 2],
            gains,
            pan: [[1.0; 2]; 4],
            clock: 3_579_545.0,
            rate: f64::from(rate),
            fraction: 0.0,
        }
    }
    pub fn reset(&mut self) {
        self.registers = [0, 15, 0, 15, 0, 15, 0, 15];
        self.latch = 0;
        self.timers = [16; 4];
        self.outputs = [false; 3];
        self.noise = 0x8000;
        self.previous = [0; 2];
        self.fraction = 0.0;
        for b in &mut self.blip {
            b.clear();
        }
    }
    pub fn set_pal(&mut self, pal: bool) {
        let c = if pal { 3_546_893.0 } else { 3_579_545.0 };
        if self.clock != c {
            self.clock = c;
            for b in &mut self.blip {
                b.set_rates(c, self.rate);
            }
        }
    }
    pub fn write(&mut self, value: u8) {
        if value & 0x80 != 0 {
            self.latch = usize::from((value >> 4) & 7);
            self.registers[self.latch] =
                (self.registers[self.latch] & 0x3f0) | u16::from(value & 15);
        } else if self.latch < 6 && self.latch & 1 == 0 {
            self.registers[self.latch] =
                (self.registers[self.latch] & 15) | (u16::from(value & 63) << 4);
        } else {
            self.registers[self.latch] = u16::from(value & 15);
        }
        if self.latch == 6 {
            self.noise = 0x8000;
            self.timers[3] = self.period(3);
        }
    }
    pub fn tone(&mut self, ch: usize, hz: f32, level: f32, pan: f32) {
        let period = (self.clock as f32 / (32.0 * hz.max(1.0)))
            .round()
            .clamp(1.0, 1023.0) as u16;
        if self.registers[ch * 2] != period {
            self.write(0x80 | ((ch as u8) * 32) | (period as u8 & 15));
            self.write((period >> 4) as u8);
        }
        self.volume(ch, level);
        self.pan[ch] = [(2.0 * (1.0 - pan)).min(1.0), (2.0 * pan).min(1.0)];
    }
    pub fn volume(&mut self, ch: usize, level: f32) {
        let attenuation = if level <= 0.0 {
            15
        } else {
            (-20.0 * level.log10() / 2.0).round().clamp(0.0, 14.0) as u8
        };
        if self.registers[ch * 2 + 1] != u16::from(attenuation) {
            self.write(0x90 | ((ch as u8) * 32) | attenuation);
        }
    }
    pub fn noise(&mut self, kind: u8, level: f32) {
        if self.registers[6] != u16::from(kind & 7) {
            self.write(0xe0 | (kind & 7));
        }
        self.volume(3, level);
    }
    fn period(&self, ch: usize) -> u32 {
        if ch < 3 {
            u32::from(self.registers[ch * 2]).max(1) * 16
        } else {
            match self.registers[6] & 3 {
                3 => u32::from(self.registers[4]).max(1) * 32,
                n => 512 << n,
            }
        }
    }
    fn mix(&self) -> [i32; 2] {
        let mut out = [0.0; 2];
        for ch in 0..4 {
            let signal = if ch < 3 {
                if self.outputs[ch] { 1.0 } else { -1.0 }
            } else if self.noise & 1 != 0 {
                1.0
            } else {
                -1.0
            };
            let level = self.gains[(self.registers[ch * 2 + 1] & 15) as usize];
            for (i, o) in out.iter_mut().enumerate() {
                *o -= signal * level * self.pan[ch][i] * 1800.0;
            }
        }
        [out[0].round() as i32, out[1].round() as i32]
    }
    fn advance(&mut self, cycles: u32) {
        let mut at = 0;
        while at < cycles {
            let value = self.mix();
            for (i, b) in self.blip.iter_mut().enumerate() {
                b.add_delta(at, value[i] - self.previous[i]);
                self.previous[i] = value[i];
            }
            let step = (cycles - at).min(*self.timers.iter().min().unwrap());
            for t in &mut self.timers {
                *t -= step;
            }
            at += step;
            for ch in 0..4 {
                if self.timers[ch] == 0 {
                    self.timers[ch] = self.period(ch);
                    if ch < 3 {
                        self.outputs[ch] = !self.outputs[ch];
                    } else {
                        let feedback = if self.registers[6] & 4 != 0 {
                            (self.noise ^ (self.noise >> 3)) & 1
                        } else {
                            self.noise & 1
                        };
                        self.noise = (self.noise >> 1) | (feedback << 15);
                    }
                }
            }
        }
        for b in &mut self.blip {
            b.end_frame(cycles);
        }
    }
    pub fn render(&mut self) -> [f32; 2] {
        while self.blip[0].samples_avail() == 0 {
            self.fraction += self.clock / self.rate;
            let cycles = self.fraction.floor() as u32;
            self.fraction -= f64::from(cycles);
            self.advance(cycles.max(1));
        }
        let mut out = [0.0; 2];
        for (i, b) in self.blip.iter_mut().enumerate() {
            let mut sample = [0];
            b.read_samples(&mut sample, false);
            out[i] = f32::from(sample[0]) / 4096.0;
        }
        out
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn sega_register_latch_and_zero_period() {
        let mut p = Psg::new(48000.0);
        p.write(0x85);
        p.write(0x2a);
        assert_eq!(p.registers[0], 0x2a5);
        p.write(0x80);
        p.write(0);
        assert_eq!(p.period(0), 16);
        p.write(0x9f);
        assert_eq!(p.registers[1], 15);
    }
    #[test]
    fn periodic_lfsr_and_noise_dividers() {
        let mut p = Psg::new(48000.0);
        p.write(0xe0);
        assert_eq!(p.period(3), 512);
        p.advance(512 * 16);
        assert_eq!(p.noise, 0x8000);
        p.write(0xe2);
        assert_eq!(p.period(3), 2048);
        p.write(0xc5);
        p.write(0x12);
        p.write(0xe3);
        assert_eq!(p.period(3), 0x125 * 32);
    }
}
