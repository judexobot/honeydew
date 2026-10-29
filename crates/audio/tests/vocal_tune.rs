use honeydew_audio::vocal_tune::{Parameters, Scale, VocalTune};
use std::f32::consts::TAU;
fn tone(hz: f32, rate: u32, seconds: f32, p: Parameters) -> (VocalTune, Vec<f32>) {
    let mut fx = VocalTune::new(rate).unwrap();
    fx.set_parameters(p);
    let mut audio = Vec::new();
    for n in 0..(rate as f32 * seconds) as usize {
        let input = 0.4 * (TAU * hz * n as f32 / rate as f32).sin();
        let (out, _) = fx.process_frame(input, input);
        if n > rate as usize {
            audio.push(out);
        }
    }
    (fx, audio)
}
fn peak(audio: &[f32], center: f32, rate: u32) -> f32 {
    let mut best = (0.0, 0.0);
    for step in -80..=80 {
        let hz = center + step as f32 * 0.25;
        let coefficient = 2.0 * (TAU * hz / rate as f32).cos();
        let (mut q1, mut q2) = (0.0, 0.0);
        for (i, &sample) in audio.iter().enumerate() {
            let window = 0.5 - 0.5 * (TAU * i as f32 / audio.len() as f32).cos();
            let q = sample * window + coefficient * q1 - q2;
            q2 = q1;
            q1 = q;
        }
        let power = q1 * q1 + q2 * q2 - coefficient * q1 * q2;
        if power > best.1 {
            best = (hz, power);
        }
    }
    best.0
}
#[test]
fn actual_output_reaches_a_single_custom_note_six_semitones_away() {
    for studio in [false, true] {
        let (fx, audio) = tone(
            369.994,
            48000,
            2.0,
            Parameters {
                scale: Scale::Custom,
                custom_notes: 1,
                retune_ms: 2.0,
                hard_tune: true,
                studio,
                ..Default::default()
            },
        );
        let target = fx.target_midi().unwrap();
        assert_eq!(target.rem_euclid(12), 0);
        let expected = 440.0 * 2.0_f32.powf((target as f32 - 69.0) / 12.0);
        let actual = peak(&audio, expected, 48000);
        assert!(
            (1200.0 * (actual / expected).log2()).abs() < 10.0,
            "studio={studio} target={expected} actual={actual}"
        );
    }
}
#[test]
fn shifted_output_is_a4_in_both_profiles() {
    for studio in [false, true] {
        let (_, audio) = tone(
            450.0,
            48000,
            2.0,
            Parameters {
                retune_ms: 2.0,
                hard_tune: true,
                studio,
                ..Default::default()
            },
        );
        assert!((peak(&audio, 440.0, 48000) - 440.0).abs() < 1.5);
    }
}
#[test]
fn dry_and_bypass_have_declared_exact_latency() {
    for rate in [8000, 44100, 48000, 96000, 192000] {
        for studio in [false, true] {
            let mut fx = VocalTune::new(rate).unwrap();
            fx.set_parameters(Parameters {
                wet: 0.0,
                studio,
                ..Default::default()
            });
            let delay = fx.latency_samples();
            for i in 0..delay + 10 {
                let x = if i == 0 { 1.0 } else { 0.0 };
                let (l, r) = fx.process_frame(x, -x);
                assert_eq!(l, if i == delay { 1.0 } else { 0.0 });
                assert_eq!(r, -l);
            }
        }
    }
}
#[test]
fn anti_phase_vocals_are_detected_and_noise_is_rejected() {
    let mut fx = VocalTune::new(48000).unwrap();
    for n in 0..16000 {
        let x = 0.3 * (TAU * 220.0 * n as f32 / 48000.0).sin();
        fx.process_frame(x, -x);
    }
    assert!((fx.detected_hz().unwrap() - 220.0).abs() < 1.0);
    let mut rng = 123456789u32;
    for _ in 0..24000 {
        rng ^= rng << 13;
        rng ^= rng >> 17;
        rng ^= rng << 5;
        let x = (rng as f32 / u32::MAX as f32 - 0.5) * 0.1;
        fx.process_frame(x, x);
    }
    assert!(fx.detected_hz().is_none());
}
#[test]
fn block_boundaries_do_not_change_audio() {
    let mut a = VocalTune::new(48000).unwrap();
    let mut b = VocalTune::new(48000).unwrap();
    let input: Vec<f32> = (0..12000)
        .map(|n| 0.3 * (TAU * 231.0 * n as f32 / 48000.0).sin())
        .collect();
    let expected: Vec<_> = input.iter().map(|&x| a.process_frame(x, x).0).collect();
    let mut l = input.clone();
    let mut r = input;
    for (left, right) in l.chunks_mut(137).zip(r.chunks_mut(137)) {
        b.process(left, right).unwrap();
    }
    assert_eq!(l, expected);
    assert_eq!(l, r);
}

