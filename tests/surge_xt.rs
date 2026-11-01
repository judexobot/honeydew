#![cfg(all(feature = "surge-xt", any(target_os = "android", target_os = "linux")))]
//! Real native engine -> Rust adapter -> instrument module -> Honeydew mixer.
//! HONEYDEW_SURGE_LIBRARY points to the compiled production bridge, never a mock.
use honeydew::ui::main_menu::SongLibrary;
use honeydew::{
    audio_engine::SongAudioEngine,
    content, flm,
    song::{ChannelKind, Event, Song, SynthKind},
};
use honeydew_audio::{
    ProcessContext, device::AudioRenderer, instrument::Instrument, surge_xt::SurgeXt,
};
use std::path::Path;

const RATE: usize = 48_000;
const CATALOG: &str = include_str!("../src/content/presets/surge_xt_factory.tsv");

fn song() -> Song {
    let mut song = Song::default();
    song.tempo_bpm = 120.0;
    song.add_channel(ChannelKind::Synth(SynthKind::SurgeXt));
    song
}
fn index(path: &str) -> usize {
    CATALOG.lines().position(|p| p == path).unwrap() + 1
}
fn choose(song: &mut Song, path: &str) {
    let relative = path.strip_suffix(".fxp").unwrap();
    let file = Path::new(content::ROOT)
        .join("Presets/Surge XT")
        .join(format!("{relative}.flmpst"));
    let bytes = content::factory().read(&file).unwrap();
    let preset = flm::read_preset(&bytes, Some(57)).unwrap();
    song.channels[1].rack.modules[0].load_preset(preset);
}
fn render(engine: &mut SongAudioEngine, seconds: usize) -> f32 {
    let mut output = [0.0; 514]; // 257 stereo frames: does not divide 32.
    let mut peak = 0.0f32;
    for _ in 0..(RATE * seconds).div_ceil(257) {
        engine.render(&mut output, 2);
        assert!(output.iter().all(|v| v.is_finite()));
        peak = output.iter().fold(peak, |a, b| a.max(b.abs()));
    }
    peak
}
fn slab(engine: &SongAudioEngine) -> &[f32] {
    let at = (0..engine.meter_count())
        .find(|&i| {
            let key = engine.meter_key(i);
            key.channel == 1 && key.instance_id == 2
        })
        .expect("Surge must not disappear into an empty instrument lane");
    engine.analysis(at)
}

#[test]
fn every_browser_entry_keeps_its_category_exact_native_index_and_saved_patch() {
    let mut song = song();
    let mut categories = std::collections::BTreeSet::new();
    for (i, path) in CATALOG.lines().enumerate() {
        choose(&mut song, path);
        let module = &song.channels[1].rack.modules[0];
        assert_eq!(
            (module.params[33] * 641.0).round() as usize,
            i + 1,
            "{path}"
        );
        let (category, name) = path.rsplit_once('/').unwrap();
        categories.insert(category);
        assert_eq!(module.file.preset_name, name.strip_suffix(".fxp").unwrap());
        let saved = flm::read(&flm::write(&song).unwrap()).unwrap();
        assert_eq!(
            saved.channels[1].rack.modules[0].params[33],
            module.params[33]
        );
        assert_eq!(
            saved.channels[1].rack.modules[0].file.preset_name,
            module.file.preset_name
        );
    }
    assert_eq!(CATALOG.lines().count(), 641);
    assert_eq!(categories.len(), 17);
}

#[test]
fn original_init_saw_crosses_arbitrary_rust_buffers_without_dropping_pcm() {
    let mut synth = SurgeXt::new(RATE as f32).unwrap();
    let context = ProcessContext {
        sample_rate: RATE as f32,
        tempo_bpm: 120.0,
        start_beat: 0.0,
        playing: false,
    };
    // Periodic Honeydew legacy/default synchronization cannot alter the FXP.
    for i in 1..34 {
        synth.set_parameter(i, 0.0);
    }
    synth.set_controller(4, 0.5);
    synth.note_on(69, 0.9, 0.5);
    let mut peak = 0.0f32;
    let mut frames = 0;
    for _ in 0..20 {
        for size in [1, 7, 31, 32, 33, 63, 127, 511, 1024] {
            let (mut left, mut right) = (vec![0.0; size], vec![0.0; size]);
            synth.process(&mut left, &mut right, context);
            assert!(left.iter().chain(&right).all(|v| v.is_finite()));
            peak = left.iter().chain(&right).fold(peak, |a, b| a.max(b.abs()));
            frames += size;
        }
    }
    assert!(peak > 0.001, "Original Init Saw PCM must reach Rust");
    assert_eq!(synth.analysis()[2], 1.0);
    assert_eq!(synth.analysis()[4] as usize, frames.div_ceil(32));
    assert_eq!(synth.analysis()[6], 0.0);
}

