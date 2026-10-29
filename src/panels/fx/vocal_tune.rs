//! Independent VocalTune editor: four touch pages and a live pitch history.
use super::live_analysis;
use crate::{panels::build::*, ui::UiCtx};
use eframe::egui::{Align2, Color32, FontId, Id, Rect, Sense, Stroke, Ui, pos2, vec2};
use honeydew_audio::{
    analysis,
    vocal_tune::{Scale, scale_mask},
};
const NOTES: [&str; 12] = [
    "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
];
fn mask(module: &ModuleInstance) -> u16 {
    let p = &module.params;
    let scale = match (p[2] * 5.0).round() as u8 {
        1 => Scale::Major,
        2 => Scale::Minor,
        3 => Scale::Pentatonic,
        4 => Scale::Blues,
        5 => Scale::Custom,
        _ => Scale::Chromatic,
    };
    let custom = (0..12).fold(0, |m, i| m | if p[12 + i] >= 0.5 { 1 << i } else { 0 });
    scale_mask((p[1] * 11.0).round() as u8, scale, custom)
}
fn note_hook(module: &mut ModuleInstance, _: &mut PanelState, edited: &[u32]) -> Vec<usize> {
    let mut changed = Vec::new();
    if edited.iter().any(|id| (14..=25).contains(id)) {
        module.params[2] = 1.0;
        changed.push(2);
    } else if module.params[2] < 0.9 {
        let allowed = mask(module);
        for i in 0..12 {
            let value = f32::from(allowed & (1 << i) != 0);
            if module.params[12 + i] != value {
                module.params[12 + i] = value;
                changed.push(12 + i);
            }
        }
    }
    changed
}
#[derive(Clone)]
struct History {
    cents: [f32; 180],
    at: usize,
    last: f64,
}
impl Default for History {
    fn default() -> Self {
        Self {
            cents: [f32::NAN; 180],
            at: 0,
            last: -1.0,
        }
    }
}
fn monitor(
    ui: &mut Ui,
    cx: &UiCtx,
    id: Id,
    rect: Rect,
    module: &mut ModuleInstance,
    _: &mut PanelState,
    ink: Color32,
) -> Vec<usize> {
    let feed = live_analysis(ui, module.instance_id).and_then(|s| analysis::vocal_tune(&s));
    let p = &module.params;
    let reference = 430.0 + 20.0 * p[9];
    let reading = feed.filter(|(hz, _, _, _)| *hz > 0.0 && hz.is_finite());
    let mut cents = f32::NAN;
    let caption = if let Some((hz, target, ratio, clarity)) = reading {
        let midi = 69.0 + 12.0 * (hz / reference).log2();
        let note = midi.round() as i32;
        let input = format!(
            "{}{}",
            NOTES[note.rem_euclid(12) as usize],
            note.div_euclid(12) - 1
        );
        if let Some(target) = target {
            cents = (midi - target as f32) * 100.0;
            format!(
                "{input} → {}{}  {cents:+.0}c · SHIFT {:+.0}c · {:.0}%",
                NOTES[target.rem_euclid(12) as usize],
                target.div_euclid(12) - 1,
                1200.0 * ratio.log2(),
                clarity * 100.0
            )
        } else {
            format!("IN {input}    No allowed notes")
        }
    } else if p[2] >= 0.9 && mask(module) == 0 {
        "No allowed notes — correction paused".into()
    } else {
        "IN —    TARGET —    Waiting for voiced audio".into()
    };
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 4.0, cx.theme.colors.control_fill.to_color32());
    painter.text(
        pos2(rect.center().x, rect.top() + cx.m.u(0.42)),
        Align2::CENTER_CENTER,
        caption,
        FontId::proportional(cx.m.u(0.45)),
        ink,
    );
    let graph = rect
        .shrink2(vec2(cx.m.u(0.3), cx.m.u(0.22)))
        .with_min_y(rect.top() + cx.m.u(0.9));
    let now = ui.input(|i| i.time);
    let mut history = ui
        .ctx()
        .data_mut(|d| d.get_temp::<History>(id))
        .unwrap_or_default();
    if now - history.last >= 1.0 / 30.0 {
        history.cents[history.at] = cents;
        history.at = (history.at + 1) % 180;
        history.last = now;
    }
    for value in [-50.0, 0.0, 50.0] {
        let y = graph.center().y - value / 100.0 * graph.height() * 0.45;
        painter.line_segment(
            [pos2(graph.left(), y), pos2(graph.right(), y)],
            Stroke::new(
                1.0,
                ink.gamma_multiply(if value == 0.0 { 0.4 } else { 0.12 }),
            ),
        );
    }
    let mut previous = None;
    for i in 0..180 {
        let value = history.cents[(history.at + i) % 180];
        if !value.is_finite() {
            previous = None;
            continue;
        }
        let point = pos2(
            graph.left() + i as f32 / 179.0 * graph.width(),
            graph.center().y - value.clamp(-100.0, 100.0) / 100.0 * graph.height() * 0.45,
        );
        if let Some(old) = previous {
            painter.line_segment([old, point], Stroke::new(cx.m.u(0.05).max(1.0), ink));
        }
        previous = Some(point);
    }
    ui.ctx().data_mut(|d| d.insert_temp(id, history));
    ui.ctx()
        .request_repaint_after(std::time::Duration::from_millis(33));
    Vec::new()
}
fn keyboard(
    ui: &mut Ui,
    cx: &UiCtx,
    id: Id,
    rect: Rect,
    module: &mut ModuleInstance,
    _: &mut PanelState,
    ink: Color32,
) -> Vec<usize> {
    let allowed = mask(module);
    let mut changed = Vec::new();
    for (n, name) in NOTES.iter().enumerate() {
        let cell = Rect::from_min_max(
            pos2(rect.left() + n as f32 * rect.width() / 12.0, rect.top()),
            pos2(
                rect.left() + (n + 1) as f32 * rect.width() / 12.0 - 1.0,
                rect.bottom(),
            ),
        );
        let black = name.contains('#');
        let on = allowed & (1 << n) != 0;
        ui.painter().rect_filled(
            cell,
            2.0,
            if on {
                ink.gamma_multiply(if black { 0.45 } else { 0.8 })
            } else {
                cx.theme.colors.control_fill.to_color32()
            },
        );
        ui.painter().text(
            cell.center(),
            Align2::CENTER_CENTER,
            name,
            FontId::proportional(cx.m.u(0.42)),
            if on {
                Color32::BLACK
            } else {
                ink.gamma_multiply(0.5)
            },
        );
        if ui.interact(cell, id.with(n), Sense::click()).clicked() {
            for i in 0..12 {
                module.params[12 + i] = f32::from((allowed ^ (1 << n)) & (1 << i) != 0);
                changed.push(12 + i);
            }
            module.params[2] = 1.0;
            changed.push(2);
        }
    }
    changed
}
fn presets(
    ui: &mut Ui,
    cx: &UiCtx,
    id: Id,
    rect: Rect,
    module: &mut ModuleInstance,
    _: &mut PanelState,
    ink: Color32,
) -> Vec<usize> {
    for (n, name) in ["Natural", "Hard Tune", "Vibrato", "Gentle"]
        .into_iter()
        .enumerate()
    {
        let cell = Rect::from_min_size(
            rect.min + vec2(n as f32 * rect.width() / 4.0, 0.0),
            vec2(rect.width() / 4.0 - 2.0, rect.height()),
        );
        ui.painter()
            .rect_filled(cell, 4.0, cx.theme.colors.control_fill.to_color32());
        ui.painter().text(
            cell.center(),
            Align2::CENTER_CENTER,
            name,
            FontId::proportional(cx.m.u(0.45)),
            ink,
        );
        if ui.interact(cell, id.with(n), Sense::click()).clicked()
            && let Some(preset) = crate::content::preset(53, name)
        {
            let indices = [3, 4, 6, 8, 10, 24, 25, 26, 27, 28, 29, 31, 32];
            for &p in &indices {
                module.params[p] = preset.params[p];
            }
            return indices.to_vec();
        }
    }
    Vec::new()
}
pub fn panel() -> Option<Panel> {
    Some(
        Panel::new(vstack([
            tabs(1, "TUNE · NOTES · EXPRESSION · OUTPUT").h(Size::U(1.75)),
            vstack([
                hstack([
                    spinner(2, "Key").p(1),
                    spinner(3, "Scale").p(2),
                    knob(4, "Correction").p(3),
                    knob(5, "Retune").p(4),
                ])
                .h(Size::U(2.5)),
                hstack([
                    knob(6, "Humanize").p(6),
                    knob(27, "Flex Tune").p(29),
                    knob(28, "Transition").p(28),
                    toggle(12, "Hard Tune").p(10),
                ])
                .h(Size::U(2.5)),
            ])
            .h(Size::U(5.0))
            .show(Show::Tab(0)),
            vstack([
                hstack((0..6).map(|i| toggle(14 + i as u32, NOTES[i]).p(12 + i))).h(Size::U(1.75)),
                hstack((6..12).map(|i| toggle(14 + i as u32, NOTES[i]).p(12 + i))).h(Size::U(1.75)),
                display(29).h(Size::U(1.5)).draw(keyboard),
            ])
            .h(Size::U(5.0))
            .show(Show::Tab(1)),
            vstack([
                hstack([
                    toggle(30, "Formants").p(25),
                    knob(31, "Formant Shift").p(26),
                    knob(32, "Vibrato").p(27),
                    knob(26, "Protect").p(24),
                ])
                .h(Size::U(2.5)),
                hstack([
                    knob(7, "Sensitivity").p(7),
                    spinner(33, "Input Range").p(30),
                    knob(8, "Deadband").p(8),
                    spinner(34, "Quality").p(32),
                ])
                .h(Size::U(2.5)),
            ])
            .h(Size::U(5.0))
            .show(Show::Tab(2)),
            vstack([
                hstack([
                    knob(10, "Mix").p(5),
                    knob(35, "Output Gain").p(31),
                    knob(11, "Reference").p(9),
                    knob(9, "Fine Tune").p(11),
                ])
                .h(Size::U(2.5)),
                display(36).h(Size::U(1.3)).draw(presets),
                toggle(37, "Enabled / Bypass").p(0).h(Size::U(1.2)),
            ])
            .h(Size::U(5.0))
            .show(Show::Tab(3)),
            display(13).h(Size::Flex(1.0)).draw(monitor),
        ]))
        .hook(note_hook),
    )
}
#[cfg(test)]
pub const REFERENCE: Reference = Reference {
    size: [14.0, 9.99],
    settings: &[],
    cases: &[
        Case {
            name: "TUNE",
            tab: 0,
            params: &[],
            rows: &[
                crate::panels::build::row(
                    1,
                    Widget::Tabs,
                    "TUNE · NOTES · EXPRESSION · OUTPUT",
                    [0.00, 0.00, 14.00, 1.75],
                )
                .h(Size::U(1.75)),
                crate::panels::build::row(2, Widget::Spinner, "Key", [0.00, 1.75, 3.50, 2.50]).p(1),
                crate::panels::build::row(3, Widget::Spinner, "Scale", [3.50, 1.75, 3.50, 2.50])
                    .p(2),
                crate::panels::build::row(4, Widget::Knob, "Correction", [7.00, 1.75, 3.50, 2.50])
                    .p(3),
                crate::panels::build::row(5, Widget::Knob, "Retune", [10.50, 1.75, 3.50, 2.50])
                    .p(4),
                crate::panels::build::row(6, Widget::Knob, "Humanize", [0.00, 4.25, 3.50, 2.50])
                    .p(6),
                crate::panels::build::row(27, Widget::Knob, "Flex Tune", [3.50, 4.25, 3.50, 2.50])
                    .p(29),
                crate::panels::build::row(28, Widget::Knob, "Transition", [7.00, 4.25, 3.50, 2.50])
                    .p(28),
                crate::panels::build::row(
                    12,
                    Widget::Toggle,
                    "Hard Tune",
                    [10.50, 4.25, 3.50, 2.50],
                )
                .p(10),
                crate::panels::build::row(13, Widget::Display, "", [0.00, 6.75, 14.00, 3.24]),
            ],
            ..Case::DEFAULT
        },
        Case {
            name: "NOTES",
            tab: 1,
            params: &[],
            rows: &[
                crate::panels::build::row(
                    1,
                    Widget::Tabs,
                    "TUNE · NOTES · EXPRESSION · OUTPUT",
                    [0.00, 0.00, 14.00, 1.75],
                )
                .h(Size::U(1.75)),
                crate::panels::build::row(14, Widget::Toggle, "C", [0.00, 1.75, 2.33, 1.75]).p(12),
                crate::panels::build::row(15, Widget::Toggle, "C#", [2.33, 1.75, 2.33, 1.75]).p(13),
                crate::panels::build::row(16, Widget::Toggle, "D", [4.66, 1.75, 2.34, 1.75]).p(14),
                crate::panels::build::row(17, Widget::Toggle, "D#", [7.00, 1.75, 2.33, 1.75]).p(15),
                crate::panels::build::row(18, Widget::Toggle, "E", [9.33, 1.75, 2.33, 1.75]).p(16),
                crate::panels::build::row(19, Widget::Toggle, "F", [11.66, 1.75, 2.34, 1.75]).p(17),
                crate::panels::build::row(20, Widget::Toggle, "F#", [0.00, 3.50, 2.33, 1.75]).p(18),
                crate::panels::build::row(21, Widget::Toggle, "G", [2.33, 3.50, 2.33, 1.75]).p(19),
                crate::panels::build::row(22, Widget::Toggle, "G#", [4.66, 3.50, 2.34, 1.75]).p(20),
                crate::panels::build::row(23, Widget::Toggle, "A", [7.00, 3.50, 2.33, 1.75]).p(21),
                crate::panels::build::row(24, Widget::Toggle, "A#", [9.33, 3.50, 2.33, 1.75]).p(22),
                crate::panels::build::row(25, Widget::Toggle, "B", [11.66, 3.50, 2.34, 1.75]).p(23),
                crate::panels::build::row(29, Widget::Display, "", [0.00, 5.25, 14.00, 1.50])
                    .h(Size::U(1.50)),
                crate::panels::build::row(13, Widget::Display, "", [0.00, 6.75, 14.00, 3.24]),
            ],
            ..Case::DEFAULT
        },
        Case {
            name: "EXPRESSION",
            tab: 2,
            params: &[],
            rows: &[
                crate::panels::build::row(
                    1,
                    Widget::Tabs,
                    "TUNE · NOTES · EXPRESSION · OUTPUT",
                    [0.00, 0.00, 14.00, 1.75],
                )
                .h(Size::U(1.75)),
                crate::panels::build::row(30, Widget::Toggle, "Formants", [0.00, 1.75, 3.50, 2.50])
                    .p(25),
                crate::panels::build::row(
                    31,
                    Widget::Knob,
                    "Formant Shift",
                    [3.50, 1.75, 3.50, 2.50],
                )
                .p(26),
                crate::panels::build::row(32, Widget::Knob, "Vibrato", [7.00, 1.75, 3.50, 2.50])
                    .p(27),
                crate::panels::build::row(26, Widget::Knob, "Protect", [10.50, 1.75, 3.50, 2.50])
                    .p(24),
                crate::panels::build::row(7, Widget::Knob, "Sensitivity", [0.00, 4.25, 3.50, 2.50])
                    .p(7),
                crate::panels::build::row(
                    33,
                    Widget::Spinner,
                    "Input Range",
                    [3.50, 4.25, 3.50, 2.50],
                )
                .p(30),
                crate::panels::build::row(8, Widget::Knob, "Deadband", [7.00, 4.25, 3.50, 2.50])
                    .p(8),
                crate::panels::build::row(
                    34,
                    Widget::Spinner,
                    "Quality",
                    [10.50, 4.25, 3.50, 2.50],
                )
                .p(32),
                crate::panels::build::row(13, Widget::Display, "", [0.00, 6.75, 14.00, 3.24]),
            ],
            ..Case::DEFAULT
        },
        Case {
            name: "OUTPUT",
            tab: 3,
            params: &[],
            rows: &[
                crate::panels::build::row(
                    1,
                    Widget::Tabs,
                    "TUNE · NOTES · EXPRESSION · OUTPUT",
                    [0.00, 0.00, 14.00, 1.75],
                )
                .h(Size::U(1.75)),
                crate::panels::build::row(10, Widget::Knob, "Mix", [0.00, 1.75, 3.50, 2.50]).p(5),
                crate::panels::build::row(
                    35,
                    Widget::Knob,
                    "Output Gain",
                    [3.50, 1.75, 3.50, 2.50],
                )
                .p(31),
                crate::panels::build::row(11, Widget::Knob, "Reference", [7.00, 1.75, 3.50, 2.50])
                    .p(9),
                crate::panels::build::row(9, Widget::Knob, "Fine Tune", [10.50, 1.75, 3.50, 2.50])
                    .p(11),
                crate::panels::build::row(36, Widget::Display, "", [0.00, 4.25, 14.00, 1.30])
                    .h(Size::U(1.30)),
                crate::panels::build::row(
                    37,
                    Widget::Toggle,
                    "Enabled / Bypass",
                    [0.00, 5.55, 14.00, 1.20],
                )
                .p(0)
                .h(Size::U(1.20)),
                crate::panels::build::row(13, Widget::Display, "", [0.00, 6.75, 14.00, 3.24]),
            ],
            ..Case::DEFAULT
        },
    ],
};
