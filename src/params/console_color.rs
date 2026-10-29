//! Console Color: four standalone console-output post-processing profiles (module ID 56).
use super::{Display, ModuleSpec, ParamSpec as P};
const PARAMS: &[P] = &[
    P::ENABLE, // 0
    P::menu("Console", 0.0, &["SNES", "GBA", "N64", "PS1"]), // 1
    P::cont("Bit Reduction", 0.0, Display::linear(0.0, 100.0, 0, "%")), // 2
    P::cont("Clock Reduction", 0.0, Display::linear(0.0, 75.0, 0, "%")), // 3
    P::cont("Tone", 0.5, Display::linear(0.0, 100.0, 0, "%")), // 4
    P::cont("Echo", 0.3, Display::linear(0.0, 100.0, 0, "%")), // 5
    P::cont("Feedback", 0.35, Display::linear(0.0, 100.0, 0, "%")), // 6
    P::cont("Stereo Width", 0.5, Display::linear(0.0, 200.0, 0, "%")), // 7
    P::cont("Drive", 0.12, Display::linear(0.0, 100.0, 0, "%")), // 8
    P::cont("Mix", 1.0, Display::linear(0.0, 100.0, 0, "%")), // 9
    P::cont("Output Gain", 0.5, Display::linear(-12.0, 12.0, 1, " dB")), // 10
];
pub static MODULES: &[ModuleSpec] = &[ModuleSpec::effect(
    56, "Console Color", "Console Color", false, PARAMS, &[],
)];
