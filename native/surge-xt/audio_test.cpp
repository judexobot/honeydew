// GPL-3.0-or-later. Tests compile the production bridge unchanged against the
// pinned surge-common, then compare with the official desktop FXP loader.
#include "surge_honeydew.cpp"
#include <fstream>
#include <iomanip>
#include <iostream>
#include <set>
#include <chrono>

namespace {
void require(bool ok, const std::string &reason) {
    if (!ok) throw std::runtime_error(reason + ": " + honeydew_surge_last_error());
}
void seed(SurgeSynthesizer &s, uint32_t value) {
    s.storage.rngGen.g.seed(value);
    s.storage.noiseRng.reseed(value + 1);
    std::srand(value);
    // Surge's per-scene Random modulators own additional random_device-seeded
    // generators. Match those too, only in QA; production randomness is intact.
    for (int sc = 0; sc < n_scenes; ++sc) for (auto id : {ms_random_bipolar, ms_random_unipolar}) {
        auto *random = static_cast<RandomModulationSource *>(s.storage.getPatch().scene[sc].modsources[id]);
        random->gen.seed(value + sc * 17 + id);
        random->norm.reset();
        random->dis.reset();
    }
}
struct Audio {
    std::vector<float> pcm;
    double energy{0};
    float peak{0};
    void add(float l, float r) {
        require(std::isfinite(l) && std::isfinite(r), "NaN/Inf in original PCM");
        peak = std::max({peak, std::abs(l), std::abs(r)});
        energy += double(l) * l + double(r) * r;
        pcm.push_back(l); pcm.push_back(r);
    }
    double rms() const { return std::sqrt(energy / std::max(size_t(1), pcm.size())); }
};
Audio render(void *handle, SurgeSynthesizer *reference, int blocks, bool playing, bool input = false) {
    Audio audio;
    audio.pcm.reserve(size_t(blocks) * 64);
    std::array<float, 34> left{}, right{};
    left.front() = right.front() = 12345.f;
    left.back() = right.back() = -54321.f;
    for (int b = 0; b < blocks; ++b) {
        std::array<float, 32> inL{}, inR{};
        if (input) for (int i = 0; i < 32; ++i) {
            const double t = (b * 32.0 + i) / 48000.0;
            // Identical external test signal for the original input oscillator
            // and vocoder, not a replacement for any preset's synthesizer.
            inL[i] = 0.25f * (std::sin(t * 1130.973355292) + std::sin(t * 3455.751918949));
            inR[i] = 0.8f * inL[i];
        }
        const auto ppq = playing ? b * 32.0 * 120.0 / (48000.0 * 60.0) : 0.0;
        if (handle) {
            const auto ok = input ? honeydew_surge_render32_with_input(handle, inL.data(), inR.data(),
                    left.data() + 1, right.data() + 1, ppq, 120.0, playing)
                : honeydew_surge_render32(handle, left.data() + 1, right.data() + 1, ppq, 120.0, playing);
            require(ok, "C ABI render failed");
            require(left.front() == 12345.f && right.front() == 12345.f &&
                    left.back() == -54321.f && right.back() == -54321.f,
                    "32-frame C ABI overwrote a buffer canary");
            for (int i = 1; i <= 32; ++i) audio.add(left[i], right[i]);
        } else if (reference) {
            reference->process_input = input;
            if (input) {
                std::copy(inL.begin(), inL.end(), reference->input[0]);
                std::copy(inR.begin(), inR.end(), reference->input[1]);
            }
            reference->time_data.tempo = 120;
            reference->time_data.ppqPos = ppq;
            reference->time_data.isPlaying = playing;
            reference->transportRunning = playing;
            reference->resetStateFromTimeData();
            reference->process();
            for (int i = 0; i < 32; ++i)
                audio.add(reference->output[0][i], reference->output[1][i]);
        } else {
            throw std::runtime_error("Audio QA needs a native handle or an official reference");
        }
    }
    return audio;
}
void wav(const fs::path &path, const Audio &audio) {
    std::ofstream out(path, std::ios::binary);
    auto u16 = [&out](uint16_t n) { out.write(reinterpret_cast<const char *>(&n), 2); };
    auto u32 = [&out](uint32_t n) { out.write(reinterpret_cast<const char *>(&n), 4); };
    const uint32_t bytes = audio.pcm.size() * sizeof(float);
    out.write("RIFF", 4); u32(bytes + 36); out.write("WAVEfmt ", 8); u32(16);
    u16(3); u16(2); u32(48000); u32(48000 * 8); u16(8); u16(32);
    out.write("data", 4); u32(bytes);
    out.write(reinterpret_cast<const char *>(audio.pcm.data()), bytes);
    require(bool(out), "Could not write audio evidence WAV");
}
void comparePatch(SurgeSynthesizer &s, SurgeSynthesizer &r, const std::string &path) {
    auto &a = s.storage.getPatch(); auto &b = r.storage.getPatch();
    require(a.param_ptr.size() == b.param_ptr.size(), "Parameter count mismatch");
    for (size_t i = 0; i < a.param_ptr.size(); ++i) {
        const auto p = a.param_ptr[i], q = b.param_ptr[i];
        require(p->val.i == q->val.i && p->deactivated == q->deactivated &&
                p->extend_range == q->extend_range && p->temposync == q->temposync &&
                p->deform_type == q->deform_type && p->absolute == q->absolute,
                path + " decoded parameter differs from desktop loader: " + p->get_storage_name());
    }
    for (int sc = 0; sc < n_scenes; ++sc) {
        require(a.scene[sc].modulation_voice.size() == b.scene[sc].modulation_voice.size() &&
                a.scene[sc].modulation_scene.size() == b.scene[sc].modulation_scene.size(),
                path + " lost scene modulation routes");
        for (int o = 0; o < n_oscs; ++o) {
            const auto &osc = a.scene[sc].osc[o];
            require(osc.queue_type == -1 && osc.wt.queue_id == -1 && osc.wt.queue_filename.empty()
                    && !osc.wt.queue_reslice, path + " left resource work in the audio callback");
            const auto type = a.scene[sc].osc[o].type.val.i;
            if (type != ot_wavetable && type != ot_window) continue;
            const auto &wa = a.scene[sc].osc[o].wt, &wb = b.scene[sc].osc[o].wt;
            require(wa.size == wb.size && wa.n_tables == wb.n_tables && wa.flags == wb.flags,
                    path + " lost an oscillator wavetable");
            require(wa.size > 0 && wa.n_tables > 0, path + " has an empty required wavetable");
            for (int table = 0; table < wa.n_tables; ++table)
                require(!std::memcmp(wa.TableF32WeakPointers[0][table], wb.TableF32WeakPointers[0][table],
                        wa.size * sizeof(float)), path + " wavetable samples differ");
        }
    }
    require(a.modulation_global.size() == b.modulation_global.size(), path + " lost global modulation");
}
void basics() {
    for (float rate : {44100.f, 48000.f, 96000.f}) {
        std::unique_ptr<Instance> instance(static_cast<Instance *>(honeydew_surge_create(rate)));
        require(bool(instance), "Init Saw construction failed");
        require(path_to_string(instance->synth->storage.userDataPath).find(g_private_home) == 0,
                "User data escaped Honeydew private storage");
        require(path_to_string(instance->synth->storage.datapath) == g_factory_data_path,
                "Factory data used desktop portable-path discovery");
        honeydew_surge_note_on(instance.get(), 0, 69, 112);
        auto audio = render(instance.get(), nullptr, int(rate / 32), false);
        require(audio.peak > 0.001f, "Init Saw note produced silence");
        honeydew_surge_note_off(instance.get(), 0, 69);
        render(instance.get(), nullptr, int(rate * 2 / 32), false);
        require(instance->synth->storage.activeVoiceCount == 0, "Init Saw Note Off did not release");
        honeydew_surge_pitch_bend(instance.get(), 0, 8192);
        require(instance->synth->storage.pitch_bend == 0, "Centered MIDI bend detuned the patch");
        honeydew_surge_pitch_bend(instance.get(), 0, 0);
        require(instance->synth->storage.pitch_bend == -1.f, "Negative bend is not signed");
        honeydew_surge_pitch_bend(instance.get(), 0, 8192);
        for (uint8_t key : {60, 64, 67}) honeydew_surge_note_on(instance.get(), 0, key, 100);
        render(instance.get(), nullptr, 300, true);
        require(instance->synth->storage.activeVoiceCount >= 3, "Init Saw lost polyphony");
        honeydew_surge_cc(instance.get(), 0, 64, 127);
        for (uint8_t key : {60, 64, 67}) honeydew_surge_note_off(instance.get(), 0, key);
        render(instance.get(), nullptr, 300, true);
        require(instance->synth->storage.activeVoiceCount >= 3, "Surge sustain was not delivered");
        honeydew_surge_cc(instance.get(), 0, 64, 0);
        render(instance.get(), nullptr, int(rate * 2 / 32), true);
        require(instance->synth->storage.activeVoiceCount == 0, "Sustain release left stuck voices");
        honeydew_surge_panic(instance.get());
        honeydew_surge_note_on(instance.get(), 15, 60, 100);
        auto retrigger = render(instance.get(), nullptr, 1500, false);
        require(retrigger.peak > 0.001f && instance->synth->storage.activeVoiceCount > 0,
                "Host reset killed the next note");
        honeydew_surge_all_notes_off(instance.get());
        render(instance.get(), nullptr, int(rate * 2 / 32), false);
        require(instance->synth->storage.activeVoiceCount == 0, "All Notes Off left stuck voices");
        require(!honeydew_surge_load_factory(instance.get(), 642), "Invalid preset index accepted");
        require(instance->state.faults == 0, "Native fault counter is nonzero");
    }
    require(!honeydew_surge_create(std::nanf("")), "NaN sample rate accepted");
}
} // namespace

