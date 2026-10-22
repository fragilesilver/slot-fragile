mod common;

use common::{app_playing_in, boot, tmp_root_with_carts};
use slot::app::App;
use slot::core::motion_blur_option;
use slot_input::{Action, Btn};
use slot_store::{read_extras, Core, Platform};
use slot_ui::{QuickRow, QuickValue};

#[test]
fn each_core_spells_its_frame_blending_its_own_way() {
    assert_eq!(
        motion_blur_option(Core::Mgba, true),
        Some(("mgba_interframe_blending", "lcd_ghosting"))
    );
    assert_eq!(
        motion_blur_option(Core::Mgba, false),
        Some(("mgba_interframe_blending", "OFF"))
    );
    assert_eq!(
        motion_blur_option(Core::Gpsp, true),
        Some(("gpsp_frame_mixing", "enabled"))
    );
    assert_eq!(
        motion_blur_option(Core::Gpsp, false),
        Some(("gpsp_frame_mixing", "disabled"))
    );
}

#[test]
fn a_loading_core_is_handed_the_consoles_motion_blur() {
    let d = tmp_root_with_carts(&["Emerald"]);
    let mut e = read_extras(d.path());
    e.set_flag("motion_blur_gba", true);
    slot_store::write_extras(d.path(), &e).unwrap();
    let mut a = app_playing_in(d.path(), "Emerald");
    assert_eq!(
        a.take_motion_blur(),
        Some(true),
        "the core never heard of it"
    );
    assert_eq!(a.take_motion_blur(), None, "it was handed over twice");
}

#[test]
fn the_row_flips_saves_and_tells_the_running_core() {
    let d = tmp_root_with_carts(&["Emerald", "Fusion"]);
    let mut a = boot(d.path());
    let _ = a.take_motion_blur();
    a.apply(Action::QuickMenu);
    let walk = |a: &mut App, row: QuickRow| {
        for _ in 0..row.position(&a.hidden_rows()) {
            a.apply(Action::GbaDown(Btn::Down));
        }
    };
    walk(&mut a, QuickRow::Screen);
    a.apply(Action::GbaDown(Btn::A));
    walk(&mut a, QuickRow::ScreenGba);
    a.apply(Action::GbaDown(Btn::A));
    walk(&mut a, QuickRow::MotionBlurGba);
    assert_eq!(
        a.quick_value(QuickRow::MotionBlurGba),
        Some(QuickValue::Off)
    );
    a.apply(Action::GbaDown(Btn::Right));
    assert!(a.motion_blur(Platform::Gba));
    assert!(!a.motion_blur(Platform::Gb), "GB followed GBA");
    assert_eq!(read_extras(d.path()).get("motion_blur_gba"), Some("1"));
    assert_eq!(a.take_motion_blur(), Some(true));
}
