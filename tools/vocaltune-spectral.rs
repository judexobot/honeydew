#[path = "../crates/audio/src/vocal_tune.rs"]
mod vocal_tune;
use std::f64::consts::PI;
use vocal_tune::{Parameters, Scale, VocalTune};

fn magnitude(samples: &[f32], hz: f64, rate: f64) -> f64 {
    let coefficient = 2.0 * (2.0 * PI * hz / rate).cos();
    let (mut q1, mut q2) = (0.0_f64, 0.0_f64);
    for (i, &v) in samples.iter().enumerate() {
        let w = 0.5 - 0.5 * (2.0 * PI * i as f64 / samples.len() as f64).cos();
        let q0 = v as f64 * w + coefficient * q1 - q2;
        q2 = q1;
        q1 = q0;
    }
    q1 * q1 + q2 * q2 - coefficient * q1 * q2
}
#[test]
fn actual_output_is_tuned_closer_to_440() {
    let rate = 48_000.0_f64;
    let mut fx = VocalTune::new(rate as u32).unwrap();
    fx.set_parameters(Parameters { root: 0, scale: Scale::Chromatic,
        correction: 1.0, retune_ms: 2.0, wet: 1.0, ..Parameters::default() });
    let mut output = Vec::new();
    for i in 0..120_000 {
        let x = (2.0 * PI * 450.0 * i as f64 / rate).sin() as f32 * 0.5;
        let (y, _) = fx.process_frame(x, x);
        if i >= 72_000 { output.push(y); }
    }
    let mut strongest = (0_i32, -1.0_f64);
    for hz in 425..=460 {
        let magnitude = magnitude(&output, hz as f64, rate);
        if magnitude > strongest.1 { strongest = (hz, magnitude); }
    }
    println!("strongest output frequency: {} Hz", strongest.0);
    assert!((strongest.0 - 440).abs() <= 5,
        "expected pitch near 440 Hz, got {} Hz", strongest.0);
}

#[test]
fn supports_android_and_offline_render_sample_rates() {
    for rate in [8_000_u32, 16_000, 22_050, 44_100, 48_000, 96_000, 192_000] {
        let mut effect = VocalTune::new(rate).expect("supported playback rate");
        let (l, r) = effect.process_frame(0.5, -0.5);
        assert!(l.is_finite() && r.is_finite(), "rate: {rate}");
    }
    assert!(VocalTune::new(7_999).is_err());
}
