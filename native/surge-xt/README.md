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
- Original Surge XT **factory wavetables** are packaged into Android assets and
  extracted once into the application's private storage. The native Surge engine
  scans the exact `.wt` and `.wav` files for stock oscillator resources.
- An Init Saw PCM self-test runs on first use and reports native failures through
  Android logcat (tag `HoneydewSurgeXT`). It is a diagnostic, not substituted audio.

The original engine can receive standard MIDI pitch bend, notes and CCs; the
integration retains internal compatibility data for existing Honeydew projects.

## Corresponding source, license and Android build

Original upstream: https://github.com/surge-synthesizer/surge
Pinned revision: `5a4828a33ac4b2e2b45671b34e9e0e3f0c614765`
Upstream license: GPL-3.0-or-later.

Set `ANDROID_NDK_HOME`, then run:
```sh
bash tools/android-surge-xt.sh
HONEYDEW_SURGE_XT=1 sh tools/android-dist.sh
```

The build verifies the ARM64 native ELF C ABI, real original FXP bank manifest,
factory wavetable assets and the APK's bundled libraries.

## Validation status

Compilation/packaging does not prove that audio output works on a real phone.
The next Android APK must still be checked using real notes and several factory
presets, in particular wavetable, FM and FX-based sounds. Failures should be
diagnosed from the `HoneydewSurgeXT` logcat tag, not hidden by a fake synth.
