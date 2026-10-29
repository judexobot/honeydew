# Ultimate Audio integration — working state

- Fork: judexobot/honeydew. Development branch: integration/ultimate-audio.
- Baseline: feature/gen-fm-vocaltune-pro, 8ae264c6ad4b788c33fd7c5512b62f9ceb011d24.
- Prior Android APK: https://github.com/judexobot/honeydew/actions/runs/40929480661
- General CI baseline: https://github.com/judexobot/honeydew/actions/runs/40929480809
- Fixed its VocalTune rustfmt failure; local workspace Clippy (all targets, -D warnings) passes after it.

Audit: VocalTune is monophonic anti-aliased YIN + STFT/cepstral formants, with measured synthetic tests and two latency profiles. Voice-corpus/phone validation remains pending. The DMG core is register/timer based and bandlimited; analog/zombie revision details remain approximate. Its Wave RAM editor works but needs clipboard/import/reset operations. Gen currently has 56 saved parameters, six musical FM voices, linear ADSR, approximate algorithm families and three PolyBLEP PSG tones; it is not yet a YM2612/SN76489 emulator. Gen's channel-aware methods are currently bypassed by the host's APU-only MIDI routing. Repeated notes can stack unintended voices, sustain/panic and output-control smoothing need coverage. New Gen controls must be appended; old projects must keep their existing model.

Next: extend MIDI routing/recording to Gen; evaluate licensed Nuked-OPN2 as an optional clocked core, measuring cost before enabling it; add real register-clocked PSG, meaningful UI editing and original presets. Audit license: Nuked-OPN2 LGPL-2.1-or-later is compatible with this GPL-3.0-or-later repository. No game ROM/BIOS/assets are required. Consolidate CI with the final artifact name Honeydew-VocalTune-GameBoy-Gen-Android-arm64-debug. Preserve main and the user's baseline branch.

Workspace: /workspace/honeydew-ultimate is isolated from the previous /workspace/honeydew worktree. Reuse /workspace/honeydew/target via CARGO_TARGET_DIR; Android SDK/NDK/JDK are in /workspace/.honeydew-build-support with /tmp symlinks. Keep large build tools off /tmp (tmpfs shares the 8 GiB RAM limit). Read /workspace/.honeydew-build-support/ultimate-clippy.log for the baseline check.

## Core work in progress

- MIDI routing/recorders and sequencer checkpoint rows now support 16 sources on Gen racks (4 retained on APU-only racks); Gen lane destinations are in two short menu columns.
- Optional Gen YM2612 model is being added alongside Legacy. The first 56 parameter indices are untouched; appended model/region/PSG controls plus 4 × 10 raw operator controls bring the DSP contract to 102.
- Nuked-OPN2 is vendored unmodified at 335747d78cb0abbc3b55b004e62dad9763140115. The host adapter snapshots/coalesces bounded bus writes, pairs shared FNUM latches, keeps key-off/on retriggers and applies optional host pan. C undefined-behavior sanitizer smoke passed. A measured factor-of-two FNUM error was fixed to the actual chip formula (21 − block).
- New pure Rust clocked Sega PSG has raw latch writes, 10-bit periods, original attenuation and noise divisors/tone-C feedback, plus bandlimited resampling. Register/noise tests pass.
- Native tests: four in crates/audio/tests/gen.rs cover actual pitch across sample rates/regions, silence/release/panic, distinct algorithms and finite input handling. Re-run the latest log; their first run found the FNUM issue above.
- Pending immediately: expose/match all 102 params in the registry and editor, add original presets, finish clipboard operations and VocalTune tracking tests, then run strict checks and consolidate Actions. Do not call the new model production-ready before engine + UI + Android pass together.

## Integration checkpoint (2026-10-29)

Local commits: be14707 (format/audit), 328b599 (optional YM2612/clocked PSG and 16-source host routing),
2d9d80c (106-parameter editor, algorithms, operator copy/init, patch A/B and 13 original presets),
67c7135 (bounded vocal octave tracking, DMG sustain, Wave RAM exchange, integration tests).
Remote Git data commits corresponding to the last three: 22d9878270b2f7452e162ccd03b08293ee3dbae5,
9cd8d6bac9dc583a366bada03537954f3476b9af, 3f17140b2932688917bd1d48d3c3632862dec8a7.
These are being published together with the consolidated workflow, to avoid unnecessary builds.

- Gen retains all first 56 parameters and the default Legacy sound model; new projects/presets can choose YM2612.
  There are now 106 parameters (four native AM enables appended at 102–105). Clocked PSG is also optional.
  Raw YM envelopes, rate scaling, levels, multipliers, detune, SSG-EG and LFO drive the vendored chip registers.
  Six FM voices and three PSG tones remain bounded. Host MIDI, piano roll and recorders now address 16 sources.
  Engine changes release voices with a short output crossfade. A/B slots and operator clipboard are editor-session
  tools; the selected sound is saved as normal project/preset parameters. PSG Arpeggio is a sound for sequenced
  arpeggios, not an automatic arpeggiator.
- New Gen core tests measured A4 within 4 cents at 8/44.1/48/96/192 kHz in PAL and NTSC, silence/release,
  channel-specific panic, distinct algorithms and actual raw envelope/level/AM effects. C adapter UBSan smoke passes.
  An initial standalone core benchmark took 0.405 CPU seconds for 3 seconds of chip audio on this host; use the
  expanded instruments_smoke example for the complete six-voice engine and native ARM64 measurements.
- DMG: CC123 now honors sustain; CC120 cancels it. Wave clipboard/reset and validated 32-nibble HEX exchange are
  functional. Wave RAM remains saved in projects/presets. Standalone wave file dialogs are not implemented.
- VocalTune: octave hysteresis now expires after five consistent detector hops (25 ms), allowing real upward
  octave changes that previously latched forever. New tests use synthetic harmonic vowels with vibrato,
  portamento and amplitude changes. No human vocal corpus or subjective listening validation is claimed.
- The formatting-only remote CI 40948920494 passed fmt and Clippy, then exposed five pre-existing Gen test
  fixture failures. Selector coverage and the add-channel table have been corrected; browser tests now scroll
  to rows beyond the viewport instead of assuming the longer instrument list fits. No checks were disabled.
- Ultimate workflow consolidates format, all-target Clippy, complete workspace library tests, instrument
  integration/persistence tests, DSP spectral tests, allocation checks, content-tool and wasm compilation,
  native ARM64 DSP/audio fixtures, and a signed/aligned debug APK. Android builds run only for [android]
  milestones or manual dispatch. Artifact: Honeydew-VocalTune-GameBoy-Gen-Android-arm64-debug.

Still verifying locally: full application library suite and the new interaction tests; strict Clippy; native
ARM64/wasm cross compilation; all presets through SongAudioEngine. Logs are in /workspace/.honeydew-build-support/.
The previous `ultimate-editor-check.log` has overlapping stale output from two processes; use new logs instead.
Remaining scope: recorded-audio manual pitch editing, real-phone latency/touch/audio testing, full DMG conformance
ROM harness and revision-specific analog/zombie behavior, console-level YM timers/CSM/channel-3 special mode,
sample/DAC editor, full hardware-reference comparisons. The optional Nuked engine materially improves operators
and algorithm fidelity, but the complete musical instrument is not claimed cycle-accurate or 100% faithful.
