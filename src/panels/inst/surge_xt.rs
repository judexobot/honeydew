//! Touch-oriented Surge XT editor. This is a native Honeydew control surface for
//! the *original Surge XT C++ DSP*, not a separate or imitated synthesizer.
//! Layout mirrors Surge XT's main signal flow: oscillators, mixer, filters,
//! envelopes, modulation and macros, with a categorized factory patch browser.
use crate::panels::build::*;
use crate::ui::UiCtx;
use eframe::egui::{Color32, Id, Rect, Ui, pos2};

/// Runtime audio diagnostics are drawn on the actual instrument card; silently
/// omitting the instrument from a failed engine build is never treated as success.
fn diagnostics(
    ui: &Ui,
    cx: &UiCtx,
    _: &ModuleInstance,
    _: &PanelState,
    _: &[(u32, Rect)],
    area: Rect,
    _: Color32,
) {
    let (message, color) = match honeydew_audio::surge_xt::diagnostic() {
        2 => ("SURGE XT: DSP PCM TEST PASSED", Color32::from_rgb(108, 214, 151)),
        -1 => ("SURGE XT ERROR: NATIVE LIBRARY NOT LOADED", Color32::from_rgb(255, 115, 94)),
        -2 => ("SURGE XT ERROR: C++ API SYMBOL MISSING", Color32::from_rgb(255, 115, 94)),
        -3 => ("SURGE XT ERROR: SYNTH INITIALIZATION FAILED", Color32::from_rgb(255, 115, 94)),
        -4 => ("SURGE XT ERROR: FACTORY PRESET DID NOT LOAD", Color32::from_rgb(255, 115, 94)),
        -5 => ("SURGE XT ERROR: ORIGINAL DSP OUTPUT IS SILENT", Color32::from_rgb(255, 115, 94)),
        _ => ("SURGE XT: CHECKING NATIVE AUDIO ENGINE", Color32::LIGHT_GRAY),
    };
    ui.painter().text(
        pos2(area.left() + cx.m.u(0.35), area.bottom() - cx.m.u(0.12)),
        Align2::LEFT_BOTTOM,
        message,
        cx.m.font_id(FontSize::Pt8),
        color,
    );
}

pub fn panel() -> Option<Panel> {
    Some(Panel::new(vstack([
        tabs(0, "SCENE A · OSC · MIX · FILTER · AMP · FENV · LFO · MACROS · BANK · MIDI")
            .h(Size::U(1.25)),
        // A compact signal-flow overview like the original Surge XT editor.
        vstack([
            hstack([
                vstack([
                    text(1100, "OSCILLATORS").h(Size::U(0.65)),
                    hstack([
                        knob(1101, "OSC 1 Type").p(9),
                        knob(1102, "OSC 2 Type").p(10),
                        knob(1103, "OSC 3 Type").p(11),
                    ]),
                ]),
                vstack([
                    text(1110, "FILTER 1").h(Size::U(0.65)),
                    hstack([
                        knob(1111, "Cutoff").p(18),
                        knob(1112, "Res").p(19),
                        knob(1113, "Env").p(20),
                    ]),
                ]),
            ]).h(Size::U(2.4)),
            hstack([
                vstack([
                    text(1120, "OSC MIXER").h(Size::U(0.65)),
                    hstack([
                        knob(1121, "Osc 1").p(14),
                        knob(1122, "Osc 2").p(15),
                        knob(1123, "Osc 3").p(16),
                    ]),
                ]),
                vstack([
                    text(1130, "AMP ENVELOPE").h(Size::U(0.65)),
                    hstack([
                        knob(1131, "A").p(21),
                        knob(1132, "D").p(22),
                        knob(1133, "S").p(23),
                        knob(1134, "R").p(24),
                    ]),
                ]),
            ]).h(Size::U(2.4)),
            hstack([
                knob(1140, "Macro 1").p(1),
                knob(1141, "Macro 2").p(2),
                knob(1142, "Macro 3").p(3),
                knob(1143, "Macro 4").p(4),
            ]).h(Size::U(2.0)),
        ]).show(Show::Tab(0)),
        vstack([
            hstack([
                knob(1201, "OSC 1 Type").p(9),
                knob(1202, "OSC 2 Type").p(10),
                knob(1203, "OSC 3 Type").p(11),
            ]).h(Size::U(2.4)),
            hstack([
                knob(1204, "Pitch").p(12),
                knob(1205, "Shape").p(13),
            ]).h(Size::U(2.4)),
        ]).show(Show::Tab(1)),
        hstack([
            knob(1301, "OSC 1").p(14),
            knob(1302, "OSC 2").p(15),
            knob(1303, "OSC 3").p(16),
            knob(1304, "Master").p(32),
        ]).h(Size::U(2.7)).show(Show::Tab(2)),
        hstack([
            knob(1401, "Type").p(17),
            knob(1402, "Cutoff").p(18),
            knob(1403, "Resonance").p(19),
            knob(1404, "Env depth").p(20),
        ]).h(Size::U(2.7)).show(Show::Tab(3)),
        hstack([
            v_slider(1501, "Attack").p(21),
            v_slider(1502, "Decay").p(22),
            v_slider(1503, "Sustain").p(23),
            v_slider(1504, "Release").p(24),
        ]).h(Size::U(4.5)).show(Show::Tab(4)),
        hstack([
            v_slider(1601, "Attack").p(25),
            v_slider(1602, "Decay").p(26),
            v_slider(1603, "Sustain").p(27),
            v_slider(1604, "Release").p(28),
        ]).h(Size::U(4.5)).show(Show::Tab(5)),
        hstack([
            knob(1701, "Rate").p(29),
            knob(1702, "Shape").p(30),
            knob(1703, "Depth").p(31),
        ]).h(Size::U(2.7)).show(Show::Tab(6)),
        vstack([
            hstack([
                knob(1801, "Macro 1").p(1),
                knob(1802, "Macro 2").p(2),
                knob(1803, "Macro 3").p(3),
                knob(1804, "Macro 4").p(4),
            ]).h(Size::U(2.5)),
            hstack([
                knob(1805, "Macro 5").p(5),
                knob(1806, "Macro 6").p(6),
                knob(1807, "Macro 7").p(7),
                knob(1808, "Macro 8").p(8),
            ]).h(Size::U(2.5)),
        ]).show(Show::Tab(7)),
        vstack([
            text(1901, "SURGE XT ORIGINAL FACTORY · 641 PATCHES").h(Size::U(0.85)),
            text(1902, "Tap the preset name at the top of the rack.").h(Size::U(0.85)),
            text(1903, "Browse by Basses, Leads, Pads, Plucks, Keys, FX...").h(Size::U(0.85)),
            text(1904, "Selected patches load original Surge XT FXP data.").h(Size::U(0.85)),
        ]).h(Size::U(3.8)).show(Show::Tab(8)),
        vstack([
            text(2001, "ORIGINAL SURGE XT SYNTHESIZER").h(Size::U(0.9)),
            text(2002, "MIDI notes, pitch bend, CC and 8 macros").h(Size::U(0.9)),
            text(2003, "Tap a piano key to trigger the original engine.").h(Size::U(0.9)),
        ]).h(Size::U(3.5)).show(Show::Tab(9)),
    ])).overlay(diagnostics))
}

#[cfg(test)]
pub const REFERENCE: Reference = Reference {
    size: [0.0, 0.0],
    settings: &[],
    cases: &[],
};
