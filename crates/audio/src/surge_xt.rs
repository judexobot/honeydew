//! Android host adapter for the **unmodified Surge XT C++ DSP core**.
//!
//! No alternate oscillator, filter or patch engine lives here: all samples come
//! from libhoneydew_surge_xt.so, built against Surge's original surge-common.
//! The library is loaded lazily outside the audio callback. Its absence is a
//! hard instrument-construction failure, not a silent fake synth.
#![cfg(target_os = "android")]

use crate::ProcessContext;
use crate::instrument::Instrument;
use std::ffi::{c_char, c_void};
use std::sync::OnceLock;

const BLOCK: usize = 32;
const RTLD_NOW: i32 = 2;
const LIBRARY: &[u8] = b"libhoneydew_surge_xt.so\0";

#[link(name = "dl")]
unsafe extern "C" {
    fn dlopen(file: *const c_char, flags: i32) -> *mut c_void;
    fn dlsym(handle: *mut c_void, symbol: *const c_char) -> *mut c_void;
    fn dlclose(handle: *mut c_void) -> i32;
}

type Create = unsafe extern "C" fn(f32) -> *mut c_void;
type Destroy = unsafe extern "C" fn(*mut c_void);
type NoteOn = unsafe extern "C" fn(*mut c_void, u8, u8, u8);
type NoteOff = unsafe extern "C" fn(*mut c_void, u8, u8);
type All = unsafe extern "C" fn(*mut c_void);
type Cc = unsafe extern "C" fn(*mut c_void, u8, u8, u8);
type Bend = unsafe extern "C" fn(*mut c_void, u8, i32);
type Macro = unsafe extern "C" fn(*mut c_void, i32, f32);
type Render = unsafe extern "C" fn(*mut c_void, *mut f32, *mut f32, f64, f64, i32) -> i32;

#[derive(Clone, Copy)]
struct Api {
    create: Create,
    destroy: Destroy,
    on: NoteOn,
    off: NoteOff,
    notes_off: All,
    panic: All,
    cc: Cc,
    bend: Bend,
    macro_value: Macro,
    render: Render,
}

impl Api {
    // All symbols must exist before an instance is created. The dlopen handle
    // intentionally lives until process exit; no audio callback can race dlclose.
    unsafe fn load() -> Option<Self> {
        let handle = unsafe { dlopen(LIBRARY.as_ptr().cast(), RTLD_NOW) };
        if handle.is_null() {
            return None;
        }
        unsafe fn symbol<T: Copy>(lib: *mut c_void, name: &[u8]) -> Option<T> {
            let raw = unsafe { dlsym(lib, name.as_ptr().cast()) };
            (!raw.is_null()).then(|| unsafe { std::mem::transmute_copy::<*mut c_void, T>(&raw) })
        }
        let result = (|| Some(Self {
            create: unsafe { symbol(handle, b"honeydew_surge_create\0")? },
            destroy: unsafe { symbol(handle, b"honeydew_surge_destroy\0")? },
            on: unsafe { symbol(handle, b"honeydew_surge_note_on\0")? },
            off: unsafe { symbol(handle, b"honeydew_surge_note_off\0")? },
            notes_off: unsafe { symbol(handle, b"honeydew_surge_all_notes_off\0")? },
            panic: unsafe { symbol(handle, b"honeydew_surge_panic\0")? },
            cc: unsafe { symbol(handle, b"honeydew_surge_cc\0")? },
            bend: unsafe { symbol(handle, b"honeydew_surge_pitch_bend\0")? },
            macro_value: unsafe { symbol(handle, b"honeydew_surge_macro\0")? },
            render: unsafe { symbol(handle, b"honeydew_surge_render32\0")? },
        }))();
        if result.is_none() {
            let _ = unsafe { dlclose(handle) };
        }
        result
    }
}

static API: OnceLock<Option<Api>> = OnceLock::new();

pub struct SurgeXt {
    api: Api,
    instance: *mut c_void,
    left: [f32; BLOCK],
    right: [f32; BLOCK],
    position: usize,
}

// The audio engine moves an Instrument between threads but never uses the same
// instance concurrently. C++ state belongs exclusively to this object.
unsafe impl Send for SurgeXt {}

