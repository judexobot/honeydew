//! Android USB MIDI, outside the real-time rendering thread.
use eframe::egui::Context;
use jni::objects::{JByteArray, JClass, JObjectArray, JString};
use std::sync::{
    Mutex,
    atomic::{AtomicU32, Ordering},
    mpsc::SyncSender,
};
static SEND: Mutex<Option<(SyncSender<crate::midi::device::Message>, Option<Context>)>> = Mutex::new(None);
static GENERATION: AtomicU32 = AtomicU32::new(0);
pub fn ports() -> Vec<String> {
    let mut result = Vec::new();
    let _ = super::with_activity(|env, activity| {
        let array = JObjectArray::from(
            env.call_method(activity, "midiPorts", "()[Ljava/lang/String;", &[])?
                .l()?,
        );
        for i in 0..env.get_array_length(&array)? {
            let name = jni::objects::JString::from(env.get_object_array_element(&array, i)?);
            result.push(env.get_string(&name)?.into());
        }
        Ok(())
    });
    result
}
pub struct Session(u32);
impl Drop for Session {
    fn drop(&mut self) {
        if GENERATION.load(Ordering::Acquire) == self.0 {
            let _ = super::with_activity(|env, activity| {
                env.call_method(activity, "closeMidiPorts", "()V", &[])
                    .map(drop)
            });
            if let Ok(mut send) = SEND.lock() {
                *send = None;
            }
        }
    }
}
pub fn open(
    choice: &str,
    tx: SyncSender<crate::midi::device::Message>,
    ctx: Option<Context>,
) -> Result<Session, String> {
    let generation = GENERATION.fetch_add(1, Ordering::AcqRel).wrapping_add(1);
    *SEND.lock().map_err(|_| "MIDI receiver unavailable")? = Some((tx, ctx));
    super::with_activity(|env, activity| {
        let choice = env.new_string(choice)?;
        env.call_method(
            activity,
            "openMidiPorts",
            "(Ljava/lang/String;)V",
            &[(&choice).into()],
        )
        .map(drop)
    })?;
    Ok(Session(generation))
}
#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_daijro_honeydew_HoneydewMidiBridge_onMidiMessage(
    mut env: jni::JNIEnv<'_>,
    _: JClass<'_>,
    device_name: JString<'_>,
    message: JByteArray<'_>,
    count: jni::sys::jint,
) {
    if !super::audio_active() || !(2..=3).contains(&count) {
        return;
    }
    let mut bytes = [0i8; 3];
    if env
        .get_byte_array_region(&message, 0, &mut bytes[..count as usize])
        .is_err()
    {
        return;
    }
    let bytes: Vec<u8> = bytes[..count as usize].iter().map(|b| *b as u8).collect();
    if crate::midi::MidiMsg::parse(&bytes).is_none() {
        return;
    }
    let name: String = match env.get_string(&device_name) {
        Ok(name) => name.into(),
        Err(_) => return,
    };
    if let Ok(send) = SEND.lock()
        && let Some((tx, ctx)) = &*send
        && tx.try_send((name, bytes)).is_ok()
        && let Some(ctx) = ctx
    {
        ctx.request_repaint();
    }
}
