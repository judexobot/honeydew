// GPL-3.0-or-later. Fixed-width C ABI shared by the native audio QA and Rust.
#pragma once
#include <stdint.h>
#define HONEYDEW_SURGE_ABI 2u
#define HONEYDEW_SURGE_BLOCK 32
#if defined(__cplusplus)
#define HONEYDEW_NOEXCEPT noexcept
extern "C" {
#else
#define HONEYDEW_NOEXCEPT
#endif
#define HONEYDEW_EXPORT __attribute__((visibility("default")))
typedef struct HoneydewSurgeState {
    uint32_t preset;
    uint32_t voices;
    uint32_t note_ons;
    uint32_t note_offs;
    uint32_t blocks;
    uint32_t faults;
    float peak;
} HoneydewSurgeState;
HONEYDEW_EXPORT uint32_t honeydew_surge_abi_version(void) HONEYDEW_NOEXCEPT;
HONEYDEW_EXPORT const char *honeydew_surge_last_error(void) HONEYDEW_NOEXCEPT;
HONEYDEW_EXPORT int honeydew_surge_set_data_path(const char *factory, const char *user) HONEYDEW_NOEXCEPT;
HONEYDEW_EXPORT const char *honeydew_surge_factory_path(int preset) HONEYDEW_NOEXCEPT;
HONEYDEW_EXPORT void *honeydew_surge_create(float rate) HONEYDEW_NOEXCEPT;
HONEYDEW_EXPORT void honeydew_surge_destroy(void *handle) HONEYDEW_NOEXCEPT;
HONEYDEW_EXPORT int honeydew_surge_load_factory(void *handle, int preset) HONEYDEW_NOEXCEPT;
HONEYDEW_EXPORT void honeydew_surge_note_on(void *handle, uint8_t channel, uint8_t key, uint8_t velocity) HONEYDEW_NOEXCEPT;
HONEYDEW_EXPORT void honeydew_surge_note_off(void *handle, uint8_t channel, uint8_t key) HONEYDEW_NOEXCEPT;
HONEYDEW_EXPORT void honeydew_surge_cc(void *handle, uint8_t channel, uint8_t cc, uint8_t value) HONEYDEW_NOEXCEPT;
// Standard MIDI unsigned 14-bit value. Converted to Surge's signed range here.
HONEYDEW_EXPORT void honeydew_surge_pitch_bend(void *handle, uint8_t channel, int bend14) HONEYDEW_NOEXCEPT;
HONEYDEW_EXPORT void honeydew_surge_aftertouch(void *handle, uint8_t channel, uint8_t pressure) HONEYDEW_NOEXCEPT;
HONEYDEW_EXPORT void honeydew_surge_all_notes_off(void *handle) HONEYDEW_NOEXCEPT;
HONEYDEW_EXPORT void honeydew_surge_panic(void *handle) HONEYDEW_NOEXCEPT;
HONEYDEW_EXPORT int honeydew_surge_render32(void *handle, float *left, float *right, double ppq, double bpm, int playing) HONEYDEW_NOEXCEPT;
HONEYDEW_EXPORT int honeydew_surge_render32_with_input(void *handle, const float *input_left, const float *input_right, float *left, float *right, double ppq, double bpm, int playing) HONEYDEW_NOEXCEPT;
HONEYDEW_EXPORT int honeydew_surge_get_state(void *handle, HoneydewSurgeState *state) HONEYDEW_NOEXCEPT;
HONEYDEW_EXPORT float honeydew_surge_probe(float rate) HONEYDEW_NOEXCEPT;
#if defined(__cplusplus)
}
#endif
