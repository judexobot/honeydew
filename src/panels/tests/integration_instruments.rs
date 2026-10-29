use super::*;
use eframe::egui::{Event, PointerButton, Rect, pos2, vec2};
#[test]
fn integration_panels_open_with_controls_on_every_page_at_mobile_densities() {
    for (id, pages, height) in [(53, 4, 9.99), (54, 5, 11.99), (55, 8, 13.99)] {
        for model in 0..if id == 55 { 2 } else { 1 } {
            for density in [1.0, 2.0, 3.0] {
                let rect = Rect::from_min_size(pos2(0.0, 0.0), vec2(14.0 * U, height * U));
                let mut rig = Rig::with_ppp(id, panel_for(id).unwrap().clone(), rect, density);
                if id == 55 {
                    rig.module.params[56] = model as f32;
                    rig.module.params[58] = model as f32;
                }
                for tab in 0..pages {
                    rig.state.tab = tab;
                    rig.frame(vec![]);
                    let leaves = place(&rig.panel, &rig.module, &rig.state, rect, U);
                    assert!(leaves.len() > 4);
                    for (_, r) in leaves {
                        assert!(r.width() > 0.0 && r.height() > 0.0);
                        assert!(rect.expand(1.0).contains_rect(r));
                    }
                }
            }
        }
    }
}
#[test]
fn wave_editor_interpolates_a_drag_and_stores_real_four_bit_values() {
    let rect = Rect::from_min_size(pos2(0.0, 0.0), vec2(14.0 * U, 11.99 * U));
    let mut rig = Rig::new(54, panel_for(54).unwrap().clone(), rect);
    rig.state.tab = 2;
    rig.frame(vec![]);
    let wave = rig.rect_of(41).shrink2(vec2(0.2 * U, 0.15 * U));
    let start = pos2(wave.left() + 1.0, wave.top() + 1.0);
    let end = pos2(wave.right() - 1.0, wave.bottom() - 1.0);
    rig.frame(vec![Event::PointerMoved(start)]);
    rig.press(start, PointerButton::Primary, true);
    rig.frame(vec![Event::PointerMoved(end)]);
    rig.press(end, PointerButton::Primary, false);
    assert_eq!(rig.module.params[26], 1.0);
    let wave = &rig.module.params[27..59];
    assert!(wave[0] > 0.9 && wave[31] < 0.1);
    assert!((wave[16] - 0.5).abs() < 0.1);
    assert!(
        wave.iter()
            .all(|v| (*v * 15.0 - (*v * 15.0).round()).abs() < 1e-5)
    );
}
#[test]
fn editing_a_builtin_scale_selects_custom_and_keeps_the_other_notes() {
    let rect = Rect::from_min_size(pos2(0.0, 0.0), vec2(14.0 * U, 9.99 * U));
    let mut rig = Rig::new(53, panel_for(53).unwrap().clone(), rect);
    rig.module.params[2] = 0.2;
    rig.state.tab = 1;
    rig.frame(vec![]);
    rig.frame(vec![]);
    assert_eq!(rig.module.params[13], 0.0);
    let c_sharp = rig.rect_of(15).center();
    rig.frame(vec![Event::PointerMoved(c_sharp)]);
    rig.press(c_sharp, PointerButton::Primary, true);
    rig.press(c_sharp, PointerButton::Primary, false);
    assert_eq!(rig.module.params[2], 1.0);
    assert_eq!(rig.module.params[13], 1.0);
    assert_eq!(rig.module.params[15], 0.0);
}

fn custom_button(rig: &mut Rig, display: u32, index: usize, count: usize) {
    let rect = rig.rect_of(display);
    let point = pos2(
        rect.left() + (index as f32 + 0.5) * rect.width() / count as f32,
        rect.top() + 0.5 * U,
    );
    rig.frame(vec![Event::PointerMoved(point)]);
    rig.press(point, PointerButton::Primary, true);
    rig.press(point, PointerButton::Primary, false);
}
#[test]
fn gen_operator_copy_and_patch_ab_edit_the_actual_saved_parameters() {
    let rect = Rect::from_min_size(pos2(0.0, 0.0), vec2(14.0 * U, 13.99 * U));
    let mut rig = Rig::new(55, panel_for(55).unwrap().clone(), rect);
    rig.module.params[56] = 1.0;
    rig.module.params[62] = 0.4;
    rig.module.params[102] = 1.0;
    rig.state.tab = 1;
    rig.frame(vec![]);
    custom_button(&mut rig, 66, 0, 3);
    rig.state.tab = 4;
    rig.frame(vec![]);
    custom_button(&mut rig, 66, 1, 3);
    assert_eq!(rig.module.params[92], 0.4);
    assert_eq!(rig.module.params[105], 1.0);
    rig.state.tab = 7;
    rig.module.params[3] = 0.2;
    rig.frame(vec![]);
    custom_button(&mut rig, 67, 0, 5);
    rig.module.params[3] = 0.7;
    custom_button(&mut rig, 67, 1, 5);
    custom_button(&mut rig, 67, 2, 5);
    assert_eq!(rig.module.params[3], 0.2);
    custom_button(&mut rig, 67, 3, 5);
    assert_eq!(rig.module.params[3], 0.7);
    custom_button(&mut rig, 67, 4, 5);
    assert_eq!(rig.module.params[3], honeydew_audio::r#gen::DEFAULTS[3]);
    assert_eq!(rig.module.params[56], 1.0);
}
#[test]
fn gameboy_wave_copy_reset_and_paste_change_the_real_wave_ram() {
    let rect = Rect::from_min_size(pos2(0.0, 0.0), vec2(14.0 * U, 11.99 * U));
    let mut rig = Rig::new(54, panel_for(54).unwrap().clone(), rect);
    rig.state.tab = 2;
    rig.module.params[26] = 1.0;
    rig.module.params[27..59].fill(1.0);
    rig.frame(vec![]);
    custom_button(&mut rig, 65, 0, 4);
    custom_button(&mut rig, 65, 2, 4);
    assert_eq!(rig.module.params[26], 0.0);
    custom_button(&mut rig, 65, 1, 4);
    assert_eq!(rig.module.params[26], 1.0);
    assert!(rig.module.params[27..59].iter().all(|v| *v == 1.0));
}
