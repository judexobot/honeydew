// Copyright (c) the Honeydew contributors. GPL-3.0-or-later.
// This is a C ABI around *Surge XT's original C++ synthesizer*, not a DSP reimplementation.
// Build with the unmodified Surge XT source through bridge.cmake.
#include "SurgeSynthesizer.h"
#include "honeydew_surge_init_patch.h"
#include <algorithm>
#include <array>
#include <cmath>
#include <cstdint>
#include <memory>
#include <new>
#include <string>

namespace {
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
struct Host final : SurgeSynthesizer::PluginLayer {
    void surgeParameterUpdated(const SurgeSynthesizer::ID &, float) override {}
    void surgeMacroUpdated(long, float) override {}
};
struct Instance {
    Host host;
    std::unique_ptr<SurgeSynthesizer> synth;
    std::array<int, kParameterCount> param_ids{};
    explicit Instance(float rate) {
        synth = std::make_unique<SurgeSynthesizer>(
            &host, SurgeStorage::skipPatchLoadDataPathSentinel);
        synth->setSamplerate(rate);
        // Decode the actual Surge XT factory Init Saw patch (FXP payload).
        synth->loadRaw(honeydew_surge_init_data, sizeof(honeydew_surge_init_data), false);
        synth->time_data.tempo = 120.0;
        synth->time_data.ppqPos = 0.0;
        auto &registry = synth->storage.getPatch().param_ptr_by_oscname;
        for (int n = 0; n < kParameterCount; ++n) {
            auto item = registry.find(kPaths[n]);
            param_ids[n] = item == registry.end() ? -1 : item->second->id;
        }
    }
};
} // namespace

// Export a stable C ABI to Rust even when hiding Surge internal C++ symbols.
#define HONEYDEW_EXPORT __attribute__((visibility("default")))
extern "C" {
// The caller owns the returned pointer. Construction may load Surge's internal resources,
// so it must happen off the real-time callback.
HONEYDEW_EXPORT void *honeydew_surge_create(float sample_rate) noexcept {
    if (!std::isfinite(sample_rate) || sample_rate < 8000 || sample_rate > 384000) return nullptr;
    try { return new Instance(sample_rate); } catch (...) { return nullptr; }
}
HONEYDEW_EXPORT void honeydew_surge_destroy(void *handle) noexcept {
    delete static_cast<Instance *>(handle);
}
HONEYDEW_EXPORT void honeydew_surge_note_on(void *handle, uint8_t channel, uint8_t key, uint8_t velocity) noexcept {
    if (!handle) return;
    try {
        static_cast<Instance *>(handle)->synth->playNote(
            static_cast<char>(channel & 15), static_cast<char>(key & 127),
            static_cast<char>(velocity & 127), 0);
    } catch (...) {}
}
HONEYDEW_EXPORT void honeydew_surge_note_off(void *handle, uint8_t channel, uint8_t key) noexcept {
    if (!handle) return;
    try {
        static_cast<Instance *>(handle)->synth->releaseNote(
            static_cast<char>(channel & 15), static_cast<char>(key & 127), 0);
    } catch (...) {}
}
HONEYDEW_EXPORT void honeydew_surge_panic(void *handle) noexcept {
    if (!handle) return;
    try { static_cast<Instance *>(handle)->synth->allSoundOff(); } catch (...) {}
}
HONEYDEW_EXPORT void honeydew_surge_all_notes_off(void *handle) noexcept {
    if (!handle) return;
    try { static_cast<Instance *>(handle)->synth->allNotesOff(); } catch (...) {}
}
HONEYDEW_EXPORT void honeydew_surge_cc(void *handle, uint8_t channel, uint8_t cc, uint8_t value) noexcept {
    if (!handle) return;
    try {
        static_cast<Instance *>(handle)->synth->channelController(
            static_cast<char>(channel & 15), cc & 127, value & 127);
    } catch (...) {}
}
HONEYDEW_EXPORT void honeydew_surge_pitch_bend(void *handle, uint8_t channel, int bend14) noexcept {
    if (!handle) return;
    try {
        static_cast<Instance *>(handle)->synth->pitchBend(
            static_cast<char>(channel & 15), std::clamp(bend14, 0, 16383));
    } catch (...) {}
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
    } catch (...) {}
}
HONEYDEW_EXPORT void honeydew_surge_macro(void *handle, int which, float value) noexcept {
    if (!handle || which < 0 || which >= n_customcontrollers || !std::isfinite(value)) return;
    try {
        static_cast<Instance *>(handle)->synth->setMacroParameter01(
            which, std::clamp(value, 0.0f, 1.0f));
    } catch (...) {}
}
// Exactly Surge's native 32-frame process block (SURGE_COMPILE_BLOCK_SIZE=32).
// No allocation, file I/O or synchronization is performed by this ABI layer.
HONEYDEW_EXPORT int honeydew_surge_render32(void *handle, float *left, float *right,
                            double ppq, double bpm, int playing) noexcept {
    if (!handle || !left || !right) return 0;
    try {
        auto &s = *static_cast<Instance *>(handle)->synth;
        if (std::isfinite(bpm) && bpm > 0) s.time_data.tempo = bpm;
        if (std::isfinite(ppq)) s.time_data.ppqPos = ppq;
        s.time_data.isPlaying = playing != 0;
        s.process();
        for (int i = 0; i < 32; ++i) {
            left[i] = std::isfinite(s.output[0][i]) ? s.output[0][i] : 0.f;
            right[i] = std::isfinite(s.output[1][i]) ? s.output[1][i] : 0.f;
        }
        return 1;
    } catch (...) {
        std::fill_n(left, 32, 0.f);
        std::fill_n(right, 32, 0.f);
        return 0;
    }
}
} // extern "C"
