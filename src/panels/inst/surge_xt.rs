//! Native Surge XT controls backed by original C++ parameter IDs, not recreated synthesis.
use crate::panels::build::*;

pub fn panel() -> Option<Panel> {
    Some(Panel::new(vstack([
        tabs(0, "MACROS · OSC · MIX · FILTER · AMP ENV · FILT ENV · LFO · PRESETS · MIDI")
            .h(Size::U(1.25)),
        hstack([
            knob(101, "Macro 1").p(1),
            knob(102, "Macro 2").p(2),
            knob(103, "Macro 3").p(3),
            knob(104, "Macro 4").p(4),
        ]).h(Size::U(2.4)).show(Show::Tab(0)),
        hstack([
            knob(105, "Macro 5").p(5),
            knob(106, "Macro 6").p(6),
            knob(107, "Macro 7").p(7),
            knob(108, "Macro 8").p(8),
        ]).h(Size::U(2.4)).show(Show::Tab(0)),
        hstack([
            knob(109, "OSC 1 Type").p(9),
            knob(110, "OSC 2 Type").p(10),
            knob(111, "OSC 3 Type").p(11),
        ]).h(Size::U(2.4)).show(Show::Tab(1)),
        hstack([
            knob(112, "OSC 1 Pitch").p(12),
            knob(113, "OSC 1 Param").p(13),
        ]).h(Size::U(2.4)).show(Show::Tab(1)),
        hstack([
            knob(114, "OSC 1 Level").p(14),
            knob(115, "OSC 2 Level").p(15),
            knob(116, "OSC 3 Level").p(16),
            knob(132, "Master Level").p(32),
        ]).h(Size::U(2.4)).show(Show::Tab(2)),
        hstack([
            knob(117, "Filter Type").p(17),
            knob(118, "Cutoff").p(18),
            knob(119, "Resonance").p(19),
            knob(120, "Env Depth").p(20),
        ]).h(Size::U(2.4)).show(Show::Tab(3)),
        hstack([
            v_slider(121, "Attack").p(21),
            v_slider(122, "Decay").p(22),
            v_slider(123, "Sustain").p(23),
            v_slider(124, "Release").p(24),
        ]).h(Size::U(3.5)).show(Show::Tab(4)),
        hstack([
            v_slider(125, "Attack").p(25),
            v_slider(126, "Decay").p(26),
            v_slider(127, "Sustain").p(27),
            v_slider(128, "Release").p(28),
        ]).h(Size::U(3.5)).show(Show::Tab(5)),
        hstack([
            knob(129, "LFO Rate").p(29),
            knob(130, "LFO Shape").p(30),
            knob(131, "LFO Depth").p(31),
        ]).h(Size::U(2.4)).show(Show::Tab(6)),
        vstack([
            text(203, "641 ORIGINAL SURGE XT FACTORY PRESETS").h(Size::U(0.75)),
            text(204, "Tap the preset name above (Default) to browse by category.").h(Size::U(0.75)),
            text(205, "Basses, Brass, FX, Keys, Leads, Pads, Plucks, and more.").h(Size::U(0.75)),
            text(206, "These load Surge XT's original FXP sound data.").h(Size::U(0.75)),
        ]).h(Size::U(3.2)).show(Show::Tab(7)),
        vstack([
            text(200, "SURGE XT · ORIGINAL DSP").h(Size::U(0.75)),
            text(201, "Polyphonic MIDI · Pitch Bend · CC · 8 Macros").h(Size::U(0.75)),
            text(202, "Native original Surge XT C++ engine").h(Size::U(0.75)),
        ]).h(Size::U(3.2)).show(Show::Tab(8)),
    ])))
}
#[cfg(test)]
pub const REFERENCE: Reference = Reference {
    size: [0.0, 0.0],
    settings: &[],
    cases: &[],
};
