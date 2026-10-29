use super::*;
use eframe::egui::{Event, PointerButton, Rect, pos2, vec2};
#[test]
fn integration_panels_open_with_controls_on_every_page_at_mobile_densities() {
    for (id, pages, height) in [(53, 4, 9.99), (54, 5, 10.99)] {
        for density in [1.0, 2.0, 3.0] {
            let rect = Rect::from_min_size(pos2(0.0, 0.0), vec2(14.0 * U, height * U));
            let mut rig = Rig::with_ppp(id, panel_for(id).unwrap().clone(), rect, density);
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
#[test]
fn wave_editor_interpolates_a_drag_and_stores_real_four_bit_values() {
    let rect = Rect::from_min_size(pos2(0.0, 0.0), vec2(14.0 * U, 10.99 * U));
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
