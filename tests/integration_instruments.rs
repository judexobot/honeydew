use honeydew::{
    audio_engine::SongAudioEngine,
    content, flm,
    params::module_spec,
    song::{ChannelKind, LaneKind, ModuleInstance, Song, SynthKind},
};
use honeydew_audio::{device::AudioRenderer, gameboy_instrument};
#[test]
fn new_parameters_match_the_audio_defaults() {
    assert_eq!(
        module_spec(53).unwrap().params.len(),
        honeydew_audio::vocal_tune::NUM_PARAMS
    );
    assert_eq!(
        ModuleInstance::new(53, 2).params,
        honeydew_audio::vocal_tune::DEFAULTS
    );
    assert_eq!(
        ModuleInstance::new(54, 2).params,
        gameboy_instrument::DEFAULTS
    );
}
#[test]
fn real_engine_keeps_same_pitch_on_two_midi_channels_independent() {
    let mut song = Song::default();
    song.add_channel(ChannelKind::Synth(SynthKind::GameBoyApu));
    let mut engine = SongAudioEngine::new(&song, 48000.0).unwrap();
    let mut audio = vec![0.0; 8192];
    engine.midi_note(1, 0, 60, Some(0.8));
    engine.midi_note(1, 1, 60, Some(0.8));
    engine.render(&mut audio, 2);
    let index = (0..engine.meter_count())
        .find(|&i| {
            let key = engine.meter_key(i);
            key.channel == 1 && key.instance_id == 2
        })
        .unwrap();
    assert_eq!(&engine.analysis(index)[..2], &[1.0, 1.0]);
    assert!(audio.iter().any(|x| x.abs() > 0.001));
    engine.midi_note(1, 0, 60, None);
    engine.render(&mut audio, 2);
    assert_eq!(&engine.analysis(index)[..2], &[0.0, 1.0]);
    engine.midi_note(1, 1, 60, None);
    engine.render(&mut audio, 2);
    assert_eq!(&engine.analysis(index)[..2], &[0.0, 0.0]);
}
#[test]
fn song_and_presets_keep_custom_wave_and_lane_destinations() {
    let mut song = Song::default();
    song.add_channel(ChannelKind::Synth(SynthKind::GameBoyApu));
    let lane = &mut song.channels[1].lanes[0];
    lane.set_midi_channel(Some(2));
    let module = &mut song.channels[1].rack.modules[0];
    module.params[26] = 1.0;
    for i in 0..32 {
        module.params[27 + i] = (i % 16) as f32 / 15.0;
    }
    let module = module.clone();
    let read = flm::read(&flm::write(&song).unwrap()).unwrap();
    assert_eq!(read.channels[1].lanes[0].midi_channel(), Some(2));
    assert_eq!(read.channels[1].rack.modules[0].params, module.params);
    let preset = flm::read_preset(&flm::write_preset(&module).unwrap(), Some(54)).unwrap();
    assert_eq!(preset.params, module.params);
}
#[test]
fn old_twenty_two_parameter_gameboy_preset_keeps_layered_routing() {
    let mut old = ModuleInstance::new(54, 2);
    old.params.truncate(22);
    old.params[13] = 0.6;
    let preset = flm::read_preset(&flm::write_preset(&old).unwrap(), Some(54)).unwrap();
    assert_eq!(preset.params[23], 0.0);
    assert_eq!(preset.params[13], 0.6);
    assert_eq!(preset.params.len(), gameboy_instrument::NUM_PARAMS);
}
#[test]
fn every_new_preset_is_loadable_and_the_gameboy_presets_make_audio() {
    for name in ["Natural", "Hard Tune", "Vibrato", "Gentle"] {
        let m = content::preset(53, name).unwrap();
        assert_eq!(
            flm::read_preset(&flm::write_preset(&m).unwrap(), Some(53))
                .unwrap()
                .params,
            m.params
        );
    }
    for name in [
        "DMG Pulse Lead",
        "DMG Hollow",
        "DMG Sweep",
        "DMG Triangle Bass",
        "DMG Saw",
        "DMG Noise Hat",
        "APU Poly Keys",
        "APU Stereo Pad",
    ] {
        let mut song = Song::default();
        song.add_channel(ChannelKind::Synth(SynthKind::GameBoyApu));
        song.channels[1].rack.modules[0].load_preset(content::preset(54, name).unwrap());
        let mut engine = SongAudioEngine::new(&song, 48000.0).unwrap();
        engine.note_on(1, 48, 0.8);
        let mut audio = vec![0.0; 12000];
        engine.render(&mut audio, 2);
        assert!(audio.iter().all(|x| x.is_finite()));
        assert!(audio.iter().any(|x| x.abs() > 1e-4), "{name} was silent");
    }
}
#[test]
fn piano_roll_lanes_share_one_apu_and_preserve_their_destinations() {
    use honeydew::song::Event;
    let mut song = Song::default();
    song.add_channel(ChannelKind::Synth(SynthKind::GameBoyApu));
    for midi in 0..2 {
        let lane = if midi == 0 {
            0
        } else {
            song.channels[1].add_lane(LaneKind::Notes)
        };
        let lane = &mut song.channels[1].lanes[lane];
        lane.set_midi_channel(Some(midi));
        let clip = lane.add_clip(0, 4.0);
        clip.events.push(Event::Note {
            pos: 0,
            len_beats: 1.0 + midi as f64,
            key: 60,
            velocity: 200,
            pan: 127,
            slide: false,
        });
    }
    let mut engine = SongAudioEngine::new(&song, 48000.0).unwrap();
    engine.play();
    let mut audio = vec![0.0; 60000];
    engine.render(&mut audio, 2);
    let index = (0..engine.meter_count())
        .find(|&i| {
            let key = engine.meter_key(i);
            key.channel == 1 && key.instance_id == 2
        })
        .unwrap();
    assert_eq!(&engine.analysis(index)[..2], &[0.0, 1.0]);
}

