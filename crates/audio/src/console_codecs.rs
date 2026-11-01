//! Real console ADPCM block codecs, used by Console Color's optional native-codec mode.
//! These implement the actual on-wire BRR / SPU ADPCM / 9-byte N64 VADPCM frame layouts.
//! Scope: an independent, fixed-codebook, per-channel PCM encode/decode pass. This is not
//! a complete S-DSP/SPU/RSP, instrument sampler, trained N64 codebook, or DAC emulation.
//! All storage is fixed-size; the audio callback never allocates.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CodecKind { Brr, Vag, Vadpcm }

impl CodecKind {
    const fn samples(self) -> usize {
        match self { Self::Vag => 28, Self::Brr | Self::Vadpcm => 16 }
    }
}

fn signed_nibble(n: u8) -> i32 {
    let n = (n & 15) as i32;
    if n >= 8 { n - 16 } else { n }
}

fn pcm(v: i32) -> i16 { v.clamp(i16::MIN as i32, i16::MAX as i32) as i16 }

fn float_pcm(v: f32) -> i16 {
    (v.clamp(-1.0, 1.0) * 32767.0).round() as i16
}

// BRR: 9 bytes / 16 samples, 4-bit residuals, shift and 4 prediction filters.
// Histories are kept in the S-DSP's signed 15-bit internal format. Unlike the
// older coloration approximation, arithmetic shifts and 15-bit wrap match
// hardware. Pitch-dependent Gaussian voice interpolation is a separate stage.
fn brr_predict(filter: usize, h: [i32; 2]) -> i32 {
    let a = h[0];
    let b = h[1];
    match filter {
        1 => a + ((-a) >> 4),
        2 => (a << 1) + ((-3 * a) >> 5) - b + (b >> 4),
        3 => (a << 1) + ((-13 * a) >> 6) - b + ((3 * b) >> 4),
        _ => 0,
    }
}

fn brr_sample(nibble: i32, shift: u8, filter: usize, hist: &mut [i32; 2]) -> i16 {
    let residual = if shift > 12 {
        (nibble >> 3) << 11
    } else {
        (nibble << shift) >> 1
    };
    // The S-DSP wraps the 16-bit intermediate into a signed 15-bit value.
    let intermediate = (residual as i16).wrapping_add(brr_predict(filter, *hist) as i16);
    let internal = ((intermediate << 1) >> 1) as i32;
    hist[1] = hist[0];
    hist[0] = internal;
    (internal << 1) as i16
}

pub fn decode_brr(block: &[u8; 9], hist: &mut [i32; 2]) -> [i16; 16] {
    let filter = ((block[0] >> 2) & 3) as usize;
    let shift = block[0] >> 4;
    let mut out = [0i16; 16];
    for (i, dst) in out.iter_mut().enumerate() {
        let byte = block[1 + i / 2];
        let n = if i & 1 == 0 { byte >> 4 } else { byte & 15 };
        *dst = brr_sample(signed_nibble(n), shift, filter, hist);
    }
    out
}

pub fn encode_brr(input: &[i16; 16], hist: [i32; 2]) -> [u8; 9] {
    let mut best = [0u8; 9];
    let mut best_error = f64::INFINITY;
    for filter in 0..4 {
        for shift in 0..=12 {
            let mut state = hist;
            let mut candidate = [0u8; 9];
            candidate[0] = ((shift << 4) | (filter << 2)) as u8;
            let step = (2.0f32).powi(shift - 1);
            let mut error = 0.0f64;
            for (i, &target) in input.iter().enumerate() {
                let prediction = brr_predict(filter, state);
                let q = ((target as f32 * 0.5 - prediction as f32) / step)
                    .round().clamp(-8.0, 7.0) as i32;
                let reconstructed = brr_sample(q, shift as u8, filter, &mut state);
                error += (target as f64 - reconstructed as f64).powi(2);
                let byte = &mut candidate[1 + i / 2];
                if i & 1 == 0 { *byte = (q as u8 & 15) << 4; }
                else { *byte |= q as u8 & 15; }
            }
            if error < best_error {
                best_error = error;
                best = candidate;
            }
        }
    }
    best
}

const PS1_A: [i32; 5] = [0, 60, 115, 98, 122];
const PS1_B: [i32; 5] = [0, 0, -52, -55, -60];

fn vag_predict(filter: usize, state: [i32; 2]) -> i32 {
    (state[0] * PS1_A[filter] + state[1] * PS1_B[filter] + 32) >> 6
}

