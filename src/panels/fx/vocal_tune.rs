//! VocalTune v0.3 panel, effect ID 52.
use crate::panels::build::*;
pub fn panel() -> Option<Panel> {
    Some(Panel::new(hstack([
        knob(1, "Key").p(1),
        knob(2, "Scale").p(2),
        knob(3, "Correction").p(3),
        knob(4, "Retune").p(4),
        knob(5, "Wet").p(5),
    ])))
}
#[cfg(test)]
#[rustfmt::skip]
pub const REFERENCE: Reference = Reference {
    size: [17.5, 3.49],
    settings: &[],
    cases: &[Case {
        rows: &[
            row(1, Widget::Knob, "Key", [0.0, 0.0, 3.5, 3.49]).p(1),
            row(2, Widget::Knob, "Scale", [3.5, 0.0, 3.5, 3.49]).p(2),
            row(3, Widget::Knob, "Correction", [7.0, 0.0, 3.5, 3.49]).p(3),
            row(4, Widget::Knob, "Retune", [10.5, 0.0, 3.5, 3.49]).p(4),
            row(5, Widget::Knob, "Wet", [14.0, 0.0, 3.5, 3.49]).p(5),
        ],
        ..Case::DEFAULT
    }],
};
