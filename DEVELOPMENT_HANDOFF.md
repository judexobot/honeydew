# Ultimate Audio integration — 2026-10-29

Fork only: judexobot/honeydew. Branch: integration/ultimate-audio. main and upstream are untouched.
Baseline: feature/gen-fm-vocaltune-pro, 8ae264c6ad4b788c33fd7c5512b62f9ceb011d24.
Previous successful Android APK: https://github.com/judexobot/honeydew/actions/runs/40929480661
Original CI failure: https://github.com/judexobot/honeydew/actions/runs/40929480809 (VocalTune rustfmt).

## Implemented

- IDs 53/54/55 and all prior parameter indices remain. Gen retains the Legacy default and all first 56 values;
  new fields are appended, for 106 total. Older presets acquire compatible defaults without switching engines.
- Gen: optional Nuked-OPN2 YM2612 model, pinned unmodified LGPL-2.1-or-later source at
  335747d78cb0abbc3b55b004e62dad9763140115. Separate bounded C adapter paces/coalesces bus writes, snapshots
  address/data pairs, pairs FNUM high/low latches, and retains retrigger edges. A low FNUM write commits its high
  latch even when its low byte is unchanged. YM2612 FNUM scaling corrected against measured audio pitch.
- Real chip AR/DR/SR/SL/RR, rate scaling, TL, detune, SSG-EG, multiplier, per-operator AM, LFO, algorithms and feedback.
  PAL/NTSC clocks; bandlimited host resampling, approximate external high-pass, additional host pan/velocity/glide.
- Independent clocked Sega PSG: latch registers, 10-bit periods, Sega zero-period behavior, 2-dB attenuation,
  16-bit white/periodic LFSR, clock divisors/tone-C feedback, bandlimited output. Modern PSG remains selectable.
- Bounded six FM/three PSG voice allocation; same-note retriggers reuse their source's voice. Sustain, bend and
  CC120/123 respect 16 MIDI sources. Host live chains, recorders, sequencer/checkpoint/seek rows and lane menus
  now route all 16 sources to Gen. APU-only racks retain four sources. Engine swaps fade the previous output.
- Gen editor: eight pages, mode-specific operator controls, eight selectable routing diagrams, actual live operator
  envelope trace/LFO phase/channel/output meters, operator copy/paste/init, patch A/B and init. A/B snapshots are
  temporary editor state; the selected sound saves in ordinary project/preset parameters.
- Thirteen original factory patches: FM Bass/Piano, Electric Piano, Bell, Metallic Percussion, Brass, Synth Lead,
  Pluck, Organ, Sega-style Pad, PSG Lead/Arpeggio and Noise Percussion. PSG Arpeggio is a patch for sequenced
  arpeggios, not an automatic arpeggiator. No game ROMs, BIOS, music or proprietary patch data.
- DMG: CC123 now retains sustain, CC120 cancels it. Touch Wave RAM editor gains copy/paste/reset and validated
  32-nibble HEX import/export. Wave state saves with projects/presets. Standalone wave file dialogs are pending.
- VocalTune: bounded octave hysteresis accepts persistent real octave changes after five detector hops (25 ms);
  the previous detector could latch indefinitely. Existing independent YIN/STFT/cepstral formant processing and
  33 parameters remain. New synthetic vowel tests cover octaves, vibrato, portamento and changing amplitude.
- Fixed CI's format issue, added missing Gen selector fixtures and menu row, and made browser tests scroll to
  entries beyond the viewport. Added preset-count coverage for the new bank. No checks were disabled.

## Verified so far

- Local Android ARM64 whole-app `cargo ndk ... check --lib --locked`: PASS.
- Local wasm32 whole workspace including C adapter: PASS.
- Native C adapter UBSan smoke: PASS.
- Local Gen DSP: 5 tests PASS (measured A4 within 4 cents in PAL/NTSC at 8/44.1/48/96/192 kHz, silence/release,
  channel panic, eight distinct algorithms, invalid inputs and actual raw envelope/level/multiplier/AM effects).
- Local VocalTune DSP: 9 tests PASS, including output cents, latency, synthetic formants and moving vowels.
- First full app library run: 4354 passed, 5 stale fixtures failed, 2 baseline ignored. All five fixtures are now
  corrected; fresh targeted checks and the complete Actions suite are pending. Existing panel reference/bounds,
  density, wave drawing and parameter/preset roundtrip checks passed in that run.
