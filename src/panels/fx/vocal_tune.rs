//! VocalTune ID 53: native, responsive tuning controls and live-pitch display.
use super::live_analysis;
use crate::panels::build::*;
use crate::ui::UiCtx;
use eframe::egui::{Align2, Color32, FontId, Id, Rect, Ui};
use honeydew_audio::analysis;

/// A monitor of VocalTune's own pitch detector, not a Pitcher implementation.
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
        Some((hz, Some(target), ratio)) if hz > 0.0 && hz.is_finite() => {
            const NOTES: [&str; 12] = [
                "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
            ];
            let note = target.rem_euclid(12) as usize;
            let octave = target.div_euclid(12) - 1;
            let correction = 1200.0 * ratio.max(0.0001).log2();
            format!(
                "INPUT {hz:.1} Hz     TARGET {}{octave}     SHIFT {correction:+.0} cents",
                NOTES[note]
            )
        }
        _ => "INPUT -- Hz     TARGET --     Waiting for voice".to_string(),
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
        tabs(1, "TUNE · EXPRESSION · OUTPUT").h(Size::U(1.25)),
        hstack([
            spinner(2, "Key").p(1),
            spinner(3, "Scale").p(2),
            knob(4, "Correction").p(3),
            knob(5, "Retune").p(4),
        ])
        .h(Size::U(3.74))
        .show(Show::Tab(0)),
        hstack([
            knob(6, "Humanize").p(6),
            knob(7, "Sensitivity").p(7),
            knob(8, "Deadband").p(8),
            knob(9, "Fine Tune").p(11),
        ])
        .h(Size::U(3.74))
        .show(Show::Tab(1)),
        hstack([
            knob(10, "Mix").p(5),
            knob(11, "Reference").p(9),
            toggle(12, "Hard Tune").p(10),
        ])
        .h(Size::U(3.74))
        .show(Show::Tab(2)),
        display(13).h(Size::U(2.0)).draw(live_pitch),
    ])))
}

#[cfg(test)]
const TABS: Row = row(
    1,
    Widget::Tabs,
    "TUNE · EXPRESSION · OUTPUT",
    [0.0, 0.0, 14.0, 1.25],
)
.h(Size::U(1.25));

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
                row(13, Widget::Display, "", [0.0, 4.99, 14.0, 2.0]),
            ],
            ..Case::DEFAULT
        },
        Case {
            name: "EXPRESSION",
            tab: 1,
            rows: &[
                TABS,
                row(6, Widget::Knob, "Humanize", [0.0, 1.25, 3.5, 3.74]).p(6),
                row(7, Widget::Knob, "Sensitivity", [3.5, 1.25, 3.5, 3.74]).p(7),
                row(8, Widget::Knob, "Deadband", [7.0, 1.25, 3.5, 3.74]).p(8),
                row(9, Widget::Knob, "Fine Tune", [10.5, 1.25, 3.5, 3.74]).p(11),
                row(13, Widget::Display, "", [0.0, 4.99, 14.0, 2.0]),
            ],
            ..Case::DEFAULT
        },
        Case {
            name: "OUTPUT",
            tab: 2,
            rows: &[
                TABS,
                row(10, Widget::Knob, "Mix", [0.0, 1.25, 4.66, 3.74]).p(5),
                row(11, Widget::Knob, "Reference", [4.66, 1.25, 4.67, 3.74]).p(9),
                row(12, Widget::Toggle, "Hard Tune", [9.33, 1.25, 4.67, 3.74]).p(10),
                row(13, Widget::Display, "", [0.0, 4.99, 14.0, 2.0]),
            ],
            ..Case::DEFAULT
        },
    ],
};