// PS1 VAG/SPU ADPCM: 16 bytes / 28 samples, low-nibble FIRST.
pub fn decode_vag(block: &[u8; 16], hist: &mut [i32; 2]) -> [i16; 28] {
    let filter = ((block[0] >> 4) as usize).min(4);
    let shift = (block[0] & 15).min(12) as i32;
    let mut out = [0i16; 28];
    for (i, dst) in out.iter_mut().enumerate() {
        let byte = block[2 + i / 2];
        let n = if i & 1 == 0 { byte & 15 } else { byte >> 4 };
        let reconstructed = pcm((signed_nibble(n) << 12 >> shift) + vag_predict(filter, *hist));
        hist[1] = hist[0];
        hist[0] = reconstructed as i32;
        *dst = reconstructed;
    }
    out
}

pub fn encode_vag(input: &[i16; 28], hist: [i32; 2]) -> [u8; 16] {
    let mut best = [0u8; 16];
    let mut best_error = f64::INFINITY;
    for filter in 0..5 {
        for shift in 0..=12 {
            let mut state = hist;
            let mut candidate = [0u8; 16];
            candidate[0] = ((filter << 4) | shift) as u8;
            // Flags byte is zero: a middle block of a continuous stream.
            let scale = (1 << (12 - shift)) as f32;
            let mut error = 0.0f64;
            for (i, &target) in input.iter().enumerate() {
                let p = vag_predict(filter, state);
                let q = ((target as f32 - p as f32) / scale).round().clamp(-8.0, 7.0) as i32;
                let reconstructed = pcm(p + (q << (12 - shift))) as i32;
                state[1] = state[0];
                state[0] = reconstructed;
                error += (target as f64 - reconstructed as f64).powi(2);
                let byte = &mut candidate[2 + i / 2];
                if i & 1 == 0 { *byte = q as u8 & 15; }
                else { *byte |= (q as u8 & 15) << 4; }
            }
            if error < best_error {
                best_error = error;
                best = candidate;
            }
        }
    }
    best
}

// N64 VADPCM uses vectors of eight residuals and a Q11 prediction-codebook.
// Four fixed, stable order-2 predictor pairs; this is a valid codebook, but
// *not* a trained per-instrument codebook as used by Nintendo's tabledesign.
const N64_PAIRS: [[i32; 2]; 4] = [
    [0, 0], [1536, 0], [3072, -1229], [3584, -1638],
];

fn n64_matrix(predictor: usize) -> [[i32; 10]; 8] {
    let [a, b] = N64_PAIRS[predictor];
    let mut m = [[0; 10]; 8];
    for i in 0..8 {
        // Indexing previous rows is required to build the causal prediction matrix.
        // Iterate mutable coefficients to satisfy strict Clippy without changing DSP.
        let (past, future) = m.split_at_mut(i);
        let row = &mut future[0];
        for (j, coefficient) in row.iter_mut().enumerate() {
            let prev = if i == 0 {
                if j == 1 { 2048 } else { 0 }
            } else { past[i - 1][j] };
            let older = if i == 0 {
                if j == 0 { 2048 } else { 0 }
            } else if i == 1 {
                if j == 1 { 2048 } else { 0 }
            } else { past[i - 2][j] };
            *coefficient = (a * prev + b * older) >> 11;
        }
        row[2 + i] += 2048;
    }
    m
}

fn n64_dot(row: &[i32; 10], values: &[i32; 10]) -> i32 {
    let sum: i64 = row.iter().zip(values).map(|(&a, &b)| i64::from(a) * i64::from(b)).sum();
    (sum >> 11).clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32
}

/// The exact fixed codebook required to decode frames produced by encode_vadpcm.
/// Each predictor stores 2 vectors of 8 Q11 coefficients.
pub fn vadpcm_codebook() -> [[[i16; 8]; 2]; 4] {
    let mut codebook = [[[0i16; 8]; 2]; 4];
    for (p, entry) in codebook.iter_mut().enumerate() {
        let matrix = n64_matrix(p);
        for (i, row) in matrix.iter().enumerate() {
            entry[0][i] = pcm(row[0]);
            entry[1][i] = pcm(row[1]);
        }
    }
    codebook
}

