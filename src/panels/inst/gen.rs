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
pub const REFERENCE: Reference = Reference { size: [14.0, 11.0], settings: &[], cases: &[] };
