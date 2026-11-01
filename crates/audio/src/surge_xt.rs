//! Host for Surge XT's original surge-common. Native Linux QA runs the same
//! C ABI, adapter and mixer; Android uses its NativeActivity private paths.
use crate::ProcessContext;
use crate::instrument::Instrument;
use std::ffi::{CStr, CString, c_char, c_void};
use std::sync::atomic::{AtomicI32, AtomicU8, AtomicUsize, Ordering};
use std::sync::{Mutex, OnceLock};

const BLOCK: usize = 32;
const ABI: u32 = 2;
pub const FACTORY_PRESET_COUNT: usize = 641;
static DIAGNOSTIC: AtomicI32 = AtomicI32::new(0);
static ERROR: Mutex<String> = Mutex::new(String::new());
static RENDER_ERROR: [AtomicU8; 512] = [const { AtomicU8::new(0) }; 512];
static RENDER_ERROR_LEN: AtomicUsize = AtomicUsize::new(0);

/// A library, B constructor, C patch, D MIDI, E native PCM; errors are negative.
pub fn diagnostic() -> i32 {
    DIAGNOSTIC.load(Ordering::Relaxed)
}

/// UI/app thread only. No formatting, allocation, logging or locks in process().
pub fn diagnostic_message() -> String {
    if diagnostic() == -6 {
        let len = RENDER_ERROR_LEN
            .load(Ordering::Acquire)
            .min(RENDER_ERROR.len());
        let bytes: Vec<u8> = RENDER_ERROR[..len]
            .iter()
            .map(|b| b.load(Ordering::Relaxed))
            .collect();
        return format!(
            "E: Surge XT PCM failed: {}",
            String::from_utf8_lossy(&bytes)
        );
    }
    ERROR.lock().unwrap_or_else(|p| p.into_inner()).clone()
}

fn fail(stage: i32, reason: String) -> String {
    DIAGNOSTIC.store(stage, Ordering::Relaxed);
    *ERROR.lock().unwrap_or_else(|p| p.into_inner()) = reason.clone();
    #[cfg(target_os = "android")]
    if let Ok(message) = CString::new(reason.as_str()) {
        unsafe {
            __android_log_write(6, c"HoneydewSurgeXT".as_ptr(), message.as_ptr());
        }
    }
    reason
}

fn reached(stage: i32) {
    let _ = DIAGNOSTIC.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |old| {
        (old >= 0 && old < stage).then_some(stage)
    });
}

#[cfg(target_os = "android")]
#[link(name = "log")]
unsafe extern "C" {
    fn __android_log_write(priority: i32, tag: *const c_char, message: *const c_char) -> i32;
}

#[link(name = "dl")]
unsafe extern "C" {
    fn dlopen(file: *const c_char, flags: i32) -> *mut c_void;
    fn dlsym(handle: *mut c_void, symbol: *const c_char) -> *mut c_void;
    fn dlclose(handle: *mut c_void) -> i32;
    fn dlerror() -> *const c_char;
}

#[repr(C)]
#[derive(Default)]
struct State {
    preset: u32,
    voices: u32,
    note_ons: u32,
    note_offs: u32,
    blocks: u32,
    faults: u32,
    peak: f32,
}

#[derive(Clone, Copy)]
struct Api {
    create: unsafe extern "C" fn(f32) -> *mut c_void,
    destroy: unsafe extern "C" fn(*mut c_void),
    on: unsafe extern "C" fn(*mut c_void, u8, u8, u8),
    off: unsafe extern "C" fn(*mut c_void, u8, u8),
    notes_off: unsafe extern "C" fn(*mut c_void),
    panic: unsafe extern "C" fn(*mut c_void),
    cc: unsafe extern "C" fn(*mut c_void, u8, u8, u8),
    bend: unsafe extern "C" fn(*mut c_void, u8, i32),
    pressure: unsafe extern "C" fn(*mut c_void, u8, u8),
    load_factory: unsafe extern "C" fn(*mut c_void, i32) -> i32,
    probe: unsafe extern "C" fn(f32) -> f32,
    render: unsafe extern "C" fn(*mut c_void, *mut f32, *mut f32, f64, f64, i32) -> i32,
    render_input: unsafe extern "C" fn(
        *mut c_void,
        *const f32,
        *const f32,
        *mut f32,
        *mut f32,
        f64,
        f64,
        i32,
    ) -> i32,
    state: unsafe extern "C" fn(*mut c_void, *mut State) -> i32,
    error: unsafe extern "C" fn() -> *const c_char,
}

