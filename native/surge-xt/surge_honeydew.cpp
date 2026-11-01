// Copyright (c) the Honeydew contributors. GPL-3.0-or-later.
// This is a C ABI around *Surge XT's original C++ synthesizer*, not a DSP reimplementation.
// Build with the unmodified Surge XT source through bridge.cmake.
#include "SurgeSynthesizer.h"
#include "surge_honeydew.h"
#include "honeydew_surge_init_patch.h"
#include "honeydew_surge_factory_bank.h"
#if defined(__ANDROID__)
#include <jni.h>
#endif
#include <algorithm>
#include <array>
#include <cmath>
#include <cstdint>
#include <cstdio>
#include <cstring>
#include <exception>
#include <memory>
#include <new>
#include <string>
#include <stdexcept>
#include <vector>

namespace {
// Java initializes this once, before starting NativeActivity's Rust engine.
// Actual factory .wt/.wav data reside in the application's private files.
std::string g_factory_data_path;
std::string g_private_home;
thread_local char g_error[1024]{};
void error(const char *stage, const char *detail) noexcept {
    std::snprintf(g_error, sizeof(g_error), "%s: %s", stage, detail);
}
static_assert(BLOCK_SIZE == HONEYDEW_SURGE_BLOCK, "C++ and Rust must use identical block sizes");
static_assert(honeydew_surge_factory_count == 641, "Factory catalog ABI changed");
// Exact native Surge OSC parameter keys from the upstream SurgePatch registry,
// resolved off the realtime thread and cached as original Surge parameter IDs.
constexpr const char *kPaths[] = {
    "a/osc/1/type", "a/osc/2/type", "a/osc/3/type",
    "a/osc/1/pitch", "a/osc/1/param1",
    "a/mixer/osc1/volume", "a/mixer/osc2/volume", "a/mixer/osc3/volume",
    "a/filter/1/type", "a/filter/1/cutoff", "a/filter/1/resonance",
    "a/filter/1/feg_amount",
    "a/aeg/attack", "a/aeg/decay", "a/aeg/sustain", "a/aeg/release",
    "a/feg/attack", "a/feg/decay", "a/feg/sustain", "a/feg/release",
    "a/vlfo/1/rate", "a/vlfo/1/type", "a/vlfo/1/amplitude",
    "global/volume",
};
constexpr int kParameterCount = sizeof(kPaths) / sizeof(kPaths[0]);
struct Host final : SurgeSynthesizer::PluginLayer, SurgeStorage::ErrorListener {
    uint32_t errors{0};
    char detail[1024]{};
    void surgeParameterUpdated(const SurgeSynthesizer::ID &, float) override {}
    void surgeMacroUpdated(long, float) override {}
    void onSurgeError(const std::string &msg, const std::string &title,
                      const SurgeStorage::ErrorType &) override {
        ++errors;
        std::snprintf(detail, sizeof(detail), "%s: %s", title.c_str(), msg.c_str());
    }
};
struct Instance {
    Host host;
    std::unique_ptr<SurgeSynthesizer> synth;
    std::array<int, kParameterCount> param_ids{};
    HoneydewSurgeState state{};
    void load(const uint8_t *data, size_t size, int preset) {
        if (size < 32 || std::memcmp(data, "sub3", 4))
            throw std::runtime_error("C: invalid original FXP chunk header");
        // loadRaw's patch_loaded flag is not a parse result. Validate the XML
        // using the same TinyXML implementation before accepting the chunk.
        uint32_t xmlsize = uint32_t(data[4]) | uint32_t(data[5]) << 8 |
                           uint32_t(data[6]) << 16 | uint32_t(data[7]) << 24;
        if (xmlsize > size - 32) throw std::runtime_error("C: truncated patch XML");
        std::string xml(reinterpret_cast<const char *>(data + 32), xmlsize);
        TiXmlDocument document;
        document.Parse(xml.c_str(), nullptr, TIXML_ENCODING_LEGACY);
        auto patch = document.FirstChildElement("patch");
        if (document.Error() || !patch || !patch->FirstChildElement("parameters"))
            throw std::runtime_error("C: original patch XML could not be decoded");
        // Upstream endian-decodes patch_header in place. Never pass ELF .rodata.
        std::vector<uint8_t> writable(data, data + size);
        host.errors = 0;
        synth->patch_loaded = false;
        synth->loadRaw(writable.data(), static_cast<int>(writable.size()), false);
        if (!synth->patch_loaded || host.errors)
            throw std::runtime_error(host.errors ? host.detail : "C: loadRaw failed");
        // Populate scene data and drain queued resource/FX preparation BEFORE
        // the first note. This also keeps lazy patch work out of our callback.
        synth->processAudioThreadOpsWhenAudioEngineUnavailable(true);
        synth->processControl();
        if (host.errors) throw std::runtime_error(host.detail);
        state = {};
        state.preset = preset;
        synth->audio_processing_active = true;
    }
    explicit Instance(float rate) {
        if (g_factory_data_path.empty() || g_private_home.empty())
            throw std::runtime_error("B: HoneydewActivity did not initialize private Surge paths");
        // The patch bank already lives in memory; do not scan 641 FXP files or
        // auto-load a desktop default. Only scan the original stock resources.
        synth = std::make_unique<SurgeSynthesizer>(
            &host, SurgeStorage::skipPatchLoadDataPathSentinel);
        synth->storage.addErrorListener(&host);
        if (host.errors) throw std::runtime_error(host.detail);
        synth->storage.datapath = string_to_path(g_factory_data_path);
        synth->storage.refresh_wtlist();
        synth->storage.refresh_irlist();
        if (synth->storage.WindowWT.size == 0 || synth->storage.wt_list.empty())
            throw std::runtime_error("B: missing original Window/factory wavetable resources");
        synth->setSamplerate(rate);
        // Decode the actual Surge XT factory Init Saw patch (FXP payload).
        // Upstream SurgePatch::load_patch() intentionally byte-swaps fields
        // *in place* (patch_header::xmlsize and wtsize). Never give it the
        // compiled const factory array in ELF .rodata: that memory is
        // write-protected on Android and corrupts/segfaults the patch load.
        synth->time_data.tempo = 120.0;
        synth->time_data.ppqPos = 0.0;
        synth->time_data.timeSigNumerator = 4;
        synth->time_data.timeSigDenominator = 4;
        synth->resetStateFromTimeData();
        load(honeydew_surge_init_data, sizeof(honeydew_surge_init_data), 0);
        auto &registry = synth->storage.getPatch().param_ptr_by_oscname;
        for (int n = 0; n < kParameterCount; ++n) {
            // Surge names every host parameter with the '/param/' OSC prefix.
            auto item = registry.find(std::string("/param/") + kPaths[n]);
            param_ids[n] = item == registry.end() ? -1 : item->second->id;
        }
    }
    ~Instance() {
        if (synth) synth->storage.removeErrorListener(&host);
    }
};
void operationFailed(void *handle, const char *detail) noexcept {
    if (handle) ++static_cast<Instance *>(handle)->state.faults;
    error("D", detail);
}
} // namespace

