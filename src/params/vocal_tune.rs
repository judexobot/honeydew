//! VocalTune v0.4, independent vocal correction effect, type ID 53.
use super::{Display, ModuleSpec, ParamSpec as P};

const KEYS: &[&str] = &[
    "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
];
const SCALES: &[&str] = &[
    "Chromatic",
    "Major",
    "Minor",
    "Pentatonic",
    "Blues",
    "Custom",
];
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
    // Custom scale: twelve independent, automatable chromatic pitch classes.
    P::toggle("C", 1.0),  // 12
    P::toggle("C#", 1.0), // 13
    P::toggle("D", 1.0),  // 14
    P::toggle("D#", 1.0), // 15
    P::toggle("E", 1.0),  // 16
    P::toggle("F", 1.0),  // 17
    P::toggle("F#", 1.0), // 18
    P::toggle("G", 1.0),  // 19
    P::toggle("G#", 1.0), // 20
    P::toggle("A", 1.0),  // 21
    P::toggle("A#", 1.0), // 22
    P::toggle("B", 1.0),  // 23
    P::cont(
        "Protect Sibilants",
        0.75,
        Display::linear(0.0, 100.0, 0, "%"),
    ), // 24
    P::toggle("Formant Preserve", 1.0), // 25
    P::cont("Formant Shift", 0.5, Display::linear(-12.0, 12.0, 1, " st")), // 26
    P::cont(
        "Vibrato Preservation",
        0.35,
        Display::linear(0.0, 100.0, 0, "%"),
    ), // 27
    P::cont(
        "Note Transition",
        0.15,
        Display::linear(0.0, 100.0, 0, " ms"),
    ), // 28
    P::cont("Flex Tune", 0.1, Display::linear(0.0, 100.0, 0, " cents")), // 29
    P::menu("Input Range", 0.0, &["Full", "Low", "Medium", "High"]), // 30
    P::cont("Output Gain", 0.5, Display::linear(-24.0, 24.0, 1, " dB")), // 31
    P::menu("Quality", 0.0, &["Monitor", "Studio"]), // 32
];
pub static MODULES: &[ModuleSpec] = &[ModuleSpec::effect(
    53,
    "VocalTune",
    "VocalTune",
    false,
    PARAMETERS,
    &["TUNE", "NOTES", "EXPRESSION", "OUTPUT"],
)];