impl SurgeXt {
    pub fn new(sample_rate: f32) -> Result<Self, &'static str> {
        let api = *API.get_or_init(|| unsafe { Api::load() })
            .as_ref().ok_or("original Surge XT native library is not installed")?;
        let instance = unsafe { (api.create)(sample_rate) };
        if instance.is_null() {
            return Err("Surge XT failed to initialize its original synthesizer engine");
        }
        Ok(Self {
            api,
            instance,
            left: [0.0; BLOCK],
            right: [0.0; BLOCK],
            position: BLOCK,
        })
    }

    fn midi_on(&mut self, channel: u8, key: u8, velocity: f32) {
        let velocity = (velocity.clamp(0.0, 1.0) * 127.0).round().max(1.0) as u8;
        unsafe { (self.api.on)(self.instance, channel, key, velocity) };
    }

    fn midi_off(&mut self, channel: u8, key: u8) {
        unsafe { (self.api.off)(self.instance, channel, key) };
    }
}

impl Drop for SurgeXt {
    fn drop(&mut self) {
        unsafe { (self.api.destroy)(self.instance) };
    }
}

impl Instrument for SurgeXt {
    fn set_parameter(&mut self, index: usize, value: f32) {
        // p0 is Honeydew's own bypass. p1-p8 are Surge's genuine macros.
        if (1..=8).contains(&index) && value.is_finite() {
            unsafe { (self.api.macro_value)(self.instance, (index - 1) as i32, value) };
        }
    }

    fn note_on(&mut self, key: u8, velocity: f32, _pan: f32) {
        self.midi_on(0, key, velocity);
    }

    fn note_off(&mut self, key: u8) {
        self.midi_off(0, key);
    }

    fn note_on_midi(&mut self, channel: u8, key: u8, velocity: f32, _pan: f32) {
        self.midi_on(channel, key, velocity);
    }

    fn note_off_midi(&mut self, channel: u8, key: u8) {
        self.midi_off(channel, key);
    }

    fn all_notes_off(&mut self) {
        unsafe { (self.api.notes_off)(self.instance) };
    }

    fn reset(&mut self) {
        unsafe { (self.api.panic)(self.instance) };
        self.left.fill(0.0);
        self.right.fill(0.0);
        self.position = BLOCK;
    }

    fn set_controller(&mut self, index: usize, value: f32) {
        if !value.is_finite() {
            return;
        }
        let value = value.clamp(0.0, 1.0);
        if index == 4 {
            unsafe { (self.api.bend)(self.instance, 0, (value * 16383.0).round() as i32) };
        } else if (7..=134).contains(&index) {
            unsafe { (self.api.cc)(self.instance, 0, (index - 7) as u8,
                (value * 127.0).round() as u8) };
        }
    }

    fn set_midi_controller(&mut self, channel: u8, index: usize, value: f32) {
        if !value.is_finite() {
            return;
        }
        let value = value.clamp(0.0, 1.0);
        if index == 4 {
            unsafe { (self.api.bend)(self.instance, channel, (value * 16383.0).round() as i32) };
        } else if (7..=134).contains(&index) {
            unsafe { (self.api.cc)(self.instance, channel, (index - 7) as u8,
                (value * 127.0).round() as u8) };
        }
    }

    fn process(&mut self, left: &mut [f32], right: &mut [f32], context: ProcessContext) {
        let rate = (context.sample_rate as f64).max(1.0);
        for (index, (l, r)) in left.iter_mut().zip(right.iter_mut()).enumerate() {
            if self.position == BLOCK {
                let ppq = context.start_beat + (index as f64 * context.tempo_bpm as f64) / (rate * 60.0);
                let ok = unsafe { (self.api.render)(
                    self.instance, self.left.as_mut_ptr(), self.right.as_mut_ptr(),
                    ppq, context.tempo_bpm as f64, i32::from(context.playing),
                ) };
                if ok == 0 {
                    self.left.fill(0.0);
                    self.right.fill(0.0);
                }
                self.position = 0;
            }
            *l += self.left[self.position];
            *r += self.right[self.position];
            self.position += 1;
        }
    }
}