int main(int argc, char **argv) {
    try {
        require(argc == 4, "usage: audio_test <factory-data> <private-user-dir> <evidence-dir>");
        require(honeydew_surge_set_data_path(argv[1], argv[2]), "Private path initialization failed");
        fs::create_directories(fs::path(argv[3]));
        const auto started = std::chrono::steady_clock::now();
        basics();
        auto bridge = std::make_unique<Instance>(48000.f);
        Host referenceHost;
        auto reference = std::make_unique<SurgeSynthesizer>(
            &referenceHost, SurgeStorage::skipPatchLoadDataPathSentinel);
        reference->storage.addErrorListener(&referenceHost);
        reference->storage.datapath = string_to_path(argv[1]);
        reference->storage.refresh_wtlist();
        reference->storage.refresh_irlist();
        reference->setSamplerate(48000);
        reference->time_data.tempo = 120;
        reference->time_data.ppqPos = 0;
        std::ofstream report(fs::path(argv[3]) / "factory-audio.tsv");
        report << "index\tpath\tpeak\trms\treference_peak\tmax_sample_difference\tstatus\n";
        std::set<std::string> categories;
        std::vector<int> variations;
        int audible = 0, exact = 0, silent = 0, inputRequired = 0;
        for (int preset = 1; preset <= honeydew_surge_factory_count; ++preset) {
            const auto &entry = honeydew_surge_factory[preset - 1];
            const std::string path = entry.path;
            const auto source = fs::path(argv[1]) / "patches_factory" / path;
            std::ifstream file(source, std::ios::binary);
            std::vector<uint8_t> raw((std::istreambuf_iterator<char>(file)), {});
            require(raw.size() == entry.size + 60 && !std::memcmp(raw.data() + 60,
                    honeydew_surge_factory_blob_start + entry.offset, entry.size),
                    path + " native catalog points at a different FXP");
            seed(*bridge->synth, 1234);
            require(honeydew_surge_load_factory(bridge.get(), preset), path + " native FXP load failed");
            seed(*reference, 1234);
            require(reference->loadPatchByPath(path_to_string(source).c_str(), -1,
                    path_to_string(source.stem()).c_str(), false), path + " official desktop load failed");
            reference->processAudioThreadOpsWhenAudioEngineUnavailable(true);
            reference->processControl();
            require(referenceHost.errors == 0, path + " official resource dependency failed");
            comparePatch(*bridge->synth, *reference, path);
            const auto category = path.substr(0, path.find('/'));
            const uint8_t key = category == "Basses" ? 36 : 60;
            const bool playing = preset % 2 == 0;
            const int blocks = (category == "Pads" || category == "FX" || category == "Sequences") ? 9000 : 4500;
            // The bridge and reference get identical MIDI, rate, transport and
            // seeds. RNG variations in some FX are recorded, not normalized away.
            seed(*bridge->synth, 5678);
            honeydew_surge_note_on(bridge.get(), 0, key, 112);
            auto audio = render(bridge.get(), nullptr, blocks, playing);
            seed(*reference, 5678);
            reference->playNote(0, key, 112, 0);
            auto expected = render(nullptr, reference.get(), blocks, playing);
            float difference = 0;
            for (size_t i = 0; i < audio.pcm.size(); ++i)
                difference = std::max(difference, std::abs(audio.pcm[i] - expected.pcm[i]));
            // The official reference must agree on silence. Slow attacks get
            // another 16 seconds and low/high keys before being classified.
            if (audio.peak < 1e-7f && expected.peak < 1e-7f) {
                for (uint8_t extra : {24, 48, 84, 108}) {
                    honeydew_surge_note_on(bridge.get(), 0, extra, 127);
                    reference->playNote(0, extra, 127, 0);
                }
                auto longAudio = render(bridge.get(), nullptr, 24000, playing);
                auto longExpected = render(nullptr, reference.get(), 24000, playing);
                audio.peak = std::max(audio.peak, longAudio.peak);
                expected.peak = std::max(expected.peak, longExpected.peak);
            }
            require(audio.peak > 1e-7f || expected.peak <= 1e-7f,
                    path + " is silent in Honeydew but audible in original Surge");
            bool withInput = false;
            if (audio.peak <= 1e-7f && expected.peak <= 1e-7f) {
                // Restore the patch and compare the real desktop input bus.
                seed(*bridge->synth, 1234);
                require(honeydew_surge_load_factory(bridge.get(), preset), path + " input reload failed");
                seed(*reference, 1234);
                require(reference->loadPatchByPath(path_to_string(source).c_str(), -1,
                    path_to_string(source.stem()).c_str(), false), path + " reference input reload failed");
                reference->processAudioThreadOpsWhenAudioEngineUnavailable(true);
                reference->processControl();
                seed(*bridge->synth, 5678);
                honeydew_surge_note_on(bridge.get(), 0, 60, 112);
                audio = render(bridge.get(), nullptr, 9000, playing, true);
                seed(*reference, 5678);
                reference->playNote(0, 60, 112, 0);
                expected = render(nullptr, reference.get(), 9000, playing, true);
                require(audio.peak > 1e-7f && expected.peak > 1e-7f,
                        path + " failed with its original audio input bus");
                difference = 0;
                for (size_t i = 0; i < audio.pcm.size(); ++i)
                    difference = std::max(difference, std::abs(audio.pcm[i] - expected.pcm[i]));
                ++inputRequired;
                withInput = true;
            }
            const auto status = withInput ? "audible_requires_audio_input" : "audible";
            if (difference < 1e-5f) ++exact;
            else variations.push_back(preset);
            if (audio.peak > 1e-7f) ++audible; else ++silent;
            if (categories.insert(category).second) wav(fs::path(argv[3]) / (category + ".wav"), audio);
            honeydew_surge_all_notes_off(bridge.get()); reference->allNotesOff();
            render(bridge.get(), nullptr, 1500, playing);
            render(nullptr, reference.get(), 1500, playing);
            require(bridge->state.faults == 0, path + " synthesis fault");
            report << preset << '\t' << path << '\t' << std::setprecision(9) << audio.peak
                   << '\t' << audio.rms() << '\t' << expected.peak << '\t' << difference
                   << '\t' << status << '\n';
            report.flush();
            if (preset % 25 == 0) std::cout << "Validated original audio " << preset << "/641\n" << std::flush;
        }
        reference->storage.removeErrorListener(&referenceHost);
        // Check differing patches against two independent official instances,
        // separating engine randomness/history from a bridge discrepancy.
        std::ofstream variationReport(fs::path(argv[3]) / "reference-variations.tsv");
        variationReport << "path\tfresh_bridge_difference\tofficial_instance_difference\tresult\n";
        int freshMatches = 0;
        for (int preset : variations) {
            const std::string path = honeydew_surge_factory[preset - 1].path;
            const auto source = fs::path(argv[1]) / "patches_factory" / path;
            auto fresh = std::make_unique<Instance>(48000.f);
            Host h1, h2;
            auto makeReference = [&](Host &host) {
                auto r = std::make_unique<SurgeSynthesizer>(&host, SurgeStorage::skipPatchLoadDataPathSentinel);
                r->storage.datapath = string_to_path(argv[1]);
                r->storage.refresh_wtlist(); r->storage.refresh_irlist();
                r->setSamplerate(48000);
                r->time_data.tempo = 120; r->time_data.ppqPos = 0;
                r->time_data.timeSigNumerator = r->time_data.timeSigDenominator = 4;
                r->resetStateFromTimeData();
                const auto init = fs::path(argv[1]) / "patches_factory/Templates/Init Saw.fxp";
                require(r->loadPatchByPath(path_to_string(init).c_str(), -1, "Init Saw", false), "Reference init failed");
                r->processAudioThreadOpsWhenAudioEngineUnavailable(true); r->processControl();
                r->audio_processing_active = true;
                seed(*r, 1234);
                require(r->loadPatchByPath(path_to_string(source).c_str(), -1,
                    path_to_string(source.stem()).c_str(), false), "Reference variation load failed");
                r->processAudioThreadOpsWhenAudioEngineUnavailable(true); r->processControl();
                return r;
            };
            seed(*fresh->synth, 1234);
            require(honeydew_surge_load_factory(fresh.get(), preset), "Fresh factory load failed");
            auto r1 = makeReference(h1), r2 = makeReference(h2);
            const auto category = path.substr(0, path.find('/'));
            const int blocks = (category == "Pads" || category == "FX" || category == "Sequences") ? 9000 : 4500;
            seed(*fresh->synth, 5678); honeydew_surge_note_on(fresh.get(), 0, 60, 112);
            auto a = render(fresh.get(), nullptr, blocks, preset % 2 == 0);
            seed(*r1, 5678); r1->playNote(0, 60, 112, 0);
            auto b = render(nullptr, r1.get(), blocks, preset % 2 == 0);
            seed(*r2, 5678); r2->playNote(0, 60, 112, 0);
            auto c = render(nullptr, r2.get(), blocks, preset % 2 == 0);
            float ab = 0, bc = 0;
            for (size_t i = 0; i < a.pcm.size(); ++i) {
                ab = std::max(ab, std::abs(a.pcm[i] - b.pcm[i]));
                bc = std::max(bc, std::abs(b.pcm[i] - c.pcm[i]));
            }
            require(ab < 1e-5f || bc >= 1e-5f,
                    path + " differs only in the bridge, not between original instances");
            if (ab < 1e-5f) ++freshMatches;
            variationReport << path << '\t' << ab << '\t' << bc << '\t'
                << (ab < 1e-5f ? "fresh_matches_official" : "independent_official_instances_vary") << '\n';
        }
        const auto seconds = std::chrono::duration<double>(std::chrono::steady_clock::now() - started).count();
        std::ofstream summary(fs::path(argv[3]) / "summary.json");
        summary << "{\"presets\":641,\"audible\":" << audible << ",\"audio_input_required\":"
                << inputRequired << ",\"silent_matching_official\":"
                << silent << ",\"sample_matches_below_1e-5\":" << exact
                << ",\"fresh_sample_matches\":" << freshMatches
                << ",\"categories\":" << categories.size() << ",\"elapsed_seconds\":" << seconds << "}\n";
        std::cout << "PASS: 641 complete FXPs, " << audible << " audible, " << silent
                  << " silent under the same conditions in official Surge; " << exact
                  << " sample comparisons <1e-5; " << seconds << " seconds\n";
    } catch (const std::exception &e) {
        std::cerr << "FAIL: " << e.what() << '\n'; return 1;
    }
}
