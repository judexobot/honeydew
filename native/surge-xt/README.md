# Honeydew · Original Surge XT factory preset instrument

This is the **original Surge XT C++ synthesis engine**, linked from
`surge-common` into an Android ARM64 library. It does **not** recreate any
oscillator, filter, FX, envelope, LFO or patch sound in Rust.

## Product scope

Surge XT is a **factory-preset player** in Honeydew, with no custom synth
editor, no simulated controls and no dependency on Surge's desktop GUI.

- The built-in Honeydew preset browser lists **641 original Surge XT factory
  FXP patches** in their **17 original categories**, sorted alphabetically.
- Selecting a sound instantiates the original native Surge synth and loads its
  corresponding complete upstream FXP binary.
- Honeydew sequences actual MIDI notes to this synthesizer and mixes its original
  stereo PCM into the existing audio graph.
- A change of factory FXP builds a new engine outside the realtime audio thread.
- Original Surge XT **factory FXPs, wavetables, factory impulses and tuning data** are packaged into Android assets and
  extracted once into the application's private storage. The native Surge engine
  scans the exact `.wt` and `.wav` files for stock oscillator resources.
- A SHA-256 manifest verifies the resources before NativeActivity starts audio.
- Android-private user paths replace the Unix login-path assumption in
  sst-plugininfra; no process-wide HOME/XDG variables are changed.
- An Init Saw PCM self-test runs on first use and reports native failures through
  Android logcat (tag `HoneydewSurgeXT`). It is a diagnostic, not substituted audio.

The original engine can receive standard MIDI pitch bend, notes and CCs; the
integration retains internal compatibility data for existing Honeydew projects.
Its original sustain and channel pressure are supported, with all 16 MIDI
channels preserved. Input oscillators and vocoders receive the audio from the
rack above the instrument, through a bounded 32-frame input adapter. Those
patches require an audio source, just as they do on the desktop.

## Corresponding source, license and Android build

Original upstream: https://github.com/surge-synthesizer/surge
Pinned revision: `5a4828a33ac4b2e2b45671b34e9e0e3f0c614765`
Upstream license: GPL-3.0-or-later.

Set `ANDROID_NDK_HOME`, then run:
```sh
bash tools/android-surge-xt.sh
HONEYDEW_SURGE_XT=1 sh tools/android-dist.sh
```

The build verifies the ARM64 native ELF C ABI, all original FXPs and resource
hashes, the complete bank inside the actual APK library, shared C++ runtime,
signature and 16-KiB ZIP/ELF alignment.

## Validation status

Run the native audio tests on Linux (including native ARM64):
```sh
bash tools/surge-xt-test.sh
```
This compiles the same production bridge against the pinned original core.
It tests Android-style private paths in a child without HOME, Init Saw at
44.1/48/96 kHz, finite PCM, Note Off, polyphony, sustain, reset, all 641 patches,
original wavetable contents and the official desktop file loader. It renders
3–6 seconds per sound, extends initially silent cases by 16 seconds with more
keys, and supplies identical input PCM to the seven input-driven patches.
Every buffer has 32-frame boundary guards. Additional comparisons against two
fresh official instances investigate sample differences due to engine state.

The Rust integration tests use the shared library through dlopen, then exercise
the browser's exact index mapping and all 641 saved-project roundtrips, arbitrary
host buffer lengths, the actual Honeydew master mixer, stopped keyboard,
playing Piano Roll, successive preset changes, MIDI channel 16 and sustain.
Failures abort engine construction with a visible diagnostic; no empty native
instrument is substituted. Render diagnostics use fixed atomic storage, with
no logging, formatting, file access or locking in the Rust callback.

`target/surge-audio-qa` contains per-preset PCM statistics, reference-comparison
results, a summary and a WAV for each original category. GitHub Actions runs
these checks on x86-64 and native ARM64 before building the Android APK.

Native and mixer audio tests do not measure a physical Android speaker, audio
driver, touch latency, thermal behavior or Samsung Galaxy A15 performance.
Those require a real phone; APK compilation alone is never reported as proof
of physical audio output. See [AUDIT.md](AUDIT.md) for the reproduced silence.