// Platform-path adapter used by sst-plugininfra on Android (and the native QA
// build). No process-wide HOME/XDG mutation and no dependence on /sdcard.
extern "C" const char *honeydew_surge_private_home() noexcept {
    return g_private_home.empty() ? nullptr : g_private_home.c_str();
}

// Called from HoneydewActivity.onCreate before NativeActivity starts Rust;
// must remain outside the audio callback, since the host path is initialized
// only once. Android and the C++ DSP use the same original factory wavetable data.
#if defined(__ANDROID__)
extern "C" JNIEXPORT jboolean JNICALL
Java_dev_daijro_honeydew_HoneydewActivity_nativeSetSurgeFactoryDataDir(
    JNIEnv *env, jclass, jstring path, jstring user) noexcept {
    if (!path || !user) return JNI_FALSE;
    const char *utf8 = env->GetStringUTFChars(path, nullptr);
    if (!utf8) return JNI_FALSE;
    const char *user_utf8 = env->GetStringUTFChars(user, nullptr);
    if (!user_utf8) { env->ReleaseStringUTFChars(path, utf8); return JNI_FALSE; }
    const auto ok = honeydew_surge_set_data_path(utf8, user_utf8);
    env->ReleaseStringUTFChars(path, utf8);
    env->ReleaseStringUTFChars(user, user_utf8);
    if (!ok) {
        if (auto exception = env->FindClass("java/io/IOException"))
            env->ThrowNew(exception, g_error);
    }
    return ok ? JNI_TRUE : JNI_FALSE;
}
#endif

