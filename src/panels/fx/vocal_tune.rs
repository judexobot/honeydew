//! VocalTune: touch-friendly four-page vocal tuning panel, type ID 53.
use super::live_analysis;
use crate::panels::build::*;
use crate::ui::UiCtx;
use eframe::egui::{Align2, Color32, FontId, Id, Rect, Ui};
use honeydew_audio::analysis;

/// Read the independent VocalTune DSP's real-time pitch and clarity.
fn live_pitch(
    ui: &mut Ui,
    cx: &UiCtx,
    _: Id,
    rect: Rect,
    module: &mut ModuleInstance,
    _: &mut PanelState,
    ink: Color32,
) -> Vec<usize> {
    let feed = live_analysis(ui, module.instance_id).and_then(|s| analysis::vocal_tune(&s));
    let caption = match feed {
        Some((hz, Some(target), ratio, clarity)) if hz > 0.0 && hz.is_finite() => {
            const NOTES: [&str; 12] = [
                "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
            ];
            let note = target.rem_euclid(12) as usize;
            let octave = target.div_euclid(12) - 1;
            let correction = 1200.0 * ratio.max(0.0001).log2();
            format!(
                "IN {hz:.1} Hz   TO {}{octave}   SHIFT {correction:+.0}c   CONF {:.0}%",
                NOTES[note],
                clarity * 100.0
            )
        }
        _ => "IN -- Hz   TARGET --   Waiting for voiced audio".to_string(),
    };
    ui.painter()
        .rect_filled(rect, 4.0, cx.theme.colors.control_fill.to_color32());
    ui.painter().text(
        rect.center(),
        Align2::CENTER_CENTER,
        caption,
        FontId::proportional(cx.m.u(0.35)),
        ink,
    );
    Vec::new()
}

pub fn panel() -> Option<Panel> {
    Some(Panel::new(vstack([
        tabs(1, "TUNE · NOTES · EXPRESSION · OUTPUT").h(Size::U(1.25)),
        hstack([
            spinner(2, "Key").p(1),
            spinner(3, "Scale").p(2),
            knob(4, "Correction").p(3),
            knob(5, "Retune").p(4),
        ])
        .h(Size::U(3.74))
        .show(Show::Tab(0)),
        vstack([
            hstack([
                toggle(14, "C").p(12),
                toggle(15, "C#").p(13),
                toggle(16, "D").p(14),
                toggle(17, "D#").p(15),
                toggle(18, "E").p(16),
                toggle(19, "F").p(17),
            ])
            .h(Size::U(1.87)),
            hstack([
                toggle(20, "F#").p(18),
                toggle(21, "G").p(19),
                toggle(22, "G#").p(20),
                toggle(23, "A").p(21),
                toggle(24, "A#").p(22),
                toggle(25, "B").p(23),
            ])
            .h(Size::U(1.87)),
        ])
        .h(Size::U(3.74))
        .show(Show::Tab(1)),
        hstack([
            knob(6, "Humanize").p(6),
            knob(7, "Sensitivity").p(7),
            knob(8, "Deadband").p(8),
            knob(26, "Protect").p(24),
        ])
        .h(Size::U(3.74))
        .show(Show::Tab(2)),
        hstack([
            knob(10, "Mix").p(5),
            knob(11, "Reference").p(9),
            toggle(12, "Hard Tune").p(10),
            knob(9, "Fine Tune").p(11),
        ])
        .h(Size::U(3.74))
        .show(Show::Tab(3)),
        display(13).h(Size::U(2.0)).draw(live_pitch),
    ])))
}

#[cfg(test)]
const TABS: Row = row(
    1,
    Widget::Tabs,
    "TUNE · NOTES · EXPRESSION · OUTPUT",
    [0.0, 0.0, 14.0, 1.25],
)
.h(Size::U(1.25));
#[cfg(test)]
const MONITOR: Row = row(13, Widget::Display, "", [0.0, 4.99, 14.0, 2.0]);

#[cfg(test)]
pub const REFERENCE: Reference = Reference {
    size: [14.0, 6.99],
    settings: &[],
    cases: &[
        Case {
            name: "TUNE",
            rows: &[
                TABS,
                row(2, Widget::Spinner, "Key", [0.0, 1.25, 3.5, 3.74]).p(1),
                row(3, Widget::Spinner, "Scale", [3.5, 1.25, 3.5, 3.74]).p(2),
                row(4, Widget::Knob, "Correction", [7.0, 1.25, 3.5, 3.74]).p(3),
                row(5, Widget::Knob, "Retune", [10.5, 1.25, 3.5, 3.74]).p(4),
                MONITOR,
            ],
            ..Case::DEFAULT
        },
        Case {
            name: "NOTES",
            tab: 1,
            rows: &[
                TABS,
                row(14, Widget::Toggle, "C", [0.0, 1.25, 2.33, 1.87]).p(12),
                row(15, Widget::Toggle, "C#", [2.33, 1.25, 2.33, 1.87]).p(13),
                row(16, Widget::Toggle, "D", [4.66, 1.25, 2.34, 1.87]).p(14),
                row(17, Widget::Toggle, "D#", [7.0, 1.25, 2.33, 1.87]).p(15),
                row(18, Widget::Toggle, "E", [9.33, 1.25, 2.33, 1.87]).p(16),
                row(19, Widget::Toggle, "F", [11.66, 1.25, 2.34, 1.87]).p(17),
                row(20, Widget::Toggle, "F#", [0.0, 3.12, 2.33, 1.87]).p(18),
                row(21, Widget::Toggle, "G", [2.33, 3.12, 2.33, 1.87]).p(19),
                row(22, Widget::Toggle, "G#", [4.66, 3.12, 2.34, 1.87]).p(20),
                row(23, Widget::Toggle, "A", [7.0, 3.12, 2.33, 1.87]).p(21),
                row(24, Widget::Toggle, "A#", [9.33, 3.12, 2.33, 1.87]).p(22),
                row(25, Widget::Toggle, "B", [11.66, 3.12, 2.34, 1.87]).p(23),
                MONITOR,
            ],
            ..Case::DEFAULT
        },
        Case {
            name: "EXPRESSION",
            tab: 2,
            rows: &[
                TABS,
                row(6, Widget::Knob, "Humanize", [0.0, 1.25, 3.5, 3.74]).p(6),
                row(7, Widget::Knob, "Sensitivity", [3.5, 1.25, 3.5, 3.74]).p(7),
                row(8, Widget::Knob, "Deadband", [7.0, 1.25, 3.5, 3.74]).p(8),
                row(26, Widget::Knob, "Protect", [10.5, 1.25, 3.5, 3.74]).p(24),
                MONITOR,
            ],
            ..Case::DEFAULT
        },
        Case {
            name: "OUTPUT",
            tab: 3,
            rows: &[
                TABS,
                row(10, Widget::Knob, "Mix", [0.0, 1.25, 3.5, 3.74]).p(5),
                row(11, Widget::Knob, "Reference", [3.5, 1.25, 3.5, 3.74]).p(9),
                row(12, Widget::Toggle, "Hard Tune", [7.0, 1.25, 3.5, 3.74]).p(10),
                row(9, Widget::Knob, "Fine Tune", [10.5, 1.25, 3.5, 3.74]).p(11),
                MONITOR,
            ],
            ..Case::DEFAULT
        },
    ],
};
