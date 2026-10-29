//! Register-based DMG editor, including a touch-drawn 32-nibble wave table.
use crate::{panels::build::*, ui::UiCtx};
use eframe::egui::{Align2, Color32, FontId, Id, Rect, Sense, Stroke, Ui, pos2, vec2};
use honeydew_audio::gameboy_instrument::{CUSTOM_WAVE, MODE, WAVE_START};
fn wave_editor(
    ui: &mut Ui,
    cx: &UiCtx,
    id: Id,
    rect: Rect,
    module: &mut ModuleInstance,
    _: &mut PanelState,
    ink: Color32,
) -> Vec<usize> {
    let plot = rect.shrink2(vec2(cx.m.u(0.2), cx.m.u(0.15)));
    let mut samples = [0u8; 32];
    for (i, v) in samples.iter_mut().enumerate() {
        *v = if module.params[CUSTOM_WAVE] >= 0.5 {
            (module.params[WAVE_START + i] * 15.0).round() as u8
        } else {
            honeydew_audio::gameboy_instrument::wave_sample(
                (module.params[9] * 3.0).round() as u8,
                i,
            )
        };
    }
    ui.painter()
        .rect_filled(rect, 4.0, cx.theme.colors.control_fill.to_color32());
    for value in [0.0, 5.0, 10.0, 15.0] {
        let y = plot.bottom() - value / 15.0 * plot.height();
        ui.painter().line_segment(
            [pos2(plot.left(), y), pos2(plot.right(), y)],
            Stroke::new(1.0, ink.gamma_multiply(0.15)),
        );
    }
    for (i, &value) in samples.iter().enumerate() {
        let x = plot.left() + i as f32 / 32.0 * plot.width();
        let bar = Rect::from_min_max(
            pos2(x, plot.bottom() - value as f32 / 15.0 * plot.height()),
            pos2(x + plot.width() / 32.0 - 1.0, plot.bottom()),
        );
        ui.painter().rect_filled(bar, 0.0, ink.gamma_multiply(0.8));
    }
    let response = ui.interact(plot, id, Sense::click_and_drag());
    let mut changed = Vec::new();
    if (response.dragged() || response.clicked() || response.is_pointer_button_down_on())
        && let Some(point) = response.interact_pointer_pos()
    {
        let x = ((point.x - plot.left()) / plot.width() * 32.0)
            .floor()
            .clamp(0.0, 31.0) as usize;
        let y = ((plot.bottom() - point.y) / plot.height() * 15.0)
            .round()
            .clamp(0.0, 15.0) as u8;
        if module.params[CUSTOM_WAVE] < 0.5 {
            for (i, &value) in samples.iter().enumerate() {
                module.params[WAVE_START + i] = value as f32 / 15.0;
                changed.push(WAVE_START + i);
            }
            module.params[CUSTOM_WAVE] = 1.0;
            changed.push(CUSTOM_WAVE);
        }
        // Capture the initial touch before egui's drag threshold; the first move interpolates from it.
        let previous = if ui.input(|i| i.pointer.primary_pressed()) {
            None
        } else {
            ui.ctx().data_mut(|d| d.get_temp::<(usize, u8)>(id))
        };
        let (old_x, old_y) = previous.unwrap_or((x, y));
        for i in old_x.min(x)..=old_x.max(x) {
            let weight = if x == old_x {
                1.0
            } else {
                (i as f32 - old_x as f32) / (x as f32 - old_x as f32)
            };
            let value = (old_y as f32 + (y as f32 - old_y as f32) * weight).round() / 15.0;
            module.params[WAVE_START + i] = value;
            changed.push(WAVE_START + i);
        }
        ui.ctx().data_mut(|d| d.insert_temp(id, (x, y)));
    }
    if response.drag_stopped() || response.clicked() {
        ui.ctx().data_mut(|d| d.remove::<(usize, u8)>(id));
    }
    changed
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
    for (ch, name) in ["PULSE 1", "PULSE 2", "WAVE", "NOISE"].iter().enumerate() {
        let cell = Rect::from_min_size(
            rect.min + vec2(ch as f32 * rect.width() / 4.0, 0.0),
            vec2(rect.width() / 4.0 - 2.0, rect.height()),
        );
        let active = data.get(ch).copied().unwrap_or(0.0) > 0.5;
        ui.painter().rect_filled(
            cell,
            3.0,
            ink.gamma_multiply(if active { 0.3 } else { 0.07 }),
        );
        ui.painter().text(
            cell.center(),
            Align2::CENTER_CENTER,
            name,
            FontId::proportional(cx.m.u(0.4)),
            ink.gamma_multiply(if active { 1.0 } else { 0.4 }),
        );
        let level = data.get(ch + 4).copied().unwrap_or(0.0);
        let bar = Rect::from_min_size(
            cell.left_bottom() - vec2(0.0, cx.m.u(0.08)),
            vec2(cell.width() * level, cx.m.u(0.08)),
        );
        ui.painter().rect_filled(bar, 0.0, ink);
    }
    Vec::new()
}
fn row(parts: impl IntoIterator<Item = Part>) -> Part {
    hstack(parts).h(Size::U(2.5))
}
pub fn panel() -> Option<Panel> {
    Some(Panel::new(vstack([
        tabs(1, "PULSE 1 · PULSE 2 · WAVE · NOISE · MIX / OUTPUT").h(Size::U(1.75)),
        vstack([
            row([
                toggle(2, "CH1").p(18),
                knob(3, "Level").p(1),
                spinner(4, "Duty").p(2),
                spinner(5, "Sweep Pace").p(3),
                spinner(6, "Sweep Shift").p(4),
                toggle(7, "Sweep Down").p(5),
            ]),
            row([
                toggle(30, "Envelope Rise").p(59),
                spinner(31, "Envelope Pace").p(60),
                toggle(32, "Length On").p(61),
                knob(33, "Length").p(62),
                knob(34, "Transpose").p(63),
            ]),
        ])
        .h(Size::U(8.0))
        .show(Show::Tab(0)),
        vstack([
            row([
                toggle(8, "CH2").p(19),
                knob(9, "Level").p(6),
                spinner(10, "Duty").p(7),
                toggle(35, "Envelope Rise").p(64),
            ]),
            row([
                spinner(36, "Envelope Pace").p(65),
                toggle(37, "Length On").p(66),
                knob(38, "Length").p(67),
                knob(39, "Transpose").p(68),
            ]),
        ])
        .h(Size::U(8.0))
        .show(Show::Tab(1)),
        vstack([
            hstack([
                toggle(12, "CH3").p(20),
                spinner(13, "Wave Level").p(8),
                spinner(14, "Waveform").p(9),
                toggle(40, "Custom Wave").p(CUSTOM_WAVE),
            ])
            .h(Size::U(2.0)),
            display(41).h(Size::U(3.5)).draw(wave_editor),
            row([
                toggle(42, "Length On").p(69),
                knob(43, "Length").p(70),
                knob(44, "Transpose").p(71),
            ]),
        ])
        .h(Size::U(8.0))
        .show(Show::Tab(2)),
        vstack([
            row([
                toggle(15, "CH4").p(21),
                knob(16, "Level").p(10),
                toggle(17, "7-bit Noise").p(11),
                spinner(18, "Divisor").p(12),
                spinner(19, "Clock Shift").p(13),
            ]),
            row([
                toggle(45, "Envelope Rise").p(72),
                spinner(46, "Envelope Pace").p(73),
                toggle(47, "Length On").p(74),
                knob(48, "Length").p(75),
                spinner(49, "Noise Clock").p(76),
            ]),
        ])
        .h(Size::U(8.0))
        .show(Show::Tab(3)),
        vstack([
            hstack([
                spinner(50, "Mode").p(MODE),
                spinner(51, "Note Routing").p(23),
                spinner(52, "Keyboard Channel").p(24),
                knob(53, "Bend Range").p(25),
            ])
            .h(Size::U(2.0)),
            hstack([
                spinner(20, "Master").p(15),
                spinner(21, "Stereo").p(16),
                knob(22, "Output Gain").p(17),
                knob(54, "Ext. Release").p(82),
            ])
            .h(Size::U(2.0)),
            hstack([
                toggle(55, "Custom Routing").p(77),
                spinner(56, "CH1").p(78),
                spinner(57, "CH2").p(79),
                spinner(58, "CH3").p(80),
                spinner(59, "CH4").p(81),
            ])
            .h(Size::U(2.0)),
            hstack([
                knob(60, "CH1 Pan").p(83),
                knob(61, "CH2 Pan").p(84),
                knob(62, "CH3 Pan").p(85),
                knob(63, "CH4 Pan").p(86),
            ])
            .h(Size::U(2.0))
            .show(Show::Item(MODE, 1)),
        ])
        .h(Size::U(8.0))
        .show(Show::Tab(4)),
        display(64).h(Size::U(0.8)).draw(status),
    ])))
}
#[cfg(test)]
pub const REFERENCE: Reference = Reference {
    size: [14.0, 10.99],
    settings: &[],
    cases: &[
        Case {
            name: "PULSE 1",
            tab: 0,
            params: &[],
            rows: &[
                crate::panels::build::row(
                    1,
                    Widget::Tabs,
                    "PULSE 1 · PULSE 2 · WAVE · NOISE · MIX / OUTPUT",
                    [0.00, 0.00, 14.00, 1.75],
                )
                .h(Size::U(1.75)),
                crate::panels::build::row(2, Widget::Toggle, "CH1", [0.00, 1.75, 2.33, 2.50]).p(18),
                crate::panels::build::row(3, Widget::Knob, "Level", [2.33, 1.75, 2.33, 2.50]).p(1),
                crate::panels::build::row(4, Widget::Spinner, "Duty", [4.66, 1.75, 2.34, 2.50])
                    .p(2),
                crate::panels::build::row(
                    5,
                    Widget::Spinner,
                    "Sweep Pace",
                    [7.00, 1.75, 2.33, 2.50],
                )
                .p(3),
                crate::panels::build::row(
                    6,
                    Widget::Spinner,
                    "Sweep Shift",
                    [9.33, 1.75, 2.33, 2.50],
                )
                .p(4),
                crate::panels::build::row(
                    7,
                    Widget::Toggle,
                    "Sweep Down",
                    [11.66, 1.75, 2.34, 2.50],
                )
                .p(5),
                crate::panels::build::row(
                    30,
                    Widget::Toggle,
                    "Envelope Rise",
                    [0.00, 4.25, 2.80, 2.50],
                )
                .p(59),
                crate::panels::build::row(
                    31,
                    Widget::Spinner,
                    "Envelope Pace",
                    [2.80, 4.25, 2.80, 2.50],
                )
                .p(60),
                crate::panels::build::row(
                    32,
                    Widget::Toggle,
                    "Length On",
                    [5.60, 4.25, 2.80, 2.50],
                )
                .p(61),
                crate::panels::build::row(33, Widget::Knob, "Length", [8.40, 4.25, 2.80, 2.50])
                    .p(62),
                crate::panels::build::row(34, Widget::Knob, "Transpose", [11.20, 4.25, 2.80, 2.50])
                    .p(63),
                crate::panels::build::row(64, Widget::Display, "", [0.00, 9.75, 14.00, 0.80])
                    .h(Size::U(0.80)),
            ],
            ..Case::DEFAULT
        },
        Case {
            name: "PULSE 2",
            tab: 1,
            params: &[],
            rows: &[
                crate::panels::build::row(
                    1,
                    Widget::Tabs,
                    "PULSE 1 · PULSE 2 · WAVE · NOISE · MIX / OUTPUT",
                    [0.00, 0.00, 14.00, 1.75],
                )
                .h(Size::U(1.75)),
                crate::panels::build::row(8, Widget::Toggle, "CH2", [0.00, 1.75, 3.50, 2.50]).p(19),
                crate::panels::build::row(9, Widget::Knob, "Level", [3.50, 1.75, 3.50, 2.50]).p(6),
                crate::panels::build::row(10, Widget::Spinner, "Duty", [7.00, 1.75, 3.50, 2.50])
                    .p(7),
                crate::panels::build::row(
                    35,
                    Widget::Toggle,
                    "Envelope Rise",
                    [10.50, 1.75, 3.50, 2.50],
                )
                .p(64),
                crate::panels::build::row(
                    36,
                    Widget::Spinner,
                    "Envelope Pace",
                    [0.00, 4.25, 3.50, 2.50],
                )
                .p(65),
                crate::panels::build::row(
                    37,
                    Widget::Toggle,
                    "Length On",
                    [3.50, 4.25, 3.50, 2.50],
                )
                .p(66),
                crate::panels::build::row(38, Widget::Knob, "Length", [7.00, 4.25, 3.50, 2.50])
                    .p(67),
                crate::panels::build::row(39, Widget::Knob, "Transpose", [10.50, 4.25, 3.50, 2.50])
                    .p(68),
                crate::panels::build::row(64, Widget::Display, "", [0.00, 9.75, 14.00, 0.80])
                    .h(Size::U(0.80)),
            ],
            ..Case::DEFAULT
        },
        Case {
            name: "WAVE",
            tab: 2,
            params: &[],
            rows: &[
                crate::panels::build::row(
                    1,
                    Widget::Tabs,
                    "PULSE 1 · PULSE 2 · WAVE · NOISE · MIX / OUTPUT",
                    [0.00, 0.00, 14.00, 1.75],
                )
                .h(Size::U(1.75)),
                crate::panels::build::row(12, Widget::Toggle, "CH3", [0.00, 1.75, 3.50, 2.00])
                    .p(20),
                crate::panels::build::row(
                    13,
                    Widget::Spinner,
                    "Wave Level",
                    [3.50, 1.75, 3.50, 2.00],
                )
                .p(8),
                crate::panels::build::row(
                    14,
                    Widget::Spinner,
                    "Waveform",
                    [7.00, 1.75, 3.50, 2.00],
                )
                .p(9),
                crate::panels::build::row(
                    40,
                    Widget::Toggle,
                    "Custom Wave",
                    [10.50, 1.75, 3.50, 2.00],
                )
                .p(26),
                crate::panels::build::row(41, Widget::Display, "", [0.00, 3.75, 14.00, 3.50])
                    .h(Size::U(3.50)),
                crate::panels::build::row(
                    42,
                    Widget::Toggle,
                    "Length On",
                    [0.00, 7.25, 4.66, 2.50],
                )
                .p(69),
                crate::panels::build::row(43, Widget::Knob, "Length", [4.66, 7.25, 4.67, 2.50])
                    .p(70),
                crate::panels::build::row(44, Widget::Knob, "Transpose", [9.33, 7.25, 4.67, 2.50])
                    .p(71),
                crate::panels::build::row(64, Widget::Display, "", [0.00, 9.75, 14.00, 0.80])
                    .h(Size::U(0.80)),
            ],
            ..Case::DEFAULT
        },
        Case {
            name: "NOISE",
            tab: 3,
            params: &[],
            rows: &[
                crate::panels::build::row(
                    1,
                    Widget::Tabs,
                    "PULSE 1 · PULSE 2 · WAVE · NOISE · MIX / OUTPUT",
                    [0.00, 0.00, 14.00, 1.75],
                )
                .h(Size::U(1.75)),
                crate::panels::build::row(15, Widget::Toggle, "CH4", [0.00, 1.75, 2.80, 2.50])
                    .p(21),
                crate::panels::build::row(16, Widget::Knob, "Level", [2.80, 1.75, 2.80, 2.50])
                    .p(10),
                crate::panels::build::row(
                    17,
                    Widget::Toggle,
                    "7-bit Noise",
                    [5.60, 1.75, 2.80, 2.50],
                )
                .p(11),
                crate::panels::build::row(18, Widget::Spinner, "Divisor", [8.40, 1.75, 2.80, 2.50])
                    .p(12),
                crate::panels::build::row(
                    19,
                    Widget::Spinner,
                    "Clock Shift",
                    [11.20, 1.75, 2.80, 2.50],
                )
                .p(13),
                crate::panels::build::row(
                    45,
                    Widget::Toggle,
                    "Envelope Rise",
                    [0.00, 4.25, 2.80, 2.50],
                )
                .p(72),
                crate::panels::build::row(
                    46,
                    Widget::Spinner,
                    "Envelope Pace",
                    [2.80, 4.25, 2.80, 2.50],
                )
                .p(73),
                crate::panels::build::row(
                    47,
                    Widget::Toggle,
                    "Length On",
                    [5.60, 4.25, 2.80, 2.50],
                )
                .p(74),
                crate::panels::build::row(48, Widget::Knob, "Length", [8.40, 4.25, 2.80, 2.50])
                    .p(75),
                crate::panels::build::row(
                    49,
                    Widget::Spinner,
                    "Noise Clock",
                    [11.20, 4.25, 2.80, 2.50],
                )
                .p(76),
                crate::panels::build::row(64, Widget::Display, "", [0.00, 9.75, 14.00, 0.80])
                    .h(Size::U(0.80)),
            ],
            ..Case::DEFAULT
        },
        Case {
            name: "MIX Authentic",
            tab: 4,
            params: &[],
            rows: &[
                crate::panels::build::row(
                    1,
                    Widget::Tabs,
                    "PULSE 1 · PULSE 2 · WAVE · NOISE · MIX / OUTPUT",
                    [0.00, 0.00, 14.00, 1.75],
                )
                .h(Size::U(1.75)),
                crate::panels::build::row(50, Widget::Spinner, "Mode", [0.00, 1.75, 3.50, 2.00])
                    .p(22),
                crate::panels::build::row(
                    51,
                    Widget::Spinner,
                    "Note Routing",
                    [3.50, 1.75, 3.50, 2.00],
                )
                .p(23),
                crate::panels::build::row(
                    52,
                    Widget::Spinner,
                    "Keyboard Channel",
                    [7.00, 1.75, 3.50, 2.00],
                )
                .p(24),
                crate::panels::build::row(
                    53,
                    Widget::Knob,
                    "Bend Range",
                    [10.50, 1.75, 3.50, 2.00],
                )
                .p(25),
                crate::panels::build::row(20, Widget::Spinner, "Master", [0.00, 3.75, 3.50, 2.00])
                    .p(15),
                crate::panels::build::row(21, Widget::Spinner, "Stereo", [3.50, 3.75, 3.50, 2.00])
                    .p(16),
                crate::panels::build::row(
                    22,
                    Widget::Knob,
                    "Output Gain",
                    [7.00, 3.75, 3.50, 2.00],
                )
                .p(17),
                crate::panels::build::row(
                    54,
                    Widget::Knob,
                    "Ext. Release",
                    [10.50, 3.75, 3.50, 2.00],
                )
                .p(82),
                crate::panels::build::row(
                    55,
                    Widget::Toggle,
                    "Custom Routing",
                    [0.00, 5.75, 2.80, 2.00],
                )
                .p(77),
                crate::panels::build::row(56, Widget::Spinner, "CH1", [2.80, 5.75, 2.80, 2.00])
                    .p(78),
                crate::panels::build::row(57, Widget::Spinner, "CH2", [5.60, 5.75, 2.80, 2.00])
                    .p(79),
                crate::panels::build::row(58, Widget::Spinner, "CH3", [8.40, 5.75, 2.80, 2.00])
                    .p(80),
                crate::panels::build::row(59, Widget::Spinner, "CH4", [11.20, 5.75, 2.80, 2.00])
                    .p(81),
                crate::panels::build::row(64, Widget::Display, "", [0.00, 9.75, 14.00, 0.80])
                    .h(Size::U(0.80)),
            ],
            ..Case::DEFAULT
        },
        Case {
            name: "MIX Extended",
            tab: 4,
            params: &[(22, 1.0)],
            rows: &[
                crate::panels::build::row(
                    1,
                    Widget::Tabs,
                    "PULSE 1 · PULSE 2 · WAVE · NOISE · MIX / OUTPUT",
                    [0.00, 0.00, 14.00, 1.75],
                )
                .h(Size::U(1.75)),
                crate::panels::build::row(50, Widget::Spinner, "Mode", [0.00, 1.75, 3.50, 2.00])
                    .p(22),
                crate::panels::build::row(
                    51,
                    Widget::Spinner,
                    "Note Routing",
                    [3.50, 1.75, 3.50, 2.00],
                )
                .p(23),
                crate::panels::build::row(
                    52,
                    Widget::Spinner,
                    "Keyboard Channel",
                    [7.00, 1.75, 3.50, 2.00],
                )
                .p(24),
                crate::panels::build::row(
                    53,
                    Widget::Knob,
                    "Bend Range",
                    [10.50, 1.75, 3.50, 2.00],
                )
                .p(25),
                crate::panels::build::row(20, Widget::Spinner, "Master", [0.00, 3.75, 3.50, 2.00])
                    .p(15),
                crate::panels::build::row(21, Widget::Spinner, "Stereo", [3.50, 3.75, 3.50, 2.00])
                    .p(16),
                crate::panels::build::row(
                    22,
                    Widget::Knob,
                    "Output Gain",
                    [7.00, 3.75, 3.50, 2.00],
                )
                .p(17),
                crate::panels::build::row(
                    54,
                    Widget::Knob,
                    "Ext. Release",
                    [10.50, 3.75, 3.50, 2.00],
                )
                .p(82),
                crate::panels::build::row(
                    55,
                    Widget::Toggle,
                    "Custom Routing",
                    [0.00, 5.75, 2.80, 2.00],
                )
                .p(77),
                crate::panels::build::row(56, Widget::Spinner, "CH1", [2.80, 5.75, 2.80, 2.00])
                    .p(78),
                crate::panels::build::row(57, Widget::Spinner, "CH2", [5.60, 5.75, 2.80, 2.00])
                    .p(79),
                crate::panels::build::row(58, Widget::Spinner, "CH3", [8.40, 5.75, 2.80, 2.00])
                    .p(80),
                crate::panels::build::row(59, Widget::Spinner, "CH4", [11.20, 5.75, 2.80, 2.00])
                    .p(81),
                crate::panels::build::row(60, Widget::Knob, "CH1 Pan", [0.00, 7.75, 3.50, 2.00])
                    .p(83),
                crate::panels::build::row(61, Widget::Knob, "CH2 Pan", [3.50, 7.75, 3.50, 2.00])
                    .p(84),
                crate::panels::build::row(62, Widget::Knob, "CH3 Pan", [7.00, 7.75, 3.50, 2.00])
                    .p(85),
                crate::panels::build::row(63, Widget::Knob, "CH4 Pan", [10.50, 7.75, 3.50, 2.00])
                    .p(86),
                crate::panels::build::row(64, Widget::Display, "", [0.00, 9.75, 14.00, 0.80])
                    .h(Size::U(0.80)),
            ],
            ..Case::DEFAULT
        },
    ],
};