pub fn decode_vadpcm(block: &[u8; 9], hist: &mut [i32; 2]) -> [i16; 16] {
    let predictor = (block[0] & 15) as usize;
    let mut out = [0i16; 16];
    if predictor >= N64_PAIRS.len() { return out; }
    let shift = (block[0] >> 4).min(12);
    let matrix = n64_matrix(predictor);
    let mut state = *hist;
    for half in 0..2 {
        let mut values = [0i32; 10];
        values[0] = state[1];
        values[1] = state[0];
        for i in 0..8 {
            let k = half * 8 + i;
            let byte = block[1 + k / 2];
            let nib = if k & 1 == 0 { byte >> 4 } else { byte & 15 };
            values[2 + i] = signed_nibble(nib) << shift;
        }
        for i in 0..8 {
            out[half * 8 + i] = pcm(n64_dot(&matrix[i], &values));
        }
        state = [out[half * 8 + 7] as i32, out[half * 8 + 6] as i32];
    }
    *hist = state;
    out
}

fn encode_n64_candidate(input: &[i16; 16], hist: [i32; 2], predictor: usize, shift: u8)
    -> ([u8; 9], f64)
{
    let matrix = n64_matrix(predictor);
    let mut state = hist;
    let mut block = [0u8; 9];
    block[0] = (shift << 4) | predictor as u8;
    let mut error = 0.0f64;
    for half in 0..2 {
        let mut values = [0i32; 10];
        values[0] = state[1];
        values[1] = state[0];
        let mut last = [0i32; 2];
        for i in 0..8 {
            let k = half * 8 + i;
            let prediction = n64_dot(&matrix[i], &values);
            let q = ((input[k] as f32 - prediction as f32) / ((1u32 << shift) as f32))
                .round().clamp(-8.0, 7.0) as i32;
            values[2 + i] = q << shift;
            let decoded = pcm(n64_dot(&matrix[i], &values)) as i32;
            last = [decoded, last[0]];
            error += (input[k] as f64 - decoded as f64).powi(2);
            let byte = &mut block[1 + k / 2];
            if k & 1 == 0 { *byte = (q as u8 & 15) << 4; }
            else { *byte |= q as u8 & 15; }
        }
        state = last;
    }
    (block, error)
}

pub fn encode_vadpcm(input: &[i16; 16], hist: [i32; 2]) -> [u8; 9] {
    let mut best = [0u8; 9];
    let mut min_error = f64::INFINITY;
    for predictor in 0..N64_PAIRS.len() {
        // Evaluate a subset of useful scales; no heap, no pre-trained tables.
        for shift in 0..=12 {
            let (block, error) = encode_n64_candidate(input, hist, predictor, shift);
            if error < min_error { min_error = error; best = block; }
        }
    }
    best
}

/// Block-delayed stereo encode/decode, at the console's internal sample clock.
/// All three formats keep separate left/right predictor histories.
pub struct StereoCodec {
    kind: CodecKind,
    input: [[i16; 28]; 2],
    output: [[i16; 28]; 2],
    write: usize,
    read: usize,
    hist: [[i32; 2]; 2],
}

impl StereoCodec {
    pub fn new(kind: CodecKind) -> Self {
        Self {
            kind, input: [[0; 28]; 2], output: [[0; 28]; 2],
            write: 0, read: kind.samples(), hist: [[0; 2]; 2],
        }
    }

    pub fn reset(&mut self, kind: CodecKind) {
        self.kind = kind;
        self.input = [[0; 28]; 2];
        self.output = [[0; 28]; 2];
        self.write = 0;
        self.read = kind.samples();
        self.hist = [[0; 2]; 2];
    }