impl Api {
    // Keep the library loaded for the process lifetime: no callback/destructor
    // can race a dlclose. Initialization runs on the app thread.
    unsafe fn load() -> Result<Self, String> {
        let library = c"libhoneydew_surge_xt.so";
        #[cfg(not(target_os = "android"))]
        let library_path = std::env::var("HONEYDEW_SURGE_LIBRARY")
            .ok()
            .and_then(|s| CString::new(s).ok());
        #[cfg(not(target_os = "android"))]
        let library = library_path.as_deref().unwrap_or(library);
        let handle = unsafe { dlopen(library.as_ptr(), 2) };
        if handle.is_null() {
            let raw = unsafe { dlerror() };
            let detail = if raw.is_null() {
                "unknown loader error".into()
            } else {
                unsafe { CStr::from_ptr(raw) }
                    .to_string_lossy()
                    .into_owned()
            };
            return Err(fail(
                -1,
                format!("A: cannot load original Surge XT: {detail}"),
            ));
        }
        unsafe fn symbol<T: Copy>(handle: *mut c_void, name: &CStr) -> Result<T, String> {
            let raw = unsafe { dlsym(handle, name.as_ptr()) };
            if raw.is_null() {
                return Err(format!(
                    "A: missing Surge native symbol {}",
                    name.to_string_lossy()
                ));
            }
            Ok(unsafe { std::mem::transmute_copy::<*mut c_void, T>(&raw) })
        }
        let result = (|| {
            let version: unsafe extern "C" fn() -> u32 =
                unsafe { symbol(handle, c"honeydew_surge_abi_version")? };
            if unsafe { version() } != ABI {
                return Err("A: incompatible Surge XT native ABI".into());
            }
            #[cfg(not(target_os = "android"))]
            {
                let paths: unsafe extern "C" fn(*const c_char, *const c_char) -> i32 =
                    unsafe { symbol(handle, c"honeydew_surge_set_data_path")? };
                let factory = CString::new(
                    std::env::var("HONEYDEW_SURGE_DATA_PATH")
                        .map_err(|_| "B: set HONEYDEW_SURGE_DATA_PATH for native audio QA")?,
                )
                .map_err(|e| e.to_string())?;
                let user = CString::new(
                    std::env::var("HONEYDEW_SURGE_USER_PATH")
                        .map_err(|_| "B: set HONEYDEW_SURGE_USER_PATH for native audio QA")?,
                )
                .map_err(|e| e.to_string())?;
                if unsafe { paths(factory.as_ptr(), user.as_ptr()) } == 0 {
                    return Err("B: Surge XT factory or private user directory is invalid".into());
                }
            }
            Ok(Self {
                create: unsafe { symbol(handle, c"honeydew_surge_create")? },
                destroy: unsafe { symbol(handle, c"honeydew_surge_destroy")? },
                on: unsafe { symbol(handle, c"honeydew_surge_note_on")? },
                off: unsafe { symbol(handle, c"honeydew_surge_note_off")? },
                notes_off: unsafe { symbol(handle, c"honeydew_surge_all_notes_off")? },
                panic: unsafe { symbol(handle, c"honeydew_surge_panic")? },
                cc: unsafe { symbol(handle, c"honeydew_surge_cc")? },
                bend: unsafe { symbol(handle, c"honeydew_surge_pitch_bend")? },
                pressure: unsafe { symbol(handle, c"honeydew_surge_aftertouch")? },
                load_factory: unsafe { symbol(handle, c"honeydew_surge_load_factory")? },
                probe: unsafe { symbol(handle, c"honeydew_surge_probe")? },
                render: unsafe { symbol(handle, c"honeydew_surge_render32")? },
                render_input: unsafe { symbol(handle, c"honeydew_surge_render32_with_input")? },
                state: unsafe { symbol(handle, c"honeydew_surge_get_state")? },
                error: unsafe { symbol(handle, c"honeydew_surge_last_error")? },
            })
        })();
        if let Err(reason) = result {
            unsafe { dlclose(handle) };
            return Err(fail(-2, reason));
        }
        reached(1);
        result
    }
    fn detail(self, context: &str) -> String {
        let raw = unsafe { (self.error)() };
        let detail = if raw.is_null() {
            "unknown native error".into()
        } else {
            unsafe { CStr::from_ptr(raw) }.to_string_lossy()
        };
        format!("{context}: {detail}")
    }
}

