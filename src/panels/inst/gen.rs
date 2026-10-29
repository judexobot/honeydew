//! Gen editor: compatible Legacy patches and a register-clocked YM2612/PSG model.
use crate::{panels::build::*, ui::UiCtx};
use eframe::egui::{Color32, FontId, Id, Rect, Sense, Stroke, Ui, pos2, vec2};
use honeydew_audio::r#gen::{DEFAULTS, EXTRA_START, EXTRA_STRIDE, MODEL, OP_START, OP_STRIDE};
const TABS: &str = "FM · OP1 · OP2 · OP3 · OP4 · LFO · PSG · OUTPUT";
const LEGACY: [Show; 4] = [
    Show::All(&[Show::Tab(1), Show::Item(MODEL, 0)]),
    Show::All(&[Show::Tab(2), Show::Item(MODEL, 0)]),
    Show::All(&[Show::Tab(3), Show::Item(MODEL, 0)]),
    Show::All(&[Show::Tab(4), Show::Item(MODEL, 0)]),
];
const NATIVE: [Show; 4] = [
    Show::All(&[Show::Tab(1), Show::Item(MODEL, 1)]),
    Show::All(&[Show::Tab(2), Show::Item(MODEL, 1)]),
    Show::All(&[Show::Tab(3), Show::Item(MODEL, 1)]),
    Show::All(&[Show::Tab(4), Show::Item(MODEL, 1)]),
];
fn touch_button(
    ui: &mut Ui,
    cx: &UiCtx,
    id: Id,
    rect: Rect,
    label: &str,
    state: (bool, bool),
    ink: Color32,
) -> bool {
    let (enabled, selected) = state;
    let r = ui.interact(
        rect,
        id,
        if enabled {
            Sense::click()
        } else {
            Sense::hover()
        },
    );
    ui.painter().rect_filled(
        rect.shrink(1.0),
        3.0,
        ink.gamma_multiply(if selected {
            0.25
        } else if r.hovered() && enabled {
            0.15
        } else {
            0.06
        }),
    );
    ui.painter().text(
        rect.center(),
        Align2::CENTER_CENTER,
        label,
        FontId::proportional(cx.m.u(0.34)),
        ink.gamma_multiply(if enabled { 1.0 } else { 0.35 }),
    );
    enabled && r.clicked()
}
fn cell(rect: Rect, n: usize, count: usize) -> Rect {
    Rect::from_min_size(
        rect.min + vec2(n as f32 * rect.width() / count as f32, 0.0),
        vec2(rect.width() / count as f32, rect.height()),
    )
}
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
    let names = [
        "FM1", "FM2", "FM3", "FM4", "FM5", "FM6", "PSG A", "PSG B", "PSG C", "NOISE",
    ];
    for (i, name) in names.iter().enumerate() {
        let c = cell(rect, i, 12).shrink(1.0);
        let on = data.get(i).is_some_and(|v| *v > 0.5);
        ui.painter()
            .rect_filled(c, 3.0, ink.gamma_multiply(if on { 0.24 } else { 0.06 }));
        ui.painter().text(
            c.center(),
            Align2::CENTER_CENTER,
            *name,
            FontId::proportional(cx.m.u(0.27)),
            ink.gamma_multiply(if on { 1.0 } else { 0.45 }),
        );
        let level = data.get(10 + i).copied().unwrap_or(0.0).clamp(0.0, 1.0);
        let bar = Rect::from_min_size(
            c.left_bottom() - vec2(0.0, cx.m.u(0.08)),
            vec2(c.width() * level, cx.m.u(0.08)),
        );
        ui.painter().rect_filled(bar, 0.0, ink);
    }
    for i in 0..2 {
        let c = cell(rect, i + 10, 12).shrink(1.0);
        let peak = data.get(25 + i).copied().unwrap_or(0.0);
        ui.painter().rect_filled(c, 2.0, ink.gamma_multiply(0.06));
        let bar = Rect::from_min_size(
            c.left_bottom() - vec2(0.0, c.height() * peak.clamp(0.0, 1.0)),
            vec2(c.width(), c.height() * peak.clamp(0.0, 1.0)),
        );
        ui.painter().rect_filled(
            bar,
            2.0,
            if peak >= 1.0 {
                Color32::LIGHT_RED
            } else {
                ink.gamma_multiply(0.45)
            },
        );
        ui.painter().text(
            c.center(),
            Align2::CENTER_CENTER,
            if i == 0 { "L" } else { "R" },
            FontId::proportional(cx.m.u(0.3)),
            ink,
        );
    }
    Vec::new()
}
// These diagrams follow each engine's actual routing, including the retained Legacy algorithms 1–3.
fn edges(native: bool, algorithm: usize) -> &'static [(usize, usize)] {
    match (native, algorithm) {
        (_, 0) => &[(0, 1), (1, 2), (2, 3)],
        (true, 1) | (false, 3) => &[(0, 2), (1, 2), (2, 3)],
        (true, 2) => &[(1, 2), (0, 3), (2, 3)],
        (true, 3) | (false, 2) => &[(0, 1), (1, 3), (2, 3)],
        (false, 1) => &[(0, 1), (0, 2), (1, 3), (2, 3)],
        (_, 4) => &[(0, 1), (2, 3)],
        (_, 5) => &[(0, 1), (0, 2), (0, 3)],
        (_, 6) => &[(0, 1)],
        _ => &[],
    }
}
fn algorithms(
    ui: &mut Ui,
    cx: &UiCtx,
    id: Id,
    rect: Rect,
    module: &mut ModuleInstance,
    _: &mut PanelState,
    ink: Color32,
) -> Vec<usize> {
    let mut changed = Vec::new();
    let native = module.params[MODEL] >= 0.5;
    for a in 0..8 {
        let c = cell(rect, a, 8).shrink(1.0);
        let selected = (module.params[1] * 7.0).round() as usize == a;
        let r = ui.interact(c, id.with(a), Sense::click());
        ui.painter().rect_filled(
            c,
            3.0,
            ink.gamma_multiply(if selected { 0.18 } else { 0.04 }),
        );
        ui.painter().text(
            pos2(c.center().x, c.top() + cx.m.u(0.23)),
            Align2::CENTER_CENTER,
            format!("{}", a + 1),
            FontId::proportional(cx.m.u(0.29)),
            ink,
        );
        let points: [_; 4] = std::array::from_fn(|op| {
            pos2(
                c.left() + c.width() * if op % 2 == 0 { 0.28 } else { 0.72 },
                c.top() + c.height() * if op < 2 { 0.43 } else { 0.77 },
            )
        });
        for &(from, to) in edges(native, a) {
            ui.painter().arrow(
                points[from],
                points[to] - points[from],
                Stroke::new(1.0, ink.gamma_multiply(0.65)),
            );
        }
        for (op, point) in points.into_iter().enumerate() {
            let carrier = !edges(native, a).iter().any(|&(from, _)| from == op);
            ui.painter().circle_filled(
                point,
                cx.m.u(0.18),
                ink.gamma_multiply(if carrier { 0.9 } else { 0.35 }),
            );
            ui.painter().text(
                point,
                Align2::CENTER_CENTER,
                format!("{}", op + 1),
                FontId::proportional(cx.m.u(0.25)),
                cx.theme.colors.control_fill.to_color32(),
            );
        }
        if r.clicked() {
            module.params[1] = a as f32 / 7.0;
            changed.push(1);
        }
    }
    changed
}
#[derive(Clone)]
struct EnvelopeHistory {
    values: [f32; 120],
    at: usize,
    last: f64,
}
impl Default for EnvelopeHistory {
    fn default() -> Self {
        Self {
            values: [0.0; 120],
            at: 0,
            last: -1.0,
        }
    }
}
fn operator_indices(op: usize) -> impl Iterator<Item = usize> {
    (OP_START + op * OP_STRIDE..OP_START + (op + 1) * OP_STRIDE)
        .chain(EXTRA_START + op * EXTRA_STRIDE..EXTRA_START + (op + 1) * EXTRA_STRIDE)
        .chain(std::iter::once(102 + op))
}
fn set_values(module: &mut ModuleInstance, values: &[(usize, f32)]) -> Vec<usize> {
    values
        .iter()
        .filter_map(|&(i, v)| {
            let p = module.params.get_mut(i)?;
            if *p == v {
                None
            } else {
                *p = v;
                Some(i)
            }
        })
        .collect()
}
fn operator_monitor(
    ui: &mut Ui,
    cx: &UiCtx,
    id: Id,
    rect: Rect,
    module: &mut ModuleInstance,
    state: &mut PanelState,
    ink: Color32,
) -> Vec<usize> {
    let op = state.tab.saturating_sub(1).min(3);
    let clipboard_id = Id::new("gen-operator-clipboard");
    let buttons = rect.with_max_y(rect.top() + cx.m.u(1.0));
    let copy = ui.ctx().data_mut(|d| d.get_temp::<Vec<f32>>(clipboard_id));
    let mut changed = Vec::new();
    if touch_button(
        ui,
        cx,
        id.with("copy"),
        cell(buttons, 0, 3),
        "Copy operator",
        (true, false),
        ink,
    ) {
        let values: Vec<_> = operator_indices(op).map(|i| module.params[i]).collect();
        ui.ctx().data_mut(|d| d.insert_temp(clipboard_id, values));
    }
    if touch_button(
        ui,
        cx,
        id.with("paste"),
        cell(buttons, 1, 3),
        "Paste operator",
        (copy.is_some(), false),
        ink,
    ) && let Some(copy) = copy
    {
        changed.extend(set_values(
            module,
            &operator_indices(op).zip(copy).collect::<Vec<_>>(),
        ));
    }
    if touch_button(
        ui,
        cx,
        id.with("init"),
        cell(buttons, 2, 3),
        "Init operator",
        (true, false),
        ink,
    ) {
        changed.extend(set_values(
            module,
            &operator_indices(op)
                .map(|i| (i, DEFAULTS[i]))
                .collect::<Vec<_>>(),
        ));
    }
    let plot = rect
        .with_min_y(buttons.bottom() + cx.m.u(0.1))
        .shrink2(vec2(cx.m.u(0.15), cx.m.u(0.08)));
    let feed = crate::panels::fx::live_analysis(ui, module.instance_id);
    let history_id = id.with(op);
    let mut history = ui
        .ctx()
        .data_mut(|d| d.get_temp::<EnvelopeHistory>(history_id))
        .unwrap_or_default();
    let now = ui.input(|i| i.time);
    if now - history.last >= 1.0 / 30.0 {
        history.values[history.at] = feed
            .as_ref()
            .and_then(|d| d.get(20 + op))
            .copied()
            .unwrap_or(0.0);
        history.at = (history.at + 1) % 120;
        history.last = now;
    }
    ui.painter()
        .rect_filled(plot, 2.0, ink.gamma_multiply(0.05));
    for i in 1..120 {
        let point = |k: usize| {
            pos2(
                plot.left() + k as f32 / 119.0 * plot.width(),
                plot.bottom()
                    - history.values[(history.at + k) % 120].clamp(0.0, 1.0) * plot.height(),
            )
        };
        ui.painter()
            .line_segment([point(i - 1), point(i)], Stroke::new(1.5, ink));
    }
    ui.painter().text(
        plot.left_top(),
        Align2::LEFT_TOP,
        "Live envelope · most recent FM voice",
        FontId::proportional(cx.m.u(0.27)),
        ink.gamma_multiply(0.65),
    );
    ui.ctx().data_mut(|d| d.insert_temp(history_id, history));
    changed
}
fn patch_tools(
    ui: &mut Ui,
    cx: &UiCtx,
    id: Id,
    rect: Rect,
    module: &mut ModuleInstance,
    _: &mut PanelState,
    ink: Color32,
) -> Vec<usize> {
    let store_id = id.with(module.instance_id);
    let a = ui
        .ctx()
        .data_mut(|d| d.get_temp::<Vec<f32>>(store_id.with("A")));
    let b = ui
        .ctx()
        .data_mut(|d| d.get_temp::<Vec<f32>>(store_id.with("B")));
    let mut changed = Vec::new();
    for (n, name) in ["Store A", "Store B", "A", "B", "Init patch"]
        .iter()
        .enumerate()
    {
        let enabled = match n {
            2 => a.is_some(),
            3 => b.is_some(),
            _ => true,
        };
        let selected = match n {
            2 => a.as_ref(),
            3 => b.as_ref(),
            _ => None,
        }
        .is_some_and(|p| p.as_slice() == module.params.as_slice());
        if touch_button(
            ui,
            cx,
            id.with(n),
            cell(rect, n, 5),
            name,
            (enabled, selected),
            ink,
        ) {
            match n {
                0 | 1 => {
                    ui.ctx().data_mut(|d| {
                        d.insert_temp(
                            store_id.with(if n == 0 { "A" } else { "B" }),
                            module.params.clone(),
                        )
                    });
                }
                2 | 3 => {
                    if let Some(p) = if n == 2 { &a } else { &b } {
                        changed.extend(set_values(
                            module,
                            &p.iter().copied().enumerate().skip(1).collect::<Vec<_>>(),
                        ));
                    }
                }
                _ => {
                    let values: Vec<_> = DEFAULTS
                        .iter()
                        .copied()
                        .enumerate()
                        .skip(1)
                        .filter(|(i, _)| *i != MODEL)
                        .collect();
                    changed.extend(set_values(module, &values));
                }
            }
        }
    }
    changed
}
fn lfo_monitor(
    ui: &mut Ui,
    cx: &UiCtx,
    _: Id,
    rect: Rect,
    module: &mut ModuleInstance,
    _: &mut PanelState,
    ink: Color32,
) -> Vec<usize> {
    let data = crate::panels::fx::live_analysis(ui, module.instance_id).unwrap_or_default();
    let phase = data.get(24).copied().unwrap_or(0.0);
    let graph = rect.shrink2(vec2(cx.m.u(0.3), cx.m.u(0.35)));
    let native = module.params[MODEL] >= 0.5;
    let value = |p: f32| {
        if native {
            1.0 - (4.0 * p - 2.0).abs()
        } else {
            (std::f32::consts::TAU * p).sin()
        }
    };
    let point = |p: f32| {
        pos2(
            graph.left() + graph.width() * p,
            graph.center().y - graph.height() * 0.45 * value(p),
        )
    };
    for n in 1..128 {
        ui.painter().line_segment(
            [point((n - 1) as f32 / 127.0), point(n as f32 / 127.0)],
            Stroke::new(1.0, ink.gamma_multiply(0.5)),
        );
    }
    if !data.is_empty() {
        ui.painter()
            .circle_filled(point(phase.fract()), cx.m.u(0.10), ink);
    }
    ui.painter().text(
        graph.left_top(),
        Align2::LEFT_TOP,
        "LFO phase",
        FontId::proportional(cx.m.u(0.3)),
        ink,
    );
    Vec::new()
}
fn controls(parts: [Part; 4]) -> Part {
    hstack(parts).h(Size::U(3.2))
}
pub fn panel() -> Option<Panel> {
    let mut parts = vec![
        tabs(1, TABS).h(Size::U(1.6)),
        controls([
            spinner(2, "Algorithm").p(1),
            knob(3, "Feedback").p(2),
            knob(4, "FM Level").p(4),
            knob(5, "Master").p(3),
        ])
        .show(Show::Tab(0)),
        controls([
            knob(6, "Drive").p(9),
            knob(7, "DAC Color").p(10),
            knob(8, "Stereo Width").p(11),
            spinner(9, "Engine").p(54),
        ])
        .show(Show::Tab(0)),
        hstack([
            spinner(59, "FM Model").p(MODEL),
            h_slider(60, "Chip Region")
                .p(57)
                .enabled(Show::Not(&Show::All(&[
                    Show::Item(MODEL, 0),
                    Show::Item(58, 0),
                ]))),
            spinner(61, "PSG Model").p(58),
            h_slider(62, "Chip Velocity")
                .p(61)
                .enabled(Show::Item(MODEL, 1)),
        ])
        .h(Size::U(2.4))
        .show(Show::Tab(0)),
        display(65)
            .h(Size::U(2.3))
            .draw(algorithms)
            .show(Show::Tab(0)),
    ];
    for op in 0..4 {
        let id = 10 + op as u32 * 8;
        let p = OP_START + op * OP_STRIDE;
        parts.push(
            controls([
                spinner(id, "Ratio").p(p),
                knob(id + 1, "Detune").p(p + 1),
                knob(id + 2, "Level").p(p + 2),
                knob(id + 3, "Attack").p(p + 3),
            ])
            .show(LEGACY[op]),
        );
        parts.push(
            controls([
                knob(id + 4, "Decay").p(p + 4),
                knob(id + 5, "Sustain").p(p + 5),
                knob(id + 6, "Release").p(p + 6),
                knob(id + 7, "Amp Mod").p(p + 7),
            ])
            .show(LEGACY[op]),
        );
        let id = 100 + op as u32 * 16;
        let p = EXTRA_START + op * EXTRA_STRIDE;
        parts.push(
            hstack([
                knob(id, "Attack rate").p(p),
                knob(id + 1, "Decay rate").p(p + 1),
                knob(id + 2, "Sustain rate").p(p + 2),
                spinner(id + 3, "Sustain level").p(p + 3),
            ])
            .h(Size::U(2.8))
            .show(NATIVE[op]),
        );
        parts.push(
            hstack([
                knob(id + 4, "Release rate").p(p + 4),
                knob(id + 5, "Rate scale").p(p + 5),
                knob(id + 6, "Total level").p(p + 6),
                spinner(id + 7, "Detune").p(p + 7),
            ])
            .h(Size::U(2.8))
            .show(NATIVE[op]),
        );
        parts.push(
            hstack([
                spinner(id + 8, "SSG-EG").p(p + 8),
                spinner(id + 9, "Multiple").p(p + 9),
                toggle(id + 10, "AM enable").p(102 + op),
                knob(id + 11, "Master").p(3),
            ])
            .h(Size::U(2.8))
            .show(NATIVE[op]),
        );
    }
    parts.extend([
        display(66)
            .h(Size::U(2.6))
            .draw(operator_monitor)
            .show(Show::Not(&Show::All(&[
                Show::Not(&Show::Tab(1)),
                Show::Not(&Show::Tab(2)),
                Show::Not(&Show::Tab(3)),
                Show::Not(&Show::Tab(4)),
            ]))),
        controls([
            vstack([
                knob(42, "LFO Rate").p(6).show(Show::Item(MODEL, 0)),
                spinner(63, "Chip LFO").p(60).show(Show::Item(MODEL, 1)),
            ]),
            knob(43, "Pitch LFO").p(7),
            knob(44, "Amp LFO").p(8),
            knob(45, "Portamento").p(52),
        ])
        .show(Show::Tab(5)),
        h_slider(76, "Follow rate (Hz)")
            .p(6)
            .h(Size::U(1.2))
            .enabled(Show::Item(60, 0))
            .show(Show::All(&[Show::Tab(5), Show::Item(MODEL, 1)])),
        display(68)
            .h(Size::U(2.4))
            .draw(lfo_monitor)
            .show(Show::Tab(5)),
        controls([
            knob(46, "PSG A").p(44),
            knob(47, "PSG B").p(45),
            knob(48, "PSG C").p(46),
            knob(49, "Noise").p(47),
        ])
        .show(Show::Tab(6)),
        controls([
            spinner(50, "Noise Type").p(48),
            vstack([
                knob(51, "Noise Rate").p(49).show(Show::Item(58, 0)),
                spinner(64, "Noise Clock").p(59).show(Show::Item(58, 1)),
            ]),
            knob(52, "PSG Spread").p(50),
            knob(53, "PSG Mix").p(5),
        ])
        .show(Show::Tab(6)),
        controls([
            spinner(54, "Engine").p(54),
            knob(55, "Output Trim").p(55),
            knob(56, "Pitch Bend").p(51),
            knob(57, "Release Tail").p(53),
        ])
        .show(Show::Tab(7)),
        controls([
            knob(77, "Master").p(3),
            knob(78, "FM Level").p(4),
            knob(79, "PSG Level").p(5),
            knob(80, "Stereo Width").p(11),
        ])
        .show(Show::Tab(7)),
        display(67)
            .h(Size::U(1.4))
            .draw(patch_tools)
            .show(Show::Tab(7)),
        display(58).h(Size::U(1.2)).draw(status),
    ]);
    Some(Panel::new(vstack(parts)))
}
#[cfg(test)]
include!("gen_reference.rs");
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn copied_operators_include_both_models_and_all_raw_fields() {
        let mut module = ModuleInstance::new(55, 0);
        assert_eq!(module.params.len(), DEFAULTS.len());
        let copy: Vec<_> = operator_indices(0).map(|i| module.params[i]).collect();
        assert_eq!(copy.len(), 19);
        let values: Vec<_> = operator_indices(3).zip(copy).collect();
        set_values(&mut module, &values);
        for ((_, value), destination) in values.iter().zip(operator_indices(3)) {
            assert_eq!(*value, module.params[destination]);
        }
        assert_eq!(module.params[MODEL], 0.0);
    }
}
