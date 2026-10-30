//! Surge XT editor embedded into Honeydew's touch rack.
//! These eight knobs are the *real* Surge XT macro slots, not surrogate synth controls.
use crate::panels::build::*;

pub fn panel() -> Option<Panel> {
    Some(Panel::new(vstack([
        tabs(0, "MACROS · MIDI").h(Size::U(1.25)),
        hstack([
            knob(101, "Macro 1").p(1),
            knob(102, "Macro 2").p(2),
            knob(103, "Macro 3").p(3),
            knob(104, "Macro 4").p(4),
        ])
        .h(Size::U(2.4))
        .show(Show::Tab(0)),
        hstack([
            knob(105, "Macro 5").p(5),
            knob(106, "Macro 6").p(6),
            knob(107, "Macro 7").p(7),
            knob(108, "Macro 8").p(8),
        ])
        .h(Size::U(2.4))
        .show(Show::Tab(0)),
        vstack([
            text(200, "SURGE XT · ORIGINAL DSP").h(Size::U(0.75)),
            text(201, "Polyphonic MIDI · Pitch Bend · CC · 8 Macros").h(Size::U(0.75)),
            text(202, "Audio engine: native Surge XT C++").h(Size::U(0.75)),
        ])
        .h(Size::U(3.2))
        .show(Show::Tab(1)),
    ])))
}

#[cfg(test)]
pub const REFERENCE: Reference = Reference {
    size: [0.0, 0.0],
    settings: &[],
    cases: &[],
};
