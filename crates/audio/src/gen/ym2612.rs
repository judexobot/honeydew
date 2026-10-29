//! Safe, preallocated musical host for the LGPL Nuked-OPN2 register core.
//! Writes are coalesced by register and paced by the chip bus; keys retain retrigger edges.
use blip_buf::BlipBuf;
use std::sync::Once;
const NTSC: f64 = 7_670_454.0;
const PAL: f64 = 7_600_489.0;
#[repr(C, align(8))]
struct State([u8; 4096]);
unsafe extern "C" {
    fn hd_ym_init_type();
    fn hd_ym_reset(state: *mut State);
    fn hd_ym_write(state: *mut State, register: u32, value: u32);
    fn hd_ym_sample(state: *mut State, output: *mut i32);
    fn hd_ym_levels(state: *const State, output: *mut u16);
    fn hd_ym_pan(state: *mut State, channel: u32, left: u32, right: u32);
    fn hd_ym_lfo(state: *const State) -> u32;
}
static TYPE: Once = Once::new();
pub(super) struct Ym2612 {
    state: Box<State>,
    blip: [BlipBuf; 2],
    previous: [i32; 2],
    clock: f64,
    capacitor: [f32; 2],
    hp: f32,
    levels: [u16; 24],
}
impl Ym2612 {
    pub fn new(rate: f32) -> Self {
        // The upstream mode is process-global, set once before any instance can render.
        // This synchronization occurs only during construction on the non-audio thread.
        TYPE.call_once(|| unsafe { hd_ym_init_type() });
        let mut state = Box::new(State([0; 4096]));
        // SAFETY: State is aligned and sized for the adapter; its C static assertion enforces the bound.
        unsafe {
            hd_ym_reset(&mut *state);
        }
        let mut blip = [BlipBuf::new(4096), BlipBuf::new(4096)];
        for b in &mut blip {
            b.set_rates(NTSC / 144.0, f64::from(rate));
        }
        Self {
            state,
            blip,
            previous: [0; 2],
            clock: NTSC,
            capacitor: [0.0; 2],
            hp: (-std::f32::consts::TAU * 12.0 / rate).exp(),
            levels: [1023; 24],
        }
    }
    pub fn reset(&mut self) {
        unsafe {
            hd_ym_reset(&mut *self.state);
        }
        for b in &mut self.blip {
            b.clear();
        }
        self.previous = [0; 2];
        self.capacitor = [0.0; 2];
        self.levels.fill(1023);
    }
    pub fn clock(&self) -> f64 {
        self.clock
    }
    pub fn set_pal(&mut self, pal: bool, rate: f32) {
        let clock = if pal { PAL } else { NTSC };
        if clock != self.clock {
            self.clock = clock;
            for b in &mut self.blip {
                b.set_rates(clock / 144.0, f64::from(rate));
            }
        }
    }
    pub fn write(&mut self, register: u16, value: u8) {
        unsafe {
            hd_ym_write(&mut *self.state, u32::from(register), u32::from(value));
        }
    }
    pub fn key(&mut self, channel: usize, on: bool) {
        self.write(
            0x28,
            (channel + usize::from(channel >= 3)) as u8 | if on { 0xf0 } else { 0 },
        );
    }
    pub fn render(&mut self) -> [f32; 2] {
        while self.blip[0].samples_avail() == 0 {
            let mut sample = [0i32; 2];
            unsafe {
                hd_ym_sample(&mut *self.state, sample.as_mut_ptr());
            }
            for (i, b) in self.blip.iter_mut().enumerate() {
                let amplitude = sample[i] * 4;
                b.add_delta(0, amplitude - self.previous[i]);
                self.previous[i] = amplitude;
                b.end_frame(1);
            }
        }
        let mut result = [0.0; 2];
        for (i, b) in self.blip.iter_mut().enumerate() {
            let mut sample = [0i16];
            b.read_samples(&mut sample, false);
            let x = f32::from(sample[0]) / 8192.0;
            result[i] = x - self.capacitor[i];
            self.capacitor[i] = x - result[i] * self.hp;
            if self.capacitor[i].abs() < 1e-20 {
                self.capacitor[i] = 0.0;
            }
        }
        result
    }
    pub fn update_levels(&mut self) {
        unsafe {
            hd_ym_levels(&*self.state, self.levels.as_mut_ptr());
        }
    }
    pub fn pan(&mut self, channel: usize, position: f32) {
        let left = (2.0 * (1.0 - position)).min(1.0);
        let right = (2.0 * position).min(1.0);
        unsafe {
            hd_ym_pan(
                &mut *self.state,
                channel as u32,
                (left * 256.0).round() as u32,
                (right * 256.0).round() as u32,
            );
        }
    }
    pub fn lfo_phase(&self) -> f32 {
        unsafe { hd_ym_lfo(&*self.state) as f32 / 128.0 }
    }
    pub fn active(&self, channel: usize) -> bool {
        (0..4).any(|op| self.levels[channel + 6 * op] < 1010)
    }
    pub fn level(&self, channel: usize, op: usize) -> f32 {
        let slot = [0, 2, 1, 3][op];
        2.0f32.powf(-f32::from(self.levels[channel + 6 * slot]) / 64.0)
    }
}