static API: OnceLock<Result<Api, String>> = OnceLock::new();

pub struct SurgeXt {
    api: Api,
    instance: *mut c_void,
    left: [f32; BLOCK],
    right: [f32; BLOCK],
    input_left: [f32; BLOCK],
    input_right: [f32; BLOCK],
    input_active: bool,
    position: usize,
    analysis: [f32; 7],
}

// State has exclusive ownership, moving between app and audio threads.
unsafe impl Send for SurgeXt {}

impl SurgeXt {
    pub fn new(sample_rate: f32) -> Result<Self, String> {
        Self::new_with_factory_preset(sample_rate, 0)
    }
    pub fn new_with_factory_preset(sample_rate: f32, preset: usize) -> Result<Self, String> {
        if preset > FACTORY_PRESET_COUNT {
            return Err(fail(
                -4,
                "C: Surge XT factory preset index out of range".into(),
            ));
        }
        let api = *API
            .get_or_init(|| unsafe { Api::load() })
            .as_ref()
            .map_err(Clone::clone)?;
        // Sequential construction limits peak RAM on modest Android devices.
        // An Init Saw failure is a construction error, never an empty synth.
        static PROBE: OnceLock<()> = OnceLock::new();
        if PROBE.get().is_none() {
            let peak = unsafe { (api.probe)(sample_rate) };
            if !peak.is_finite() || peak <= 0.00001 {
                return Err(fail(
                    -5,
                    api.detail(&format!("E: Init Saw PCM probe failed (peak {peak})")),
                ));
            }
            let _ = PROBE.set(());
        }
        let instance = unsafe { (api.create)(sample_rate) };
        if instance.is_null() {
            return Err(fail(
                -3,
                api.detail("B: original Surge XT initialization failed"),
            ));
        }
        if preset != 0 && unsafe { (api.load_factory)(instance, preset as i32) } == 0 {
            let reason = api.detail("C: selected original factory FXP failed to load");
            unsafe { (api.destroy)(instance) };
            return Err(fail(-4, reason));
        }
        DIAGNOSTIC.store(3, Ordering::Relaxed);
        ERROR.lock().unwrap_or_else(|p| p.into_inner()).clear();
        let mut synth = Self {
            api,
            instance,
            left: [0.0; BLOCK],
            right: [0.0; BLOCK],
            input_left: [0.0; BLOCK],
            input_right: [0.0; BLOCK],
            input_active: false,
            position: BLOCK,
            analysis: [0.0; 7],
        };
        synth.refresh_analysis();
        Ok(synth)
    }
    fn refresh_analysis(&mut self) {
        let mut state = State::default();
        if unsafe { (self.api.state)(self.instance, &mut state) } != 0 {
            self.analysis = [
                state.preset as f32,
                state.voices as f32,
                state.note_ons as f32,
                state.note_offs as f32,
                state.blocks as f32,
                state.peak,
                state.faults as f32,
            ];
        }
    }
    fn midi_on(&mut self, channel: u8, key: u8, velocity: f32) {
        if !velocity.is_finite() || key > 127 || channel > 15 {
            return;
        }
        if velocity <= 0.0 {
            self.midi_off(channel, key);
            return;
        }
        let velocity = (velocity.clamp(0.0, 1.0) * 127.0).round().max(1.0) as u8;
        unsafe { (self.api.on)(self.instance, channel, key, velocity) };
        reached(4);
    }
    fn midi_off(&mut self, channel: u8, key: u8) {
        if key <= 127 && channel <= 15 {
            unsafe { (self.api.off)(self.instance, channel, key) };
        }
    }
}

impl Drop for SurgeXt {
    fn drop(&mut self) {
        unsafe { (self.api.destroy)(self.instance) };
    }
}