fn formant_centroid(audio: &[f32], low: f32, high: f32) -> f32 {
    use rustfft::{FftPlanner, num_complex::Complex32};
    let n = audio.len().next_power_of_two();
    let mut spectrum = vec![Complex32::default(); n];
    for (i, &v) in audio.iter().enumerate() {
        spectrum[i].re = v * (0.5 - 0.5 * (TAU * i as f32 / audio.len() as f32).cos());
    }
    FftPlanner::new().plan_fft_forward(n).process(&mut spectrum);
    let (mut weighted, mut energy) = (0.0, 0.0);
    for (k, v) in spectrum[..n / 2].iter().enumerate() {
        let hz = k as f32 * 48000.0 / n as f32;
        if hz >= low && hz <= high {
            let p = v.norm_sqr();
            weighted += hz * p;
            energy += p;
        }
    }
    weighted / energy.max(1e-10)
}
#[test]
fn cepstral_formants_stay_in_place_and_manual_shift_moves_them() {
    let mut input = Vec::new();
    for n in 0..96000 {
        let time = n as f32 / 48000.0;
        let mut sample = 0.0;
        for harmonic in 1..=40 {
            let hz = harmonic as f32 * 150.0;
            let envelope = 0.025
                + (-0.5 * ((hz - 600.0) / 100.0).powi(2)).exp()
                + 0.7 * (-0.5 * ((hz - 1650.0) / 170.0).powi(2)).exp()
                + 0.3 * (-0.5 * ((hz - 2700.0) / 250.0).powi(2)).exp();
            sample += 0.06 * envelope * (TAU * hz * time).sin();
        }
        input.push(sample);
    }
    let original = formant_centroid(&input[48000..], 1250.0, 2100.0);
    let mut measured = Vec::new();
    for (preserve, shift) in [(false, 0.0), (true, 0.0), (true, 3.0)] {
        let mut fx = VocalTune::new(48000).unwrap();
        fx.set_parameters(Parameters {
            studio: true,
            hard_tune: true,
            retune_ms: 2.0,
            scale: Scale::Custom,
            custom_notes: 1,
            formant_preserve: preserve,
            formant_shift: shift,
            protect_sibilants: 0.0,
            ..Default::default()
        });
        let mut output = Vec::new();
        for (i, &x) in input.iter().enumerate() {
            let (y, _) = fx.process_frame(x, x);
            if i >= 48000 {
                output.push(y);
            }
        }
        measured.push(formant_centroid(&output, 1250.0, 2300.0));
    }
    println!("F2 input={original}, shifted={measured:?}");
    assert!(
        (measured[1] - original).abs() < (measured[0] - original).abs(),
        "preserving formants must improve their location"
    );
    assert!((measured[1] / original - 1.0).abs() < 0.08);
    assert!(
        measured[2] > measured[1] * 1.07,
        "manual formant shift must move the spectral envelope"
    );
}
#[test]
fn switching_profiles_back_and_forth_discards_stale_audio() {
    let mut fx = VocalTune::new(48000).unwrap();
    let mut sample = 0;
    for studio in [false, true, false, true] {
        fx.set_parameters(Parameters {
            studio,
            hard_tune: true,
            retune_ms: 2.0,
            ..Default::default()
        });
        let mut output = Vec::new();
        for i in 0..24000 {
            let x = 0.3 * (TAU * 450.0 * sample as f32 / 48000.0).sin();
            sample += 1;
            let (y, _) = fx.process_frame(x, x);
            assert!(y.is_finite() && y.abs() < 2.0);
            if i >= 12000 {
                output.push(y);
            }
        }
        assert!((peak(&output, 440.0, 48000) - 440.0).abs() < 2.0);
    }
}

