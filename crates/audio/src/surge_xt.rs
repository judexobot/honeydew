//! Android host adapter for the **unmodified Surge XT C++ DSP core**.
//!
//! No alternate oscillator, filter or patch engine lives here: all samples come
//! from libhoneydew_surge_xt.so, built against Surge's original surge-common.
//! The library is loaded lazily outside the audio callback. Its absence is a
//! hard instrument-construction failure, not a silent fake synth.
#![cfg(target_os = "android")]

use crate::ProcessContext;
use crate::instrument::Instrument;
use std::ffi::{c_char, c_void, CStr, CString};
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::OnceLock;

static DIAGNOSTIC: AtomicI32 = AtomicI32::new(0);
/// 0: untested; 2: original DSP generated audible PCM; negative: failure.
/// This status is shared with the Surge panel so silent engine failures are visible.
pub fn diagnostic() -> i32 { DIAGNOSTIC.load(Ordering::Relaxed) }
fn status(code: i32, reason: &str) {
    DIAGNOSTIC.store(code, Ordering::Relaxed);
    if let Ok(msg) = CString::new(reason) {
        unsafe { __android_log_write(6, b"HoneydewSurgeXT\0".as_ptr().cast(), msg.as_ptr()); }
    }
}
#[link(name = "log")]
unsafe extern "C" {
    fn __android_log_write(priority: i32, tag: *const c_char, text: *const c_char) -> i32;
}


const BLOCK: usize = 32;
/// Kept in lockstep with the official pinned upstream factory catalog.
pub const FACTORY_PRESET_COUNT: usize = 641;
const RTLD_NOW: i32 = 2;
const LIBRARY: &[u8] = b"libhoneydew_surge_xt.so\0";

#[link(name = "dl")]
unsafe extern "C" {
    fn dlopen(file: *const c_char, flags: i32) -> *mut c_void;
    fn dlsym(handle: *mut c_void, symbol: *const c_char) -> *mut c_void;
    fn dlclose(handle: *mut c_void) -> i32;
    fn dlerror() -> *const c_char;
}

type Create = unsafe extern "C" fn(f32) -> *mut c_void;
type Destroy = unsafe extern "C" fn(*mut c_void);
type NoteOn = unsafe extern "C" fn(*mut c_void, u8, u8, u8);
type NoteOff = unsafe extern "C" fn(*mut c_void, u8, u8);
type All = unsafe extern "C" fn(*mut c_void);
type Cc = unsafe extern "C" fn(*mut c_void, u8, u8, u8);
type Bend = unsafe extern "C" fn(*mut c_void, u8, i32);
type Macro = unsafe extern "C" fn(*mut c_void, i32, f32);
type NativeParam = unsafe extern "C" fn(*mut c_void, i32, f32);
type FactoryPreset = unsafe extern "C" fn(*mut c_void, i32) -> i32;
type Probe = unsafe extern "C" fn(f32) -> f32;
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
    native_param: NativeParam,
    load_factory: FactoryPreset,
    probe: Probe,
    render: Render,
}

impl Api {
    // All symbols must exist before an instance is created. The dlopen handle
    // intentionally lives until process exit; no audio callback can race dlclose.
    unsafe fn load() -> Option<Self> {
        let handle = unsafe { dlopen(LIBRARY.as_ptr().cast(), RTLD_NOW) };
        if handle.is_null() {
            let err = unsafe { dlerror() };
            let reason = if err.is_null() { "no dlerror detail".into() } else {
                unsafe { CStr::from_ptr(err) }.to_string_lossy().into_owned()
            };
            status(-1, &format!("Cannot dlopen original Surge XT shared library: {reason}"));
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
            native_param: unsafe { symbol(handle, b"honeydew_surge_native_param\0")? },
            load_factory: unsafe { symbol(handle, b"honeydew_surge_load_factory\0")? },
            probe: unsafe { symbol(handle, b"honeydew_surge_probe\0")? },
            render: unsafe { symbol(handle, b"honeydew_surge_render32\0")? },
        }))();
        if result.is_none() {
            status(-2, "Surge XT shared library missing one or more C bridge symbols");
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
    // In preset-only mode there are no editor parameters to sync:
    // the original FXP owns all DSP settings and modulation assignments.
}

// The audio engine moves an Instrument between threads but never uses the same
// instance concurrently. C++ state belongs exclusively to this object.
unsafe impl Send for SurgeXt {}

impl SurgeXt {
    pub fn new(sample_rate: f32) -> Result<Self, &'static str> {
        Self::new_with_factory_preset(sample_rate, 0)
    }

    pub fn new_with_factory_preset(sample_rate: f32, preset: usize) -> Result<Self, &'static str> {
        if preset > FACTORY_PRESET_COUNT {
            return Err("Surge XT factory preset index out of range");
        }
        let api = *API.get_or_init(|| unsafe { Api::load() })
            .as_ref().ok_or("original Surge XT native library is not installed")?;
        // Run the initial PCM test BEFORE allocating the actual instrument:
        // temporarily constructing two full Surge XT engines could exhaust RAM
        // on Android phones. This is strictly off the realtime audio thread.
        // Test the *original* synth core with the official Init Saw patch,
        // rather than inferring audio success from a compiled .so. Run once
        // outside the realtime callback and report any silence in the UI.
        static PROBE: OnceLock<f32> = OnceLock::new();
        let peak = *PROBE.get_or_init(|| unsafe { (api.probe)(sample_rate) });
        if !peak.is_finite() || peak <= 0.00001 {
            status(-5, &format!("Original Surge XT generated no PCM; probe peak={peak}"));
        } else {
            DIAGNOSTIC.store(2, Ordering::Relaxed);
        }
        let instance = unsafe { (api.create)(sample_rate) };
        if instance.is_null() {
            status(-3, "Original Surge XT constructor returned null: engine initialization failed");
            return Err("Surge XT failed to initialize its original synthesizer engine");
        }
        if preset != 0 && unsafe { (api.load_factory)(instance, preset as i32) } == 0 {
            unsafe { (api.destroy)(instance) };
            status(-4, "Native Surge XT could not load the selected factory FXP patch");
            return Err("Original Surge XT factory preset could not be loaded");
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
    fn set_parameter(&mut self, _index: usize, _value: f32) {
        // Preset-only instrument: ignore Honeydew's legacy 1..32 registry
        // defaults, automation and periodic re-synchronization entirely.
        // Applying even a single host default would change an official
        // Surge XT factory patch (sometimes silencing it).
        // p33 (factory preset choice) is applied by build/new_with_factory_preset,
        // never in the realtime audio callback.
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
