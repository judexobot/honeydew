//! VocalTune: parameter registry for the independent vocal tuning effect, ID 53.
use super::{Display, ModuleSpec, ParamSpec as P};

const KEYS: &[&str] = &[
    "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
];
const SCALES: &[&str] = &["Chromatic", "Major", "Minor", "Pentatonic", "Blues"];
const PARAMETERS: &[P] = &[
    P::ENABLE,                                                       // 0
    P::menu("Key", 0.0, KEYS),                                       // 1
    P::menu("Scale", 0.0, SCALES),                                   // 2
    P::cont("Correction", 1.0, Display::linear(0.0, 100.0, 0, "%")), // 3
    P::cont(
        "Retune Speed",
        0.25,
        Display::power(2.0, 300.0, 2, 0, " ms"),
    ), // 4
    P::cont("Mix", 1.0, Display::linear(0.0, 100.0, 0, "%")),        // 5
    P::cont("Humanize", 0.2, Display::linear(0.0, 100.0, 0, "%")),   // 6
    P::cont("Sensitivity", 0.5, Display::linear(0.0, 100.0, 0, "%")), // 7
    P::cont("Deadband", 0.1, Display::linear(0.0, 50.0, 0, " cents")), // 8
    P::cont("Reference", 0.5, Display::linear(430.0, 450.0, 0, " Hz")), // 9
    P::toggle("Hard Tune", 0.0),                                     // 10
    P::cont("Fine Tune", 0.5, Display::linear(-50.0, 50.0, 0, " cents")), // 11
];
pub static MODULES: &[ModuleSpec] = &[ModuleSpec::effect(
    53,
    "VocalTune",
    "VocalTune",
    false,
    PARAMETERS,
    &["TUNE", "EXPRESSION", "OUTPUT"],
)];