    pub fn process(&mut self, left: f32, right: f32) -> [f32; 2] {
        let length = self.kind.samples();
        let mut result = [0.0; 2];
        if self.read < length {
            result = [
                self.output[0][self.read] as f32 / 32767.0,
                self.output[1][self.read] as f32 / 32767.0,
            ];
            self.read += 1;
        }
        self.input[0][self.write] = float_pcm(left);
        self.input[1][self.write] = float_pcm(right);
        self.write += 1;
        if self.write == length {
            for channel in 0..2 {
                match self.kind {
                    CodecKind::Brr => {
                        let mut input = [0i16; 16];
                        input.copy_from_slice(&self.input[channel][..16]);
                        let block = encode_brr(&input, self.hist[channel]);
                        let output = decode_brr(&block, &mut self.hist[channel]);
                        self.output[channel][..16].copy_from_slice(&output);
                    }
                    CodecKind::Vag => {
                        let block = encode_vag(&self.input[channel], self.hist[channel]);
                        self.output[channel] = decode_vag(&block, &mut self.hist[channel]);
                    }
                    CodecKind::Vadpcm => {
                        let mut input = [0i16; 16];
                        input.copy_from_slice(&self.input[channel][..16]);
                        let block = encode_vadpcm(&input, self.hist[channel]);
                        let output = decode_vadpcm(&block, &mut self.hist[channel]);
                        self.output[channel][..16].copy_from_slice(&output);
                    }
                }
            }
            self.write = 0;
            self.read = 0;
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn three_native_block_formats_and_zero_roundtrips() {
        let zero16 = [0i16; 16];
        let zero28 = [0i16; 28];
        let a = encode_brr(&zero16, [0; 2]);
        let b = encode_vag(&zero28, [0; 2]);
        let c = encode_vadpcm(&zero16, [0; 2]);
        assert_eq!(a.len(), 9);
        assert_eq!(b.len(), 16);
        assert_eq!(c.len(), 9);
        assert_eq!(decode_brr(&a, &mut [0; 2]), zero16);
        assert_eq!(decode_vag(&b, &mut [0; 2]), zero28);
        assert_eq!(decode_vadpcm(&c, &mut [0; 2]), zero16);
    }

    #[test]
    fn codecs_reconstruct_nonzero_pcm_and_preserve_stereo_isolation() {
        let mut a = [0i16; 16];
        let mut b = [0i16; 28];
        for (i, v) in a.iter_mut().enumerate() { *v = (((i as f32) * 0.39).sin() * 12_000.0) as i16; }
        for (i, v) in b.iter_mut().enumerate() { *v = (((i as f32) * 0.21).sin() * 12_000.0) as i16; }
        assert!(decode_brr(&encode_brr(&a, [0; 2]), &mut [0; 2]).iter().any(|&x| x != 0));
        assert!(decode_vag(&encode_vag(&b, [0; 2]), &mut [0; 2]).iter().any(|&x| x != 0));
        assert!(decode_vadpcm(&encode_vadpcm(&a, [0; 2]), &mut [0; 2]).iter().any(|&x| x != 0));
        for kind in [CodecKind::Brr, CodecKind::Vag, CodecKind::Vadpcm] {
            let mut stream = StereoCodec::new(kind);
            for _ in 0..(kind.samples() * 3) {
                let [l, r] = stream.process(0.4, 0.0);
                assert!(l.is_finite() && r.is_finite());
                assert_eq!(r, 0.0);
            }
        }
    }

    #[test]
    fn snes_brr_reference_shift_filter_and_wrap() {
        let mut hist = [0; 2];
        let mut block = [0u8; 9];
        block[1] = 0x7f;
        // Shift zero discards the lowest bit, high nibble is decoded first.
        let out = decode_brr(&block, &mut hist);
        assert_eq!(&out[..2], &[6, -2]);

        let mut hist = [0; 2];
        block[0] = 0x14; // range 1, predictor 1
        block[1] = 0x70;
        let out = decode_brr(&block, &mut hist);
        assert_eq!(&out[..2], &[14, 12]);

        // Values above the hardware's 15-bit range wrap, not clamp.
        let mut hist = [0; 2];
        block = [0; 9];
        block[0] = 0xc0; // range 12, predictor 0
        block[1] = 0x70;
        let out = decode_brr(&block, &mut hist);
        assert_eq!(out[0], 28_672);
        block[0] = 0xd0; // out-of-range shifts behave as special cases
        block[1] = 0x80;
        let out = decode_brr(&block, &mut [0; 2]);
        assert_eq!(out[0], -4_096);
    }

    #[test]
    fn valid_n64_codebook_and_frame_header() {
        let b = vadpcm_codebook();
        assert_eq!(b[0], [[0; 8]; 2]);
        let block = encode_vadpcm(&[4096; 16], [0; 2]);
        assert!((block[0] & 15) < 4);
        assert!((block[0] >> 4) <= 12);
    }

    #[test]
    fn reset_drops_queued_audio_and_predictor_histories() {
        let mut c = StereoCodec::new(CodecKind::Brr);
        for _ in 0..40 { c.process(0.5, 0.3); }
        c.reset(CodecKind::Vag);
        assert_eq!(c.process(0.0, 0.0), [0.0, 0.0]);
        assert_eq!(c.hist, [[0; 2]; 2]);
    }
}
