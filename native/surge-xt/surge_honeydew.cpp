// Copyright (c) the Honeydew contributors. GPL-3.0-or-later.
// This is a C ABI around *Surge XT's original C++ synthesizer*, not a DSP reimplementation.
// Build with the unmodified Surge XT source through bridge.cmake.
#include "SurgeSynthesizer.h"
#include <algorithm>
#include <cmath>
#include <cstdint>
#include <memory>
#include <new>
#include <string>

namespace {
struct Host final : SurgeSynthesizer::PluginLayer {
    void surgeParameterUpdated(const SurgeSynthesizer::ID &, float) override {}
    void surgeMacroUpdated(long, float) override {}
};
struct Instance {
    Host host;
    std::unique_ptr<SurgeSynthesizer> synth;
    explicit Instance(float rate) {
        synth = std::make_unique<SurgeSynthesizer>(
            &host, SurgeStorage::skipPatchLoadDataPathSentinel);
        synth->setSamplerate(rate);
        synth->time_data.tempo = 120.0;
        synth->time_data.ppqPos = 0.0;
    }
};
} // namespace

extern "C" {
// The caller owns the returned pointer. Construction may load Surge's internal resources,
// so it must happen off the real-time callback.
void *honeydew_surge_create(float sample_rate) noexcept {
    if (!std::isfinite(sample_rate) || sample_rate < 8000 || sample_rate > 384000) return nullptr;
    try { return new Instance(sample_rate); } catch (...) { return nullptr; }
}
void honeydew_surge_destroy(void *handle) noexcept {
    delete static_cast<Instance *>(handle);
}
void honeydew_surge_note_on(void *handle, uint8_t channel, uint8_t key, uint8_t velocity) noexcept {
    if (!handle) return;
    try {
        static_cast<Instance *>(handle)->synth->playNote(
            static_cast<char>(channel & 15), static_cast<char>(key & 127),
            static_cast<char>(velocity & 127), 0);
    } catch (...) {}
}
void honeydew_surge_note_off(void *handle, uint8_t channel, uint8_t key) noexcept {
    if (!handle) return;
    try {
        static_cast<Instance *>(handle)->synth->releaseNote(
            static_cast<char>(channel & 15), static_cast<char>(key & 127), 0);
    } catch (...) {}
}
void honeydew_surge_panic(void *handle) noexcept {
    if (!handle) return;
    try { static_cast<Instance *>(handle)->synth->allSoundOff(); } catch (...) {}
}
void honeydew_surge_all_notes_off(void *handle) noexcept {
    if (!handle) return;
    try { static_cast<Instance *>(handle)->synth->allNotesOff(); } catch (...) {}
}
void honeydew_surge_cc(void *handle, uint8_t channel, uint8_t cc, uint8_t value) noexcept {
    if (!handle) return;
    try {
        static_cast<Instance *>(handle)->synth->channelController(
            static_cast<char>(channel & 15), cc & 127, value & 127);
    } catch (...) {}
}
void honeydew_surge_pitch_bend(void *handle, uint8_t channel, int bend14) noexcept {
    if (!handle) return;
    try {
        static_cast<Instance *>(handle)->synth->pitchBend(
            static_cast<char>(channel & 15), std::clamp(bend14, 0, 16383));
    } catch (...) {}
}
void honeydew_surge_macro(void *handle, int which, float value) noexcept {
    if (!handle || which < 0 || which >= n_customcontrollers || !std::isfinite(value)) return;
    try {
        static_cast<Instance *>(handle)->synth->setMacroParameter01(
            which, std::clamp(value, 0.0f, 1.0f));
    } catch (...) {}
}
// Exactly Surge's native 32-frame process block (SURGE_COMPILE_BLOCK_SIZE=32).
// No allocation, file I/O or synchronization is performed by this ABI layer.
int honeydew_surge_render32(void *handle, float *left, float *right,
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
