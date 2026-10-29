//! Runtime microphone permission. Permission dialogs never run on the audio thread.
use jni::objects::JClass;
use std::sync::atomic::{AtomicU8, Ordering};
static PERMISSION: AtomicU8 = AtomicU8::new(0); // unknown, pending, granted, denied
pub fn ready() -> bool {
    let mut granted = false;
    let _ = super::with_activity(|env, activity| {
        granted = env
            .call_method(activity, "hasRecordingPermission", "()Z", &[])?
            .z()?;
        Ok(())
    });
    if granted {
        PERMISSION.store(2, Ordering::Release);
    } else if PERMISSION.load(Ordering::Acquire) == 2 {
        PERMISSION.store(0, Ordering::Release);
    }
    granted
}
pub fn request() {
    if PERMISSION
        .compare_exchange(0, 1, Ordering::AcqRel, Ordering::Acquire)
        .is_ok()
        && super::with_activity(|env, activity| {
            env.call_method(activity, "requestRecordingPermission", "()V", &[])
                .map(drop)
        })
        .is_err()
    {
        PERMISSION.store(3, Ordering::Release);
    }
}
pub fn denied() -> bool {
    PERMISSION.load(Ordering::Acquire) == 3
}
pub fn retry() {
    if !ready() {
        PERMISSION.store(0, Ordering::Release);
    }
}
#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_daijro_honeydew_HoneydewActivity_onRecordingPermission(
    _: jni::JNIEnv<'_>,
    _: JClass<'_>,
    granted: jni::sys::jboolean,
) {
    PERMISSION.store(if granted != 0 { 2 } else { 3 }, Ordering::Release);
}
