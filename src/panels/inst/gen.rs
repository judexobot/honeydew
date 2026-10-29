//! Gen: FM/PSG synthesis control surface for Android and desktop (instrument ID 55).
use crate::panels::build::*;
use crate::ui::UiCtx;
use eframe::egui::{Align2, Color32, FontId, Id, Rect, Ui, vec2};
fn status(
    ui: &mut Ui,
    cx: &UiCtx,
    _: Id,
    rect: Rect,
    module: &mut ModuleInstance,
    _: &mut PanelState,
    ink: Color32,
) -> Vec<usize> {
    let data = crate::panels::fx::live_analysis(ui, module.instance_id).unwrap_or_default();
    const LABELS: [&str; 10] = ["FM 1", "FM 2", "FM 3", "FM 4", "FM 5", "FM 6", "PSG A", "PSG B", "PSG C", "NOISE"];
    let painter = ui.painter_at(rect);
    for (i, name) in LABELS.iter().enumerate() {
        let width = rect.width() / LABELS.len() as f32;
        let cell = Rect::from_min_size(rect.min + vec2(i as f32 * width, 0.0), vec2(width - 1.0, rect.height()));
        let playing = data.get(i).is_some_and(|x| *x > 0.5);
        painter.rect_filled(cell, 3.0, ink.gamma_multiply(if playing { 0.22 } else { 0.06 }));
        painter.text(cell.center(), Align2::CENTER_CENTER, *name, FontId::proportional(cx.m.u(0.27)),
            ink.gamma_multiply(if playing { 1.0 } else { 0.48 }));
    }
    Vec::new()
}
fn pair_row(a: [Part; 4]) -> Part { hstack(a).h(Size::U(3.2)) }
pub fn panel() -> Option<Panel> {
    Some(Panel::new(vstack([
        tabs(1, "FM · OP1 · OP2 · OP3 · OP4 · LFO · PSG · OUTPUT").h(Size::U(1.6)),
        pair_row([
            spinner(2, "Algorithm").p(1),
            knob(3, "Feedback").p(2),
            knob(4, "FM Level").p(4),
            knob(5, "Master").p(3),
        ]).show(Show::Tab(0)),
        pair_row([
            knob(6, "Drive").p(9),
            knob(7, "DAC Color").p(10),
            knob(8, "Stereo Width").p(11),
            spinner(9, "Engine").p(54),
        ]).show(Show::Tab(0)),
        pair_row([
            spinner(10, "Ratio").p(12),
            knob(11, "Detune").p(13),
            knob(12, "Level").p(14),
            knob(13, "Attack").p(15),
        ]).show(Show::Tab(1)),
        pair_row([
            knob(14, "Decay").p(16),
            knob(15, "Sustain").p(17),
            knob(16, "Release").p(18),
            knob(17, "Amp Mod").p(19),
        ]).show(Show::Tab(1)),
        pair_row([
            spinner(18, "Ratio").p(20),
            knob(19, "Detune").p(21),
            knob(20, "Level").p(22),
            knob(21, "Attack").p(23),
        ]).show(Show::Tab(2)),
        pair_row([
            knob(22, "Decay").p(24),
            knob(23, "Sustain").p(25),
            knob(24, "Release").p(26),
            knob(25, "Amp Mod").p(27),
        ]).show(Show::Tab(2)),
        pair_row([
            spinner(26, "Ratio").p(28),
            knob(27, "Detune").p(29),
            knob(28, "Level").p(30),
            knob(29, "Attack").p(31),
        ]).show(Show::Tab(3)),
        pair_row([
            knob(30, "Decay").p(32),
            knob(31, "Sustain").p(33),
            knob(32, "Release").p(34),
            knob(33, "Amp Mod").p(35),
        ]).show(Show::Tab(3)),
        pair_row([
            spinner(34, "Ratio").p(36),
            knob(35, "Detune").p(37),
            knob(36, "Level").p(38),
            knob(37, "Attack").p(39),
        ]).show(Show::Tab(4)),
        pair_row([
            knob(38, "Decay").p(40),
            knob(39, "Sustain").p(41),
            knob(40, "Release").p(42),
            knob(41, "Amp Mod").p(43),
        ]).show(Show::Tab(4)),
        pair_row([
            knob(42, "LFO Rate").p(6),
            knob(43, "Pitch LFO").p(7),
            knob(44, "Amp LFO").p(8),
            knob(45, "Portamento").p(52),
        ]).show(Show::Tab(5)),
        pair_row([
            knob(46, "PSG A").p(44),
            knob(47, "PSG B").p(45),
            knob(48, "PSG C").p(46),
            knob(49, "Noise").p(47),
        ]).show(Show::Tab(6)),
        pair_row([
            spinner(50, "Noise Type").p(48),
            knob(51, "Noise Rate").p(49),
            knob(52, "PSG Spread").p(50),
            knob(53, "PSG Mix").p(5),
        ]).show(Show::Tab(6)),
        pair_row([
            spinner(54, "Engine").p(54),
            knob(55, "Output Trim").p(55),
            knob(56, "Pitch Bend").p(51),
            knob(57, "Release Tail").p(53),
        ]).show(Show::Tab(7)),
        display(58).h(Size::U(1.2)).draw(status),
    ])))
}
#[cfg(test)]
pub const REFERENCE: Reference = Reference {
    size: [14.0, 9.2],
    settings: &[],
    cases: &[
        Case { name: "FM", tab: 0, rows: &[
                row(1, Widget::Tabs, "FM · OP1 · OP2 · OP3 · OP4 · LFO · PSG · OUTPUT", [0.0, 0.0, 14.0, 1.6]).h(Size::U(1.6)),
                row(2, Widget::Spinner, "Algorithm", [0.0, 1.6, 3.5, 3.2]).p(1),
                row(3, Widget::Knob, "Feedback", [3.5, 1.6, 3.5, 3.2]).p(2),
                row(4, Widget::Knob, "FM Level", [7.0, 1.6, 3.5, 3.2]).p(4),
                row(5, Widget::Knob, "Master", [10.5, 1.6, 3.5, 3.2]).p(3),
                row(6, Widget::Knob, "Drive", [0.0, 4.8, 3.5, 3.2]).p(9),
                row(7, Widget::Knob, "DAC Color", [3.5, 4.8, 3.5, 3.2]).p(10),
                row(8, Widget::Knob, "Stereo Width", [7.0, 4.8, 3.5, 3.2]).p(11),
                row(9, Widget::Spinner, "Engine", [10.5, 4.8, 3.5, 3.2]).p(54),
                row(58, Widget::Display, "", [0.0, 8.0, 14.0, 1.2]).h(Size::U(1.2)),
            ], ..Case::DEFAULT },
        Case { name: "OP1", tab: 1, rows: &[
                row(1, Widget::Tabs, "FM · OP1 · OP2 · OP3 · OP4 · LFO · PSG · OUTPUT", [0.0, 0.0, 14.0, 1.6]).h(Size::U(1.6)),
                row(10, Widget::Spinner, "Ratio", [0.0, 1.6, 3.5, 3.2]).p(12),
                row(11, Widget::Knob, "Detune", [3.5, 1.6, 3.5, 3.2]).p(13),
                row(12, Widget::Knob, "Level", [7.0, 1.6, 3.5, 3.2]).p(14),
                row(13, Widget::Knob, "Attack", [10.5, 1.6, 3.5, 3.2]).p(15),
                row(14, Widget::Knob, "Decay", [0.0, 4.8, 3.5, 3.2]).p(16),
                row(15, Widget::Knob, "Sustain", [3.5, 4.8, 3.5, 3.2]).p(17),
                row(16, Widget::Knob, "Release", [7.0, 4.8, 3.5, 3.2]).p(18),
                row(17, Widget::Knob, "Amp Mod", [10.5, 4.8, 3.5, 3.2]).p(19),
                row(58, Widget::Display, "", [0.0, 8.0, 14.0, 1.2]).h(Size::U(1.2)),
            ], ..Case::DEFAULT },
        Case { name: "OP2", tab: 2, rows: &[
                row(1, Widget::Tabs, "FM · OP1 · OP2 · OP3 · OP4 · LFO · PSG · OUTPUT", [0.0, 0.0, 14.0, 1.6]).h(Size::U(1.6)),
                row(18, Widget::Spinner, "Ratio", [0.0, 1.6, 3.5, 3.2]).p(20),
                row(19, Widget::Knob, "Detune", [3.5, 1.6, 3.5, 3.2]).p(21),
                row(20, Widget::Knob, "Level", [7.0, 1.6, 3.5, 3.2]).p(22),
                row(21, Widget::Knob, "Attack", [10.5, 1.6, 3.5, 3.2]).p(23),
                row(22, Widget::Knob, "Decay", [0.0, 4.8, 3.5, 3.2]).p(24),
                row(23, Widget::Knob, "Sustain", [3.5, 4.8, 3.5, 3.2]).p(25),
                row(24, Widget::Knob, "Release", [7.0, 4.8, 3.5, 3.2]).p(26),
                row(25, Widget::Knob, "Amp Mod", [10.5, 4.8, 3.5, 3.2]).p(27),
                row(58, Widget::Display, "", [0.0, 8.0, 14.0, 1.2]).h(Size::U(1.2)),
            ], ..Case::DEFAULT },
        Case { name: "OP3", tab: 3, rows: &[
                row(1, Widget::Tabs, "FM · OP1 · OP2 · OP3 · OP4 · LFO · PSG · OUTPUT", [0.0, 0.0, 14.0, 1.6]).h(Size::U(1.6)),
                row(26, Widget::Spinner, "Ratio", [0.0, 1.6, 3.5, 3.2]).p(28),
                row(27, Widget::Knob, "Detune", [3.5, 1.6, 3.5, 3.2]).p(29),
                row(28, Widget::Knob, "Level", [7.0, 1.6, 3.5, 3.2]).p(30),
                row(29, Widget::Knob, "Attack", [10.5, 1.6, 3.5, 3.2]).p(31),
                row(30, Widget::Knob, "Decay", [0.0, 4.8, 3.5, 3.2]).p(32),
                row(31, Widget::Knob, "Sustain", [3.5, 4.8, 3.5, 3.2]).p(33),
                row(32, Widget::Knob, "Release", [7.0, 4.8, 3.5, 3.2]).p(34),
                row(33, Widget::Knob, "Amp Mod", [10.5, 4.8, 3.5, 3.2]).p(35),
                row(58, Widget::Display, "", [0.0, 8.0, 14.0, 1.2]).h(Size::U(1.2)),
            ], ..Case::DEFAULT },
        Case { name: "OP4", tab: 4, rows: &[
                row(1, Widget::Tabs, "FM · OP1 · OP2 · OP3 · OP4 · LFO · PSG · OUTPUT", [0.0, 0.0, 14.0, 1.6]).h(Size::U(1.6)),
                row(34, Widget::Spinner, "Ratio", [0.0, 1.6, 3.5, 3.2]).p(36),
                row(35, Widget::Knob, "Detune", [3.5, 1.6, 3.5, 3.2]).p(37),
                row(36, Widget::Knob, "Level", [7.0, 1.6, 3.5, 3.2]).p(38),
                row(37, Widget::Knob, "Attack", [10.5, 1.6, 3.5, 3.2]).p(39),
                row(38, Widget::Knob, "Decay", [0.0, 4.8, 3.5, 3.2]).p(40),
                row(39, Widget::Knob, "Sustain", [3.5, 4.8, 3.5, 3.2]).p(41),
                row(40, Widget::Knob, "Release", [7.0, 4.8, 3.5, 3.2]).p(42),
                row(41, Widget::Knob, "Amp Mod", [10.5, 4.8, 3.5, 3.2]).p(43),
                row(58, Widget::Display, "", [0.0, 8.0, 14.0, 1.2]).h(Size::U(1.2)),
            ], ..Case::DEFAULT },
        Case { name: "LFO", tab: 5, rows: &[
                row(1, Widget::Tabs, "FM · OP1 · OP2 · OP3 · OP4 · LFO · PSG · OUTPUT", [0.0, 0.0, 14.0, 1.6]).h(Size::U(1.6)),
                row(42, Widget::Knob, "LFO Rate", [0.0, 1.6, 3.5, 3.2]).p(6),
                row(43, Widget::Knob, "Pitch LFO", [3.5, 1.6, 3.5, 3.2]).p(7),
                row(44, Widget::Knob, "Amp LFO", [7.0, 1.6, 3.5, 3.2]).p(8),
                row(45, Widget::Knob, "Portamento", [10.5, 1.6, 3.5, 3.2]).p(52),
                row(58, Widget::Display, "", [0.0, 4.8, 14.0, 1.2]).h(Size::U(1.2)),
            ], ..Case::DEFAULT },
        Case { name: "PSG", tab: 6, rows: &[
                row(1, Widget::Tabs, "FM · OP1 · OP2 · OP3 · OP4 · LFO · PSG · OUTPUT", [0.0, 0.0, 14.0, 1.6]).h(Size::U(1.6)),
                row(46, Widget::Knob, "PSG A", [0.0, 1.6, 3.5, 3.2]).p(44),
                row(47, Widget::Knob, "PSG B", [3.5, 1.6, 3.5, 3.2]).p(45),
                row(48, Widget::Knob, "PSG C", [7.0, 1.6, 3.5, 3.2]).p(46),
                row(49, Widget::Knob, "Noise", [10.5, 1.6, 3.5, 3.2]).p(47),
                row(50, Widget::Spinner, "Noise Type", [0.0, 4.8, 3.5, 3.2]).p(48),
                row(51, Widget::Knob, "Noise Rate", [3.5, 4.8, 3.5, 3.2]).p(49),
                row(52, Widget::Knob, "PSG Spread", [7.0, 4.8, 3.5, 3.2]).p(50),
                row(53, Widget::Knob, "PSG Mix", [10.5, 4.8, 3.5, 3.2]).p(5),
                row(58, Widget::Display, "", [0.0, 8.0, 14.0, 1.2]).h(Size::U(1.2)),
            ], ..Case::DEFAULT },
        Case { name: "OUTPUT", tab: 7, rows: &[
                row(1, Widget::Tabs, "FM · OP1 · OP2 · OP3 · OP4 · LFO · PSG · OUTPUT", [0.0, 0.0, 14.0, 1.6]).h(Size::U(1.6)),
                row(54, Widget::Spinner, "Engine", [0.0, 1.6, 3.5, 3.2]).p(54),
                row(55, Widget::Knob, "Output Trim", [3.5, 1.6, 3.5, 3.2]).p(55),
                row(56, Widget::Knob, "Pitch Bend", [7.0, 1.6, 3.5, 3.2]).p(51),
                row(57, Widget::Knob, "Release Tail", [10.5, 1.6, 3.5, 3.2]).p(53),
                row(58, Widget::Display, "", [0.0, 4.8, 14.0, 1.2]).h(Size::U(1.2)),
            ], ..Case::DEFAULT },
    ],
};