#[test]
fn seeking_between_same_pitch_on_distinct_apu_channels_releases_the_old_address() {
    use honeydew::song::{Event, PPQ};
    let mut song = Song::default();
    song.add_channel(ChannelKind::Synth(SynthKind::GameBoyApu));
    for midi in 0..2 {
        let lane = if midi == 0 {
            0
        } else {
            song.channels[1].add_lane(LaneKind::Notes)
        };
        let lane = &mut song.channels[1].lanes[lane];
        lane.set_midi_channel(Some(midi));
        lane.add_clip(0, 8.0).events.push(Event::Note {
            pos: u32::from(midi) * 2 * PPQ,
            len_beats: 1.5,
            key: 60,
            velocity: 220,
            pan: 127,
            slide: false,
        });
    }
    let mut engine = SongAudioEngine::new(&song, 48_000.0).unwrap();
    let mut output = [0.0; 256];
    engine.play();
    engine.render(&mut output, 2);
    // The channel meter has no analysis. Target the instrument's meter instead.
    let index = (0..engine.meter_count())
        .find(|&i| {
            let key = engine.meter_key(i);
            key.channel == 1 && key.instance_id == 2
        })
        .expect("Game Boy instrument meter");
    assert_eq!(&engine.analysis(index)[..2], &[1.0, 0.0]);
    engine.seek(u64::from(PPQ) * 2);
    engine.render(&mut output, 2);
    assert_eq!(&engine.analysis(index)[..2], &[0.0, 1.0]);
    engine.seek(0);
    engine.render(&mut output, 2);
    assert_eq!(&engine.analysis(index)[..2], &[1.0, 0.0]);
}

#[test]
fn gen_defaults_and_legacy_presets_preserve_parameter_indices() {
    use honeydew_audio::r#gen::{DEFAULTS, MODEL, NUM_PARAMS};
    assert_eq!(ModuleInstance::new(55, 0).params, DEFAULTS);
    assert_eq!(module_spec(55).unwrap().params.len(), NUM_PARAMS);
    let mut old = ModuleInstance::new(55, 0);
    old.params.truncate(56);
    old.params[12] = 0.7;
    let restored = flm::read_preset(&flm::write_preset(&old).unwrap(), Some(55)).unwrap();
    assert_eq!(restored.params.len(), NUM_PARAMS);
    assert_eq!(&restored.params[..56], &old.params);
    assert_eq!(restored.params[MODEL], 0.0);
}

