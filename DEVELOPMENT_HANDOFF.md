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