- Strict Clippy found only new literal precision/helper arity/test-style issues, now corrected. Final rerun pending.
- First consolidated run: https://github.com/judexobot/honeydew/actions/runs/40952307441
  Validation found one remaining rustfmt line in host MIDI bookkeeping; corrected locally. ARM audio job PASSED:
  20 DMG + 8 Gen unit tests, 5 Gen + 9 VocalTune + 2 spectral integration tests, both allocation checks and WAV smoke.
  ARM QA artifact: https://github.com/judexobot/honeydew/actions/runs/40952307441/artifacts/12932187107
- ARM64 runner measurements (3 seconds audio, not phone measurements): VocalTune monitor 0.088571 s / studio
  0.100280 s; DMG Authentic 0.032928 s / Extended 0.032962 s (single-note workload); six-voice Gen Legacy
  0.090188 s / YM2612 + clocked PSG 0.261811 s. VocalTune declared DSP delay 1024/2048 samples at 48 kHz.

## Current verification and next steps

Local integration suite tests all 13 patches through SongAudioEngine, old 56-param presets, MIDI sources 2/16,
independent sustain/panic, piano roll and save/seek/release. New UI interaction tests exercise operator copy,
patch A/B and Wave RAM tools. Finish strict Clippy, rerun failed fixtures/new tests, inspect the next Actions run.
The milestone commit containing [android] requests the final debug APK, gated on all validation and ARM tests.
Artifact name: Honeydew-VocalTune-GameBoy-Gen-Android-arm64-debug. It must contain libhoneydew.so, bundled resources,
debug signing and 16-KiB ELF/ZIP alignment; tools/android-dist.sh checks these without dropping instruments/assets.

Remaining advanced scope: real-singer corpus/listening tests, recorded-audio manual note editor, physical Android
latency/touch/audio validation, full DMG conformance ROM harness/revision-specific zombie/analog cases, standalone
wave files, console-level YM timers/CSM/channel-3 special-mode/DAC sample editing and hardware-reference comparisons.
The core is materially more faithful; the whole instrument is not claimed 100% or complete-console cycle accurate.

## Workspace / history

Use /workspace/honeydew-ultimate, not the older dirty /workspace/honeydew worktree.
Reuse CARGO_TARGET_DIR=/workspace/honeydew/target; CARGO_BUILD_JOBS=1 for large local root builds (2 CPU quota).
Native Linux needs PKG_CONFIG_PATH=/tmp/honeydew-native/lib/pkgconfig and BINDGEN_EXTRA_CLANG_ARGS=-I$(cc -print-file-name=include).
Android NDK/JDK and large target caches live under /workspace/.honeydew-build-support (symlinked from /tmp).
Logs are in that folder; ultimate-editor-check.log contains overlapping stale output, use newer logs.

Local commits be14707 / 328b599 / 2d9d80c / 67c7135 / 754f2db map to GitHub commits
734c246 / 22d9878 / 9cd8d6b / 3f17140 / 382995b. Connector commit authors produce different SHAs; trees were compared
exactly before publication. Current remote checkpoint before the final fixes: 382995be169d99c02b4a559efb92170a8a32eb12.
Publish via GitHub tree/commit/ref tools (no shell push credential). Branch updates use expected_sha leases.
Payload chunk helpers and commit/tree exports are in /workspace/.honeydew-build-support/publish.

Latest local verification: Android ARM64 and wasm checks pass; all 20 DMG tests and the Gen callback allocation
check pass. Host integration passed 10/11: source-16 MIDI/sustain/panic, piano-roll save/seek/release, legacy preset
migration and defaults passed. The bank test incorrectly compared PSG patches using only tone A; it now auditions
all three independent tone channels with a triad as well. The initial single-note peaks were -9.6 to -19.9 dBFS;
Organ's trim is increased 2.4 dB to align it with the bank. The revised bank test still checks every patch's single
note and now also verifies finite unclipped triads and real differences. Re-run this test before final delivery.
Next full run (commit d7c891b before the bank correction): https://github.com/judexobot/honeydew/actions/runs/40953487471

Bank correction recheck: PASS. All 13 original presets now pass single-note/triad sound, finite-output and distinct
three-channel audition checks. Organ single-note peak is -17.5 dBFS. All 62 targeted application tests PASS, including
new operator-copy/A/B/wave-button interactions, both Gen models at mobile densities, complete panel references,
all preset roundtrips and all five previously failing fixtures. Gen callback allocation check is zero calls.
Updated tools/content-build/Cargo.lock: it predated the existing FFT/Blip dependencies as well as the new C build;
its isolated locked build now resolves all three. Android packages the licenses and immutable corresponding-source
link alongside its resources. Next Actions should perform the full library gate and build the debug APK.