#[test]
fn all_thirteen_gen_presets_roundtrip_and_render_real_audio() {
    let mut hashes = std::collections::HashSet::new();
    for name in [
        "FM Bass",
        "FM Piano",
        "Electric Piano",
        "Bell",
        "Metallic Percussion",
        "Brass",
        "Synth Lead",
        "Pluck",
        "Organ",
        "Sega-style Pad",
        "PSG Lead",
        "PSG Arpeggio",
        "Noise Percussion",
    ] {
        let preset = content::preset(55, name).expect(name);
        assert_eq!(
            flm::read_preset(&flm::write_preset(&preset).unwrap(), Some(55))
                .unwrap()
                .params,
            preset.params
        );
        let mut song = Song::default();
        song.add_channel(ChannelKind::Synth(SynthKind::Gen));
        song.channels[1].rack.modules[0].load_preset(preset);
        let mut engine = SongAudioEngine::new(&song, 48000.0).unwrap();
        engine.note_on(1, 48, 0.8);
        let mut audio = vec![0.0; 24000];
        engine.render(&mut audio, 2);
        assert!(audio.iter().all(|x| x.is_finite()), "{name}");
        let peak = audio.iter().map(|x| x.abs()).fold(0.0, f32::max);
        println!("{name}: peak {:.1} dBFS", 20.0 * peak.max(1e-9).log10());
        assert!(peak > 0.01 && peak < 0.99, "{name}: peak {peak}");
        hashes.insert(audio.iter().fold(0u64, |hash, x| {
            hash.wrapping_mul(31) ^ u64::from(x.to_bits())
        }));
        engine.note_off(1, 48);
        audio.fill(0.0);
        engine.render(&mut audio, 2);
        assert!(audio.iter().all(|x| x.is_finite()));
    }
    assert_eq!(hashes.len(), 13);
}

#[test]
fn gen_midi_sources_two_and_sixteen_share_pitch_but_not_sustain_or_panic() {
    for model in [0.0, 1.0] {
        let mut song = Song::default();
        song.add_channel(ChannelKind::Synth(SynthKind::Gen));
        let module = &mut song.channels[1].rack.modules[0];
        module.params[56] = model;
        module.params[54] = 0.5;
        for op in 0..4 {
            module.params[18 + 8 * op] = 0.0;
            module.params[66 + 10 * op] = 1.0;
        }
        let mut engine = SongAudioEngine::new(&song, 48000.0).unwrap();
        let mut audio = vec![0.0; 12000];
        let index = (0..engine.meter_count())
            .find(|&i| {
                let k = engine.meter_key(i);
                k.channel == 1 && k.instance_id == 2
            })
            .unwrap();
        engine.midi_note(1, 1, 60, Some(0.8));
        engine.midi_note(1, 15, 60, Some(0.8));
        engine.render(&mut audio, 2);
        assert_eq!(&engine.analysis(index)[..2], &[1.0, 1.0]);
        engine.midi_controller(1, 15, 71, 1.0);
        engine.midi_note(1, 15, 60, None);
        engine.midi_controller(1, 1, 127, 0.0);
        engine.render(&mut audio, 2);
        assert_eq!(&engine.analysis(index)[..2], &[0.0, 1.0]);
        engine.midi_controller(1, 15, 130, 0.0);
        engine.render(&mut audio, 2);
        assert_eq!(engine.analysis(index)[1], 1.0);
        engine.midi_controller(1, 15, 71, 0.0);
        engine.render(&mut audio, 2);
        assert!(engine.analysis(index)[..6].iter().all(|v| *v == 0.0));
    }
}

#[test]
fn gen_piano_roll_channel_sixteen_survives_save_seek_and_release() {
    use honeydew::song::{Event, PPQ};
    let mut song = Song::default();
    song.add_channel(ChannelKind::Synth(SynthKind::Gen));
    for op in 0..4 {
        song.channels[1].rack.modules[0].params[18 + 8 * op] = 0.0;
    }
    for (index, midi) in [1, 15].into_iter().enumerate() {
        let lane = if index == 0 {
            0
        } else {
            song.channels[1].add_lane(LaneKind::Notes)
        };
        let lane = &mut song.channels[1].lanes[lane];
        lane.set_midi_channel(Some(midi));
        lane.add_clip(0, 8.0).events.push(Event::Note {
            pos: index as u32 * 2 * PPQ,
            len_beats: 1.5,
            key: 60,
            velocity: 220,
            pan: 127,
            slide: false,
        });
    }
    let song = flm::read(&flm::write(&song).unwrap()).unwrap();
    assert_eq!(song.channels[1].lanes[1].midi_channel(), Some(15));
    let mut engine = SongAudioEngine::new(&song, 48000.0).unwrap();
    let index = (0..engine.meter_count())
        .find(|&i| {
            let k = engine.meter_key(i);
            k.channel == 1 && k.instance_id == 2
        })
        .unwrap();
    let mut audio = vec![0.0; 48000];
    engine.play();
    engine.render(&mut audio[..1024], 2);
    assert_eq!(engine.analysis(index)[0], 1.0);
    engine.seek(u64::from(PPQ) * 2);
    engine.render(&mut audio, 2);
    assert_eq!(engine.analysis(index)[1], 1.0);
    engine.seek(u64::from(PPQ) * 4);
    engine.render(&mut audio, 2);
    assert!(engine.analysis(index)[..6].iter().all(|v| *v == 0.0));
}
