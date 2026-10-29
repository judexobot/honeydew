# VocalTune / Game Boy integration

Base: `feature/gameboy-dmg-apu` (`b74de197`). `feature/vocaltune-v03`
(`56610061`) is already an ancestor; it has no exclusive changes to merge.
Existing module IDs, parameter indices and instruments are preserved. Old 22-parameter
APU presets retain Layered routing. New note-lane MIDI destinations use an optional,
versioned `MCHN` chunk; existing note events keep their binary format.

VocalTune is a separate monophonic processor, with anti-aliased YIN detection,
phase-locked STFT shifting, cepstral envelope preservation and independent formant
shift. All four pages control the DSP. Custom notes include single-note scales.
Monitor/Studio use 1024/2048 frames at 48 kHz (21.3/42.7 ms processing delay,
plus device buffers). Acquisition of a new voiced input takes about 55–75 ms;
processing delay and tracking response are different measurements. Honeydew aligns
parallel mixer paths, instruments below the effect and the metronome, and removes
the added delay from offline exports. Bypass and dry mix retain processing delay.

Authentic uses one APU with the four hardware channels. MIDI channels 1–4 map to
Pulse 1, Pulse 2, Wave and Noise; the keyboard destination is selectable. Each notes
lane can choose its destination in the track menu. MIDI recording retains it.
Extended uses up to eight separate APUs with voice stealing, release and per-channel
pan. These are host features, not DMG hardware polyphony. Wave RAM is 32 editable
four-bit samples, stored and automatable like the other parameters. Editing it uses
DAC-off writes, so only the wave channel retriggers.

The core clocks timers at 4,194,304 Hz and the frame sequencer at 512 Hz. Regression
tests cover register masks, power/length retention, extra length clocks, duty-phase
retrigger, sweep-negate disable, frozen noise clocks, wave sample latch/access,
DAC polarity, stereo routing and alias rejection. Edge transitions use bandlimited
resampling. References: https://gbdev.io/pandocs/Audio.html and
https://gbdev.io/pandocs/Audio_details.html . No claim of complete DMG fidelity:
revision-specific envelope zombie behavior and analog DAC fades remain approximate;
Blargg/mooneye ROMs have not been run through a full CPU/bus emulator here.

Android adds microphone permission/capture and Android MIDI Manager USB input,
running status, fragmented messages and disconnect panic. Audio is silenced and
notes released on pause. Phone-specific round-trip latency, microphone routing,
USB compatibility and touch ergonomics still require testing on physical devices.
The headless UI tests cover every page, mobile pixel densities and Wave RAM drags.

The consolidated workflow checks Rust 2024 engine/UI, persistence, DSP spectra,
callback allocation, web compatibility and native ARM64 audio. `[android]` in an
integration commit (or workflow_dispatch) builds the SDK 36 / NDK 28 ARM64 debug APK,
verifies signature/alignment and uploads
`Honeydew-VocalTune-GameBoy-Android-arm64-debug`. Audio QA WAVs and CPU timings are
separate artifacts; shared-runner timings are not Android device latency measurements.