impl Instrument for SurgeXt {
    fn set_parameter(&mut self, _index: usize, _value: f32) {
        // The FXP owns all DSP parameters, macros and gains. Legacy Honeydew
        // defaults never overwrite them. p33 selects the patch at construction.
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
    fn takes_pedal(&self) -> bool {
        true
    }
    fn analysis(&self) -> &[f32] {
        &self.analysis
    }
    fn all_notes_off(&mut self) {
        unsafe { (self.api.notes_off)(self.instance) };
    }
    fn reset(&mut self) {
        unsafe { (self.api.panic)(self.instance) };
        self.left.fill(0.0);
        self.right.fill(0.0);
        self.input_left.fill(0.0);
        self.input_right.fill(0.0);
        self.input_active = false;
        self.position = BLOCK;
    }
    fn set_controller(&mut self, index: usize, value: f32) {
        self.set_midi_controller(0, index, value);
    }
    fn set_midi_controller(&mut self, channel: u8, index: usize, value: f32) {
        if !value.is_finite() || channel > 15 {
            return;
        }
        let value = value.clamp(0.0, 1.0);
        unsafe {
            match index {
                4 => (self.api.bend)(self.instance, channel, (value * 16383.0).round() as i32),
                5 => (self.api.pressure)(self.instance, channel, (value * 127.0).round() as u8),
                7..=134 => (self.api.cc)(
                    self.instance,
                    channel,
                    (index - 7) as u8,
                    (value * 127.0).round() as u8,
                ),
                _ => {}
            }
        }
    }
    fn process(&mut self, left: &mut [f32], right: &mut [f32], context: ProcessContext) {
        self.process_buffers(None, left, right, context);
    }
    fn process_with_input(
        &mut self,
        input_left: &[f32],
        input_right: &[f32],
        left: &mut [f32],
        right: &mut [f32],
        context: ProcessContext,
    ) {
        self.process_buffers(Some((input_left, input_right)), left, right, context);
    }
}

impl SurgeXt {
    fn process_buffers(
        &mut self,
        input: Option<(&[f32], &[f32])>,
        left: &mut [f32],
        right: &mut [f32],
        context: ProcessContext,
    ) {
        let rate = f64::from(context.sample_rate).max(1.0);
        for (index, (left, right)) in left.iter_mut().zip(right.iter_mut()).enumerate() {
            if self.position == BLOCK {
                let offset = if context.playing { index as f64 } else { 0.0 };
                let ppq =
                    context.start_beat + offset * f64::from(context.tempo_bpm) / (rate * 60.0);
                let ok = unsafe {
                    if self.input_active {
                        (self.api.render_input)(
                            self.instance,
                            self.input_left.as_ptr(),
                            self.input_right.as_ptr(),
                            self.left.as_mut_ptr(),
                            self.right.as_mut_ptr(),
                            ppq,
                            f64::from(context.tempo_bpm),
                            i32::from(context.playing),
                        )
                    } else {
                        (self.api.render)(
                            self.instance,
                            self.left.as_mut_ptr(),
                            self.right.as_mut_ptr(),
                            ppq,
                            f64::from(context.tempo_bpm),
                            i32::from(context.playing),
                        )
                    }
                };
                if ok == 0 {
                    self.left.fill(0.0);
                    self.right.fill(0.0);
                    // Preserve the native thread-local error for the UI using
                    // a bounded atomic buffer: no RT allocation, locks or I/O.
                    let raw = unsafe { (self.api.error)() };
                    let mut len = 0;
                    if !raw.is_null() {
                        for target in &RENDER_ERROR {
                            let byte = unsafe { raw.add(len).read() } as u8;
                            if byte == 0 {
                                break;
                            }
                            target.store(byte, Ordering::Relaxed);
                            len += 1;
                        }
                    }
                    RENDER_ERROR_LEN.store(len, Ordering::Release);
                    DIAGNOSTIC.store(-6, Ordering::Relaxed);
                } else if self
                    .left
                    .iter()
                    .chain(self.right.iter())
                    .any(|v| v.abs() > 0.00001)
                {
                    reached(5);
                }
                // Silence is legitimate for releases, slow attacks, key zones
                // and input-driven templates. It is not a loader failure.
                self.position = 0;
            }
            // Input is collected across arbitrary host/event slices. The
            // complete preceding 32-frame block feeds Surge's original input
            // upsampler; retain its tail once input has started.
            let (il, ir) = input.map_or((0.0, 0.0), |(il, ir)| {
                (
                    il.get(index).copied().unwrap_or(0.0),
                    ir.get(index).copied().unwrap_or(0.0),
                )
            });
            self.input_left[self.position] = if il.is_finite() { il } else { 0.0 };
            self.input_right[self.position] = if ir.is_finite() { ir } else { 0.0 };
            self.input_active |= il != 0.0 || ir != 0.0;
            *left += self.left[self.position];
            *right += self.right[self.position];
            self.position += 1;
        }
        self.refresh_analysis();
    }
}
