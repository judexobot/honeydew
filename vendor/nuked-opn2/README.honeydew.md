Nuked-OPN2 1.0.9 by Alexey Khokholov (Nuke.YKT), LGPL-2.1-or-later.
Upstream snapshot: https://github.com/nukeykt/Nuked-OPN2/commit/335747d78cb0abbc3b55b004e62dad9763140115
`ym3438.c` and `ym3438.h` are unmodified. `LICENSE` is upstream's license.
`honeydew.c` is a GPL-3.0-or-later host adapter, written for Honeydew. It bounds
storage and bus writes, coalesces parameters, preserves key retriggers, and
averages one 24-slot internal sample for bandlimited host-rate resampling.
`compat/string.h` only supplies a freestanding declaration for WebAssembly;
Rust compiler builtins implement memset. No game ROM, BIOS or sound assets
are required. Analog output and a complete console CPU/bus are not validated.
