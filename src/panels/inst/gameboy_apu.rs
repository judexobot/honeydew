//! DMG APU instrument (ID 54): 5 pages of hardware-derived sound controls.
use crate::panels::build::*;

pub fn panel() -> Option<Panel> {
    Some(Panel::new(vstack([
        tabs(1, "PULSE 1 · PULSE 2 · WAVE · NOISE · MIX").h(Size::U(1.25)),
        hstack([
            toggle(2, "CH1").p(18),
            knob(3, "Level").p(1),
            spinner(4, "Duty").p(2),
            spinner(5, "Sweep Pace").p(3),
            spinner(6, "Sweep Shift").p(4),
            toggle(7, "Sweep Down").p(5),
        ]).show(Show::Tab(0)),
        hstack([
            toggle(8, "CH2").p(19),
            knob(9, "Level").p(6),
            spinner(10, "Duty").p(7),
            spinner(11, "Envelope").p(14),
        ]).show(Show::Tab(1)),
        hstack([
            toggle(12, "CH3").p(20),
            spinner(13, "Wave Level").p(8),
            spinner(14, "Waveform").p(9),
        ]).show(Show::Tab(2)),
        hstack([
            toggle(15, "CH4").p(21),
            knob(16, "Level").p(10),
            toggle(17, "7-bit Noise").p(11),
            spinner(18, "Divisor").p(12),
            spinner(19, "Clock Shift").p(13),
        ]).show(Show::Tab(3)),
        hstack([
            spinner(20, "Master").p(15),
            spinner(21, "Stereo").p(16),
            knob(22, "Output Gain").p(17),
        ]).show(Show::Tab(4)),
    ])))
}
#[cfg(test)]
pub const REFERENCE: Reference = Reference {
    size: [14.0, 6.99], settings: &[], cases: &[],
};