#[test]
fn keyboard_transport_mixer_and_repeated_preset_changes_use_original_audio() {
    let mut song = song();
    let mut engine = SongAudioEngine::new(&song, RATE as f32).unwrap();
    engine.note_on(1, 60, 0.9);
    assert!(
        render(&mut engine, 1) > 0.001,
        "Stopped keyboard must sound through the master"
    );
    assert!(slab(&engine)[2] > 0.0 && slab(&engine)[4] > 0.0);
    engine.note_off(1, 60);
    render(&mut engine, 2);
    assert_eq!(
        slab(&engine)[1],
        0.0,
        "Native Note Off must release its voices"
    );
    // Several consecutive edits: the origin hash must not adopt the old sound.
    for path in [
        "Templates/Init Saw.fxp",
        "Templates/Init Sine.fxp",
        "Templates/Init Saw.fxp",
    ] {
        choose(&mut song, path);
        let mut next = SongAudioEngine::new(&song, RATE as f32).unwrap().adopting();
        next.take_over(&mut engine);
        next.note_on(1, 60, 0.9);
        assert!(
            render(&mut next, 1) > 0.001,
            "Preset change lost audio: {path}"
        );
        assert_eq!(slab(&next)[0] as usize, index(path));
        next.note_off(1, 60);
        engine = next;
    }
    // A saved Piano Roll and its chosen original patch cross the actual mixer.
    song.channels[1].lanes[0].add_clip(0, 4.0).events = vec![Event::Note {
        pos: 0,
        len_beats: 1.0,
        key: 60,
        velocity: 224,
        pan: 127,
        slide: false,
    }];
    let saved = flm::read(&flm::write(&song).unwrap()).unwrap();
    let mut sequenced = SongAudioEngine::new(&saved, RATE as f32).unwrap();
    sequenced.play();
    assert!(
        render(&mut sequenced, 1) > 0.001,
        "Saved Piano Roll must reach Android PCM path"
    );
    sequenced.stop();
    render(&mut sequenced, 2);
    assert_eq!(slab(&sequenced)[1], 0.0);
    sequenced.note_on(1, 64, 0.9);
    assert!(
        render(&mut sequenced, 1) > 0.001,
        "Keyboard must still play after Stop"
    );
}

#[test]
fn input_templates_receive_the_existing_rack_audio_in_the_real_engine() {
    let mut song = song();
    choose(&mut song, "Templates/Audio In Stereo Osc 1.fxp");
    let rack = &mut song.channels[1].rack;
    rack.add(39); // An existing 3x Osc supplies audio to the rack above Surge.
    rack.modules.swap(0, 1);
    let mut engine = SongAudioEngine::new(&song, RATE as f32).unwrap();
    engine.note_on(1, 60, 0.9);
    assert!(render(&mut engine, 1) > 0.001);
    assert!(
        slab(&engine)[5] > 0.00001,
        "Surge's own PCM must be nonzero, independently of the dry source in the mixer"
    );
    assert_eq!(slab(&engine)[6], 0.0);
}

#[test]
fn midi_source_16_and_sustain_reach_surge_with_channel_identity() {
    let song = song();
    let mut engine = SongAudioEngine::new(&song, RATE as f32).unwrap();
    engine.midi_note(1, 15, 60, Some(0.9));
    assert!(
        render(&mut engine, 1) > 0.001,
        "MIDI channel 16 was dropped"
    );
    engine.midi_controller(1, 15, 71, 1.0);
    engine.midi_note(1, 15, 60, None);
    assert!(render(&mut engine, 1) > 0.001);
    assert!(
        slab(&engine)[1] >= 1.0,
        "Surge's own sustain must hold the voice"
    );
    engine.midi_controller(1, 15, 71, 0.0);
    render(&mut engine, 2);
    assert_eq!(slab(&engine)[1], 0.0);
}
