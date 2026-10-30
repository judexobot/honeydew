# Native Surge XT in Honeydew (GPL-3.0-or-later)

This integration links the **original Surge XT C++ core**, `surge-common`, into
`libhoneydew_surge_xt.so`, instead of rewriting its oscillators, envelopes,
filters or effects in Rust. The Rust instrument uses a minimal C ABI to send
notes, MIDI controls and Surge macros, then sums the actual original Surge output.

Upstream: https://github.com/surge-synthesizer/surge
Pinned source revision: `5a4828a33ac4b2e2b45671b34e9e0e3f0c614765`
License: GPL-3.0-or-later. Keep the corresponding Surge XT source available.

## Android compilation

Run `tools/android-surge-xt.sh` from the Honeydew checkout (after setting
ANDROID_NDK_HOME). The script checks out a pinned Surge revision and all of
its upstream submodules, configures the original CMake project with JUCE plugin
wrappers disabled, and builds `libhoneydew_surge_xt.so`. It copies the
library into Honeydew's Android `jniLibs` alongside `libhoneydew.so`.
Then build Honeydew with `cargo ndk ... --features surge-xt`.

The bridge never falls back to a fake synthesis engine. If the actual native
Surge library is unavailable, this instrument cannot be constructed.

## Current scope

Original Surge XT synthesis core and internal initial patch, polyphonic note
on/off, MIDI pitch bend/CC, 8 macro controls, 24 additional real Surge XT engine controls (oscillator, mixer, filter, amp/filter envelopes and LFO), tempo-linked 32-frame stereo
rendering, and real-time add-to-rack audio. Factory preset browsing,
serialized Surge patches, full modulation-matrix editing and the complete
Surge XT editor are **not yet wired into the Honeydew UI**. The Android
build and native audio still require CI and on-device validation.