// Export a stable C ABI to Rust even when hiding Surge internal C++ symbols.
extern "C" {
HONEYDEW_EXPORT uint32_t honeydew_surge_abi_version() noexcept { return HONEYDEW_SURGE_ABI; }
HONEYDEW_EXPORT const char *honeydew_surge_last_error() noexcept { return g_error; }
HONEYDEW_EXPORT int honeydew_surge_set_data_path(const char *factory, const char *user) noexcept {
    try {
        if (!factory || !user || !fs::path(factory).is_absolute() || !fs::path(user).is_absolute())
            throw std::runtime_error("Factory/private user paths must be absolute");
        if (!fs::is_regular_file(fs::path(factory) / "wavetables/Basic/Sine.wt") ||
            !fs::is_regular_file(fs::path(factory) / "patches_factory/Templates/Init Saw.fxp"))
            throw std::runtime_error("Original factory FXP/wavetable files are absent");
        fs::create_directories(fs::path(user));
        const auto data = path_to_string(fs::canonical(fs::path(factory)));
        const auto home = path_to_string(fs::canonical(fs::path(user)));
        // Repeated Linux QA instances use the same immutable process paths.
        if (!g_factory_data_path.empty() && (data != g_factory_data_path || home != g_private_home))
            throw std::runtime_error("Surge paths cannot change while the process is running");
        g_factory_data_path = data;
        g_private_home = home;
        g_error[0] = 0;
        return 1;
    } catch (const std::exception &e) { error("B", e.what()); return 0; }
    catch (...) { error("B", "Unknown private-path initialization exception"); return 0; }
}
HONEYDEW_EXPORT const char *honeydew_surge_factory_path(int preset) noexcept {
    if (preset == 0) return "Templates/Init Saw.fxp";
    return preset > 0 && preset <= honeydew_surge_factory_count
        ? honeydew_surge_factory[preset - 1].path : nullptr;
}
// The caller owns the returned pointer. Construction may load Surge's internal resources,
// so it must happen off the real-time callback.
HONEYDEW_EXPORT void *honeydew_surge_create(float sample_rate) noexcept {
    if (!std::isfinite(sample_rate) || sample_rate < 8000 || sample_rate > 384000) {
        error("B", "Unsupported sample rate"); return nullptr;
    }
    try { g_error[0] = 0; return new Instance(sample_rate); }
    catch (const std::exception &e) { error("B", e.what()); return nullptr; }
    catch (...) { error("B", "Unknown synthesizer constructor exception"); return nullptr; }
}
HONEYDEW_EXPORT void honeydew_surge_destroy(void *handle) noexcept {
    delete static_cast<Instance *>(handle);
}
HONEYDEW_EXPORT void honeydew_surge_note_on(void *handle, uint8_t channel, uint8_t key, uint8_t velocity) noexcept {
    if (!handle) return;
    try {
        auto &instance = *static_cast<Instance *>(handle);
        if (velocity == 0) { honeydew_surge_note_off(handle, channel, key); return; }
        ++instance.state.note_ons;
        instance.synth->playNote(
            static_cast<char>(channel & 15), static_cast<char>(key & 127),
            static_cast<char>(velocity & 127), 0);
    } catch (const std::exception &e) { error("D", e.what()); ++static_cast<Instance *>(handle)->state.faults; }
    catch (...) { error("D", "Unknown Note On exception"); ++static_cast<Instance *>(handle)->state.faults; }
}
HONEYDEW_EXPORT void honeydew_surge_note_off(void *handle, uint8_t channel, uint8_t key) noexcept {
    if (!handle) return;
    try {
        auto &instance = *static_cast<Instance *>(handle);
        ++instance.state.note_offs;
        instance.synth->releaseNote(
            static_cast<char>(channel & 15), static_cast<char>(key & 127), 0);
    } catch (const std::exception &e) { operationFailed(handle, e.what()); }
    catch (...) { operationFailed(handle, "Unknown Note Off exception"); }
}
HONEYDEW_EXPORT void honeydew_surge_panic(void *handle) noexcept {
    if (!handle) return;
    // Host reset is immediate. allSoundOff() instead queues an eight-block
    // fade and can kill a NEW note played just after Enable is switched on.
    try { static_cast<Instance *>(handle)->synth->stopSound(); }
    catch (const std::exception &e) { operationFailed(handle, e.what()); }
    catch (...) { operationFailed(handle, "Host reset failed"); }
}
HONEYDEW_EXPORT void honeydew_surge_all_notes_off(void *handle) noexcept {
    if (!handle) return;
    try { static_cast<Instance *>(handle)->synth->allNotesOff(); }
    catch (const std::exception &e) { operationFailed(handle, e.what()); }
    catch (...) { operationFailed(handle, "All Notes Off failed"); }
}
HONEYDEW_EXPORT void honeydew_surge_cc(void *handle, uint8_t channel, uint8_t cc, uint8_t value) noexcept {
    if (!handle) return;
    try {
        static_cast<Instance *>(handle)->synth->channelController(
            static_cast<char>(channel & 15), cc & 127, value & 127);
    } catch (const std::exception &e) { operationFailed(handle, e.what()); }
    catch (...) { operationFailed(handle, "MIDI controller failed"); }
}
HONEYDEW_EXPORT void honeydew_surge_pitch_bend(void *handle, uint8_t channel, int bend14) noexcept {
    if (!handle) return;
    try {
        static_cast<Instance *>(handle)->synth->pitchBend(
            static_cast<char>(channel & 15), std::clamp(bend14, 0, 16383) - 8192);
    } catch (const std::exception &e) { operationFailed(handle, e.what()); }
    catch (...) { operationFailed(handle, "MIDI pitch bend failed"); }
}
HONEYDEW_EXPORT void honeydew_surge_aftertouch(void *handle, uint8_t channel, uint8_t pressure) noexcept {
    if (!handle) return;
    try { static_cast<Instance *>(handle)->synth->channelAftertouch(channel & 15, pressure & 127); }
    catch (const std::exception &e) { operationFailed(handle, e.what()); }
    catch (...) { operationFailed(handle, "Channel pressure failed"); }
}
HONEYDEW_EXPORT void honeydew_surge_native_param(void *handle, int which, float value) noexcept {
    if (!handle || which < 0 || which >= kParameterCount || !std::isfinite(value)) return;
    try {
        auto &instance = *static_cast<Instance *>(handle);
        auto id = instance.param_ids[which];
        SurgeSynthesizer::ID parameter;
        if (instance.synth->fromSynthSideId(id, parameter)) {
            instance.synth->setParameter01(parameter, std::clamp(value, 0.f, 1.f), true);
        }
    } catch (const std::exception &e) { operationFailed(handle, e.what()); }
    catch (...) { operationFailed(handle, "Native parameter update failed"); }
}
// Called from the Honeydew app's instrument constructor, never from its
// realtime callback. The pack is the 641 official factory FXP payloads.
HONEYDEW_EXPORT int honeydew_surge_load_factory(void *handle, int preset) noexcept {
    if (!handle || preset <= 0 || preset > honeydew_surge_factory_count) return 0;
    try {
        auto &instance = *static_cast<Instance *>(handle);
        const auto &entry = honeydew_surge_factory[preset - 1];
        // Original Surge loads and endian-decodes its patch_header in place.
        // The factory bank lives in read-only ELF .rodata, so copy just the
        // selected FXP into writable, aligned storage before calling loadRaw.
        instance.load(honeydew_surge_factory_blob_start + entry.offset, entry.size, preset);
        return 1;
    } catch (const std::exception &e) { error("C", e.what()); return 0; }
    catch (...) { error("C", "Unknown factory FXP load exception"); return 0; }
}
HONEYDEW_EXPORT void honeydew_surge_macro(void *handle, int which, float value) noexcept {
    if (!handle || which < 0 || which >= n_customcontrollers || !std::isfinite(value)) return;
    try {
        static_cast<Instance *>(handle)->synth->setMacroParameter01(
            which, std::clamp(value, 0.0f, 1.0f));
    } catch (const std::exception &e) { operationFailed(handle, e.what()); }
    catch (...) { operationFailed(handle, "Macro update failed"); }
}
// Android runtime audio self-check. The original Init Saw patch must produce
// non-zero PCM with a standard note; this runs at instrument startup (not in
// the realtime callback) and never replaces Surge XT sound generation.
HONEYDEW_EXPORT float honeydew_surge_probe(float sample_rate) noexcept {
    if (!std::isfinite(sample_rate) || sample_rate < 8000 || sample_rate > 384000)
        return -1.f;
    try {
        Instance instance(sample_rate);
        auto &s = *instance.synth;
        s.playNote(0, 60, 112, 0);
        float peak = 0.f;
        for (int block = 0; block < 384; ++block) {
            s.process();
            for (int i = 0; i < 32; ++i) {
                const auto l = s.output[0][i];
                const auto r = s.output[1][i];
                if (!std::isfinite(l) || !std::isfinite(r)) {
                    error("E", "Init Saw generated NaN/Inf"); return -1.f;
                }
                peak = std::max({peak, std::abs(l), std::abs(r)});
            }
        }
        s.allSoundOff();
        return peak;
    } catch (const std::exception &e) { error("B/E", e.what()); return -1.f; }
    catch (...) { error("B/E", "Unknown Init Saw probe exception"); return -1.f; }
}

HONEYDEW_EXPORT int honeydew_surge_get_state(void *handle, HoneydewSurgeState *state) noexcept {
    if (!handle || !state) return 0;
    auto &instance = *static_cast<Instance *>(handle);
    instance.state.voices = instance.synth->storage.activeVoiceCount;
    *state = instance.state;
    return 1;
}

// Exactly Surge's native 32-frame process block (SURGE_COMPILE_BLOCK_SIZE=32).
// No allocation, file I/O or synchronization is performed by this ABI layer.
static int render32_core(void *handle, float *left, float *right,
                            double ppq, double bpm, int playing) noexcept {
    if (!handle || !left || !right) return 0;
    try {
        auto &instance = *static_cast<Instance *>(handle);
        auto &s = *instance.synth;
        if (instance.state.faults) {
            std::fill_n(left, BLOCK_SIZE, 0.f); std::fill_n(right, BLOCK_SIZE, 0.f);
            return 0; // Keep the first concrete native error for the UI.
        }
        if (std::isfinite(bpm) && bpm > 0) s.time_data.tempo = bpm;
        if (std::isfinite(ppq)) s.time_data.ppqPos = ppq;
        s.time_data.isPlaying = playing != 0;
        s.transportRunning = playing != 0;
        s.resetStateFromTimeData();
        s.process();
        ++instance.state.blocks;
        instance.state.peak = 0.f;
        for (int i = 0; i < 32; ++i) {
            if (!std::isfinite(s.output[0][i]) || !std::isfinite(s.output[1][i]))
                throw std::runtime_error("Original engine produced NaN/Inf");
            left[i] = s.output[0][i];
            right[i] = s.output[1][i];
            instance.state.peak = std::max({instance.state.peak, std::abs(left[i]), std::abs(right[i])});
        }
        if (instance.host.errors) throw std::runtime_error(instance.host.detail);
        return 1;
    } catch (const std::exception &e) {
        error("E", e.what());
        ++static_cast<Instance *>(handle)->state.faults;
        std::fill_n(left, 32, 0.f);
        std::fill_n(right, 32, 0.f);
        return 0;
    } catch (...) {
        error("E", "Unknown render exception");
        ++static_cast<Instance *>(handle)->state.faults;
        std::fill_n(left, 32, 0.f); std::fill_n(right, 32, 0.f); return 0;
    }
}
HONEYDEW_EXPORT int honeydew_surge_render32(void *handle, float *left, float *right,
                            double ppq, double bpm, int playing) noexcept {
    if (handle) static_cast<Instance *>(handle)->synth->process_input = false;
    return render32_core(handle, left, right, ppq, bpm, playing);
}
HONEYDEW_EXPORT int honeydew_surge_render32_with_input(void *handle,
                            const float *input_left, const float *input_right,
                            float *left, float *right, double ppq, double bpm, int playing) noexcept {
    if (handle) {
        auto &s = *static_cast<Instance *>(handle)->synth;
        s.process_input = input_left && input_right;
        if (s.process_input) {
            std::copy_n(input_left, BLOCK_SIZE, s.input[0]);
            std::copy_n(input_right, BLOCK_SIZE, s.input[1]);
        }
    }
    return render32_core(handle, left, right, ppq, bpm, playing);
}
} // extern "C"
