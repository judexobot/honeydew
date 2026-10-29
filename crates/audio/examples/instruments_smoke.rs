//! Audible fixtures and processing timings on the runner's actual CPU, including native ARM64 CI.
use honeydew_audio::{
    ProcessContext,
    gameboy_instrument::GameBoyInstrument,
    instrument::Instrument,
    vocal_tune::{Parameters, VocalTune},
    wav_writer::WavWriter,
};
use std::{fs, path::Path, time::Instant};

fn write(path: &Path, left: &[f32], right: &[f32]) {
    assert!(left.iter().chain(right).all(|x| x.is_finite()));
    assert!(left.iter().any(|x| x.abs() > 0.001), "silent output");
    let interleaved: Vec<_> = left.iter().zip(right).flat_map(|(&l, &r)| [l, r]).collect();
    let mut wav = WavWriter::new(fs::File::create(path).unwrap(), 48_000, 2).unwrap();
    wav.write(&interleaved).unwrap();
    wav.finish().unwrap();
}
fn main() {
    let folder = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "target/instruments-audio-qa".into());
    let folder = Path::new(&folder);
    fs::create_dir_all(folder).unwrap();
    let frames = 48_000 * 3;
    let mut report = String::from(
        "profile,architecture,audio_seconds,processing_seconds,declared_latency_frames\n",
    );
    for studio in [false, true] {
        let mut tune = VocalTune::new(48_000).unwrap();
        tune.set_parameters(Parameters {
            studio,
            hard_tune: true,
            retune_ms: 2.0,
            flex_cents: 0.0,
            vibrato: 0.0,
            transition_ms: 0.0,
            ..Parameters::default()
        });
        let mut left = vec![0.0; frames];
        let mut right = left.clone();
        for (i, (l, r)) in left.iter_mut().zip(&mut right).enumerate() {
            let phase = std::f32::consts::TAU * 450.0 * i as f32 / 48_000.0;
            *l = 0.3 * phase.sin() + 0.08 * (phase * 2.0).sin() + 0.03 * (phase * 3.0).sin();
            *r = *l;
        }
        let start = Instant::now();
        tune.process(&mut left, &mut right).unwrap();
        let elapsed = start.elapsed().as_secs_f64();
        let name = if studio {
            "vocal-studio"
        } else {
            "vocal-monitor"
        };
        report.push_str(&format!(
            "{name},{},3,{elapsed:.6},{}\n",
            std::env::consts::ARCH,
            tune.latency_samples()
        ));
        write(
            &folder.join(format!("{name}-450Hz-to-A4.wav")),
            &left,
            &right,
        );
    }
    for extended in [false, true] {
        let mut apu = GameBoyInstrument::new(48_000.0);
        apu.set_parameter(22, f32::from(extended));
        apu.set_parameter(23, 0.0);
        let mut left = vec![0.0; frames];
        let mut right = left.clone();
        let start = Instant::now();
        for (step, (l, r)) in left
            .chunks_mut(12_000)
            .zip(right.chunks_mut(12_000))
            .enumerate()
        {
            if step > 0 {
                apu.note_off(48 + ((step - 1) % 8) as u8);
            }
            apu.note_on(48 + (step % 8) as u8, 0.8, 0.5);
            apu.process(
                l,
                r,
                ProcessContext {
                    sample_rate: 48_000.0,
                    tempo_bpm: 120.0,
                    start_beat: step as f64 * 0.5,
                    playing: true,
                },
            );
        }
        let elapsed = start.elapsed().as_secs_f64();
        let name = if extended {
            "apu-extended"
        } else {
            "apu-authentic"
        };
        report.push_str(&format!(
            "{name},{},3,{elapsed:.6},0\n",
            std::env::consts::ARCH
        ));
        write(&folder.join(format!("{name}.wav")), &left, &right);
    }
    fs::write(folder.join("timings.csv"), &report).unwrap();
    print!("{report}");
}