// Synthetic harmonic excitation with moving vowel resonances. This checks tracking,
// not subjective quality or validation against a corpus of real singers.
fn vowel(phase: f32, hz: f32, amplitude: f32) -> f32 {
    let mut sample = 0.0;
    for harmonic in 1..=24 {
        let f = hz * harmonic as f32;
        let resonances = 0.3
            + 2.0 * (-0.5 * ((f - 650.0) / 160.0).powi(2)).exp()
            + (-0.5 * ((f - 1700.0) / 250.0).powi(2)).exp();
        sample += amplitude * resonances / harmonic as f32 * (TAU * phase * harmonic as f32).sin();
    }
    sample
}
#[test]
fn actual_upward_octave_transitions_do_not_remain_latched_to_the_old_note() {
    for studio in [false, true] {
        let mut fx = VocalTune::new(48000).unwrap();
        fx.set_parameters(Parameters {
            studio,
            hard_tune: true,
            ..Default::default()
        });
        let mut phase = 0.0;
        for hz in [110.0, 220.0, 440.0, 220.0, 110.0] {
            for _ in 0..14400 {
                phase = (phase + hz / 48000.0).fract();
                let x = vowel(phase, hz, 0.13);
                let (l, r) = fx.process_frame(x, x);
                assert!(l.is_finite() && r.is_finite());
            }
            let actual = fx.detected_hz().expect("voiced harmonic signal");
            let cents = 1200.0 * (actual / hz).log2();
            assert!(
                cents.abs() < 15.0,
                "studio={studio} {hz} Hz detected as {actual}, {cents} cents"
            );
        }
    }
}
#[test]
fn vibrato_portamento_and_amplitude_changes_keep_harmonic_tracking() {
    for rate in [44100, 48000, 96000] {
        let mut fx = VocalTune::new(rate).unwrap();
        let mut phase = 0.0;
        let (mut tracked, mut errors, mut octave_errors) = (0, 0, 0);
        for n in 0..rate {
            let t = n as f32 / rate as f32;
            let hz = 160.0 * 2.0f32.powf((2.0 * t + 0.22 * (TAU * 5.5 * t).sin()) / 12.0);
            phase = (phase + hz / rate as f32).fract();
            let amplitude = 0.015 + 0.12 * (TAU * 1.7 * t).sin().powi(2);
            let x = vowel(phase, hz, amplitude);
            let (l, r) = fx.process_frame(x, -x);
            assert!(l.is_finite() && r.is_finite() && l.abs() < 3.0 && r.abs() < 3.0);
            if t > 0.15 && n % (rate / 200) == 0 {
                if let Some(actual) = fx.detected_hz() {
                    // Detection observes the trailing window; 30 cents covers this moving test's delay.
                    let cents = (1200.0 * (actual / hz).log2()).abs();
                    errors += usize::from(cents > 30.0);
                    octave_errors += usize::from(cents > 600.0);
                    tracked += 1;
                }
            }
        }
        assert!(
            tracked > 150 && errors < tracked / 5 && octave_errors == 0,
            "{rate}: tracked={tracked}, errors={errors}, octave errors={octave_errors}"
        );
    }
}
