# Ultimate Audio integration — working state

- Fork: judexobot/honeydew. Development branch: integration/ultimate-audio.
- Baseline: feature/gen-fm-vocaltune-pro, 8ae264c6ad4b788c33fd7c5512b62f9ceb011d24.
- Prior Android APK: https://github.com/judexobot/honeydew/actions/runs/40929480661
- General CI baseline: https://github.com/judexobot/honeydew/actions/runs/40929480809
- Fixed its VocalTune rustfmt failure; local workspace Clippy (all targets, -D warnings) passes after it.

Audit: VocalTune is monophonic anti-aliased YIN + STFT/cepstral formants, with measured synthetic tests and two latency profiles. Voice-corpus/phone validation remains pending. The DMG core is register/timer based and bandlimited; analog/zombie revision details remain approximate. Its Wave RAM editor works but needs clipboard/import/reset operations. Gen currently has 56 saved parameters, six musical FM voices, linear ADSR, approximate algorithm families and three PolyBLEP PSG tones; it is not yet a YM2612/SN76489 emulator. Gen's channel-aware methods are currently bypassed by the host's APU-only MIDI routing. Repeated notes can stack unintended voices, sustain/panic and output-control smoothing need coverage. New Gen controls must be appended; old projects must keep their existing model.

Next: extend MIDI routing/recording to Gen; evaluate licensed Nuked-OPN2 as an optional clocked core, measuring cost before enabling it; add real register-clocked PSG, meaningful UI editing and original presets. Audit license: Nuked-OPN2 LGPL-2.1-or-later is compatible with this GPL-3.0-or-later repository. No game ROM/BIOS/assets are required. Consolidate CI with the final artifact name Honeydew-VocalTune-GameBoy-Gen-Android-arm64-debug. Preserve main and the user's baseline branch.

Workspace: /workspace/honeydew-ultimate is isolated from the previous /workspace/honeydew worktree. Reuse /workspace/honeydew/target via CARGO_TARGET_DIR; Android SDK/NDK/JDK are in /workspace/.honeydew-build-support with /tmp symlinks. Keep large build tools off /tmp (tmpfs shares the 8 GiB RAM limit). Read /workspace/.honeydew-build-support/ultimate-clippy.log for the baseline check.
