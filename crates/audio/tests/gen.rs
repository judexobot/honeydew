use honeydew_audio::{
    ProcessContext,
    r#gen::{EXTRA_START, EXTRA_STRIDE, Gen, MODEL},
    instrument::Instrument,
};
const CTX: ProcessContext = ProcessContext {
    sample_rate: 48000.0,
    tempo_bpm: 120.0,
    start_beat: 0.0,
    playing: false,
};
fn carrier(rate: f32) -> Gen {
    let mut g = Gen::new(rate);
    g.set_parameter(MODEL, 1.0);
    g.set_parameter(54, 0.5);
    g.set_parameter(1, 1.0);
    for op in 0..4 {
        let i = EXTRA_START + op * EXTRA_STRIDE;
        g.set_parameter(i, 1.0);
        g.set_parameter(i + 1, 0.0);
        g.set_parameter(i + 2, 0.0);
        g.set_parameter(i + 3, 0.0);
        g.set_parameter(i + 4, 1.0);
        g.set_parameter(i + 6, if op == 3 { 0.0 } else { 1.0 });
    }
    g
}
#[test]
fn ym2612_register_core_plays_a4_in_ntsc_and_pal_at_all_host_rates() {
    for rate in [8000, 44100, 48000, 96000, 192000] {
        for region in [0.0, 1.0] {
            let mut g = carrier(rate as f32);
            g.set_parameter(57, region);
            g.note_on(69, 1.0, 0.5);
            let (mut l, mut r) = (vec![0.0; rate / 2], vec![0.0; rate / 2]);
            g.process(&mut l, &mut r, CTX);
            assert!(l.iter().all(|x| x.is_finite()));
            let tail = &l[rate / 4..];
            let peak = tail.iter().copied().map(f32::abs).fold(0.0, f32::max);
            assert!(peak > 0.005, "{rate} region={region} peak={peak}");
            // DAC quantization adds tiny crossings at high host rates; measure the spectral fundamental.
            use rustfft::{FftPlanner, num_complex::Complex32};
            let n = tail.len().next_power_of_two();
            let mut spectrum = vec![Complex32::default(); n];
            for (i, &x) in tail.iter().enumerate() {
                let window =
                    0.5 - 0.5 * (std::f32::consts::TAU * i as f32 / tail.len() as f32).cos();
                spectrum[i].re = x * window;
            }
            FftPlanner::new().plan_fft_forward(n).process(&mut spectrum);
            let lo = (400.0 * n as f32 / rate as f32) as usize;
            let hi = (480.0 * n as f32 / rate as f32) as usize;
            let peak = (lo..=hi)
                .max_by(|&a, &b| spectrum[a].norm_sqr().total_cmp(&spectrum[b].norm_sqr()))
                .unwrap();
            let power = |i: usize| spectrum[i].norm_sqr().max(1e-20).ln();
            let offset = 0.5 * (power(peak - 1) - power(peak + 1))
                / (power(peak - 1) - 2.0 * power(peak) + power(peak + 1));
            let hz = (peak as f32 + offset) * rate as f32 / n as f32;
            let cents = 1200.0 * (hz / 440.0).log2();
            assert!(
                cents.abs() < 4.0,
                "{rate} region={region} hz={hz} error={cents}c"
            );
        }
    }
}
#[test]
fn native_silence_release_and_channel_panic_are_real_audio() {
    let mut g = carrier(48000.0);
    let (mut l, mut r) = ([0.0; 512], [0.0; 512]);
    g.process(&mut l, &mut r, CTX);
    assert!(l.iter().all(|x| *x == 0.0));
    g.note_on_midi(15, 60, 0.8, 0.5);
    g.note_on_midi(2, 64, 0.8, 0.5);
    g.process(&mut l, &mut r, CTX);
    g.set_midi_controller(15, 127, 0.0);
    g.process(&mut l, &mut r, CTX);
    assert!(g.analysis()[..6].iter().any(|x| *x > 0.5));
    g.note_off_midi(2, 64);
    for _ in 0..80 {
        l.fill(0.0);
        r.fill(0.0);
        g.process(&mut l, &mut r, CTX);
    }
    assert!(l.iter().all(|x| x.abs() < 1e-4));
}
#[test]
fn every_native_algorithm_has_distinct_audio_and_no_nan() {
    let mut hashes = std::collections::HashSet::new();
    for algorithm in 0..8 {
        let mut g = Gen::new(48000.0);
        g.set_parameter(MODEL, 1.0);
        g.set_parameter(54, 0.5);
        g.set_parameter(1, algorithm as f32 / 7.0);
        g.note_on(60, 0.8, 0.5);
        let (mut l, mut r) = (vec![0.0; 12000], vec![0.0; 12000]);
        g.process(&mut l, &mut r, CTX);
        assert!(l.iter().all(|x| x.is_finite()));
        assert!(l.iter().any(|x| x.abs() > 1e-4));
        let hash = l
            .iter()
            .fold(0u64, |a, x| a.wrapping_mul(31) ^ u64::from(x.to_bits()));
        hashes.insert(hash);
    }
    assert_eq!(hashes.len(), 8);
}
#[test]
fn invalid_notes_do_not_poison_the_output() {
    let mut g = Gen::new(48000.0);
    g.note_on(60, 1.0, f32::NAN);
    g.note_on(60, f32::INFINITY, 0.5);
    let (mut l, mut r) = ([0.0; 128], [0.0; 128]);
    g.process(&mut l, &mut r, CTX);
    assert!(l.iter().all(|x| x.is_finite()));
}

#[test]
fn native_raw_envelope_total_level_multiplier_and_am_change_sound() {
    fn render(overrides: &[(usize, f32)]) -> Vec<f32> {
        let mut instrument = carrier(48000.0);
        for &(p, value) in overrides {
            instrument.set_parameter(p, value);
        }
        instrument.note_on(69, 1.0, 0.5);
        let (mut left, mut right) = (vec![0.0; 12000], vec![0.0; 12000]);
        instrument.process(&mut left, &mut right, CTX);
        left
    }
    let peak = |audio: &[f32]| audio.iter().map(|x| x.abs()).fold(0.0, f32::max);
    let normal = render(&[]);
    let no_attack = render(&[(92, 0.0)]);
    let attenuated = render(&[(98, 0.5)]);
    let multiple = render(&[(101, 2.0 / 15.0)]);
    let lfo_off = render(&[(8, 1.0), (60, 8.0 / 8.0)]);
    let am = render(&[(8, 1.0), (60, 8.0 / 8.0), (105, 1.0)]);
    assert!(peak(&normal) > 0.05);
    assert!(
        peak(&no_attack) < 1e-4,
        "AR=0 must leave a fresh silent envelope silent"
    );
    assert!(peak(&attenuated) < peak(&normal) * 0.05);
    assert_ne!(normal, multiple);
    assert_ne!(lfo_off, am);
}
