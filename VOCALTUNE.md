# VocalTune 0.3

Independent experimental monophonic pitch correction, effect ID 52.
No code from Honeydew Pitcher or Pitcher 2 is used.
YIN note detection plus two overlapped variable delay taps.
Chromatic / major / minor scales, root, strength, retune and wet control.

Expected signal-path delay: 1536 frames (~32ms at 48 kHz), plus device latency. No formant preservation or cross-track latency compensation yet. Requires voice and performance tests.

The default GitHub CI runs on branch pushes. After passing CI, manually run the VocalTune Android debug workflow to build a debug-signed APK artifact without release-signing secrets. Never merge experimental code into main until tested.
