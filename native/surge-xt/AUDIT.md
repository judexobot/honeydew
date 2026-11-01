# Surge XT silence audit

Baseline Honeydew: `ed73050fdfd7def2fc530bb8a19b74f38329bebe`.
Original Surge revision: `5a4828a33ac4b2e2b45671b34e9e0e3f0c614765`.

The primary failure occurs before a voice is constructed. SurgeStorage asks
sst-plugininfra for local application and user paths. Its Linux homePath()
throws when HOME is absent, which is normal in an Android application process.
The old C constructor caught every exception and returned null without a reason;
generators.rs converted that failure to None and omitted the instrument.
An APK could therefore compile, display the instrument and browse every preset
while the audio graph contained no Surge generator.

The baseline library was compiled and exercised independently of Honeydew UI:

| Baseline process environment | Created engine | Init Saw PCM peak |
| --- | --- | --- |
| Desktop login HOME available | yes | 0.401855111 |
| Identical child with HOME omitted | no | -1 (probe failure) |

The repaired bridge, with the Android private-path adapter enabled and HOME
omitted, initialized successfully and produced finite Init Saw PCM (observed
peak 0.212088078). Peaks vary with phase/state; the important regression is
successful construction and nonzero, finite original PCM without login paths.

HoneydewActivity now extracts and verifies the pinned resources and supplies
factory and private user directories before starting NativeActivity's Rust
thread. The platform adapter uses those paths directly. It does not set HOME,
use shared-storage permissions or change synthesis code. Missing paths, missing
symbols, constructor exceptions, invalid FXPs and render failures retain their
concrete errors. A failed instrument aborts the replacement engine, allowing the
previous project audio engine to remain active.

A second Android-path failure was reproduced after the HOME adaptation: map the
real shared library, deny search access to its parent directory, and create an
engine with valid factory/private paths. Desktop portable-folder discovery still
threw `filesystem error: status: Permission denied [.../SurgeXTData]`.
Android storage initialization now bypasses portable/XDG/Documents discovery
entirely and uses the supplied factory directory and private application/user
folders. `surge-xt-path-test.py` combines a missing HOME with an inaccessible
mapped-library parent and requires real Init Saw PCM. No synthesis algorithms
are changed by this platform-path adaptation.

Other integration errors corrected during the audit:

- The C++ pitchBend() expects signed -8192..8191; the old ABI passed unsigned
  0..16383, so Honeydew's centered bend detuned the patch. The ABI now converts
  the standard MIDI representation explicitly.
- MIDI identities were reserved only for Gen and Game Boy. Surge now has all
  16 live/sequence/checkpoint source rows and receives its original channel
  pressure and sustain events.
- Host reset called the asynchronous eight-block allSoundOff fade, which could
  kill a newly pressed note. Host reset now invokes the original stopSound().
- Patch/control/resource preparation now completes before the first note.
  Loading and file scans run before the instrument enters the audio callback.
- A short silent interval incorrectly declared a synthesis failure, including
  after note release or during slow attacks. It is no longer an error criterion.
- Original Audio In and vocoder patches had no input bus. They now receive rack
  input through the original upsampler, input oscillators and vocoder.
- Factory resources now include exact FXP files, stock tables, factory impulses,
  tuning data and licenses. Both startup and APK checks validate their hashes.
- Read-only FXP memory was already copied in the baseline; that protection is
  retained. The bank generator additionally validates every complete chunk and
  its embedded wavetable sizes; the actual APK bank is compared byte-for-byte
  with the 641 original FXP chunks in browser order.

All 641 patches are decoded with the original loadRaw(), including both scenes,
modulation, effect and arbitrary patch data. Honeydew's legacy parameter defaults
are ignored for this preset player; they do not overwrite patch gains or macros.
The normal browser selects a stable normalized index, which is preserved in the
ordinary project and preset formats.

The automated evidence distinguishes original PCM from physical output. The
native test exercises all 641 patches and the official loader; seven require
external audio rather than notes alone. The Rust integration test follows the
actual master mixer with keyboard and sequenced notes, including save/reopen.
The Actions artifacts retain WAV/TSV/JSON evidence. No real Android phone is
attached to this workspace, so speaker output, Android driver behavior and
Galaxy A15 latency/thermal performance remain physical-device checks.

Local x86-64 results before Android packaging: 641/641 complete patches loaded
and generated finite nonzero PCM under their appropriate conditions (634 with
notes alone, seven with input). 629 matched the continuous reference comparison
below 1e-5 maximum sample error; one more matched with fresh instances. The
remaining 11 also varied between two independently created official instances.
The detailed values and classifications are retained in
`factory-audio.tsv` and `reference-variations.tsv` rather than treating those
legitimate variations as a loader error or claiming bit identity for all sounds.
All five shared-library/mixer integration tests passed. The unchanged-instrument
regression suites passed 4,402 application and 254 audio-library tests, with two
pre-existing application tests ignored.
