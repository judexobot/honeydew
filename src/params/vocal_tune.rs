//! VocalTune v0.3 parameter registry, independent effect ID 52.
use super::{Display, ModuleSpec, ParamSpec as P};
const KEYS: &[&str] = &[
    "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
];
const SCALES: &[&str] = &["Chromatic", "Major", "Minor"];
const PARAMETERS: &[P] = &[
    P::ENABLE,
    P::menu("Key", 0.0, KEYS),
    P::menu("Scale", 0.0, SCALES),
    P::cont("Correction", 1.0, Display::linear(0.0, 100.0, 0, "%")),
    P::cont(
        "Retune Speed",
        0.25,
        Display::power(2.0, 300.0, 2, 0, " ms"),
    ),
    P::cont("Wet", 1.0, Display::linear(0.0, 100.0, 0, "%")),
];
pub static MODULES: &[ModuleSpec] = &[ModuleSpec::effect(
    52,
    "VocalTune",
    "VocalTune",
    false,
    PARAMETERS,
    &[],
)];
