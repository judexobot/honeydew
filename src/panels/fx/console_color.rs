//! Console Color: a touch-sized, two-row output-coloration effect (module ID 56).
use crate::panels::build::*;

pub fn panel() -> Option<Panel> {
    Some(Panel::new(vstack([
        hstack([
            spinner(1, "Console").p(1),
            knob(2, "Bit Reduction").p(2),
            knob(3, "Clock Reduction").p(3),
            knob(4, "Tone").p(4),
            knob(5, "Echo").p(5),
        ]).h(Size::U(3.5)),
        hstack([
            knob(6, "Feedback").p(6),
            knob(7, "Stereo Width").p(7),
            knob(8, "Drive").p(8),
            knob(9, "Mix").p(9),
            knob(10, "Output Gain").p(10),
        ]).h(Size::U(3.5)),
    ])))
}
#[cfg(test)]
pub const REFERENCE: Reference = Reference {
    size: [14.0, 7.0],
    settings: &[],
    cases: &[Case {
        name: "Console Color",
        rows: &[
            row(1, Widget::Spinner, "Console", [0.0, 0.0, 2.8, 3.5]).p(1),
            row(2, Widget::Knob, "Bit Reduction", [2.8, 0.0, 2.8, 3.5]).p(2),
            row(3, Widget::Knob, "Clock Reduction", [5.6, 0.0, 2.8, 3.5]).p(3),
            row(4, Widget::Knob, "Tone", [8.4, 0.0, 2.8, 3.5]).p(4),
            row(5, Widget::Knob, "Echo", [11.2, 0.0, 2.8, 3.5]).p(5),
            row(6, Widget::Knob, "Feedback", [0.0, 3.5, 2.8, 3.5]).p(6),
            row(7, Widget::Knob, "Stereo Width", [2.8, 3.5, 2.8, 3.5]).p(7),
            row(8, Widget::Knob, "Drive", [5.6, 3.5, 2.8, 3.5]).p(8),
            row(9, Widget::Knob, "Mix", [8.4, 3.5, 2.8, 3.5]).p(9),
            row(10, Widget::Knob, "Output Gain", [11.2, 3.5, 2.8, 3.5]).p(10),
        ],
        ..Case::DEFAULT
    }],
};
