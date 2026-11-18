mod common;

use common::{app_playing_in, boot, tmp_root_with_carts};
use slot::app::App;
use slot::core::motion_blur_option;
use slot_input::{Action, Btn};
use slot_store::{read_extras, Core, Platform};
use slot_ui::{QuickRow, QuickValue};

#[test]
fn core_frame_blending_is_cleared_in_favor_of_gpu_blending() {
    assert_eq!(
        motion_blur_option(Core::Mgba, true),
        Some(("mgba_interframe_blending", "OFF"))
    );
    assert_eq!(
        motion_blur_option(Core::Mgba, false),
        Some(("mgba_interframe_blending", "OFF"))
    );
    assert_eq!(
        motion_blur_option(Core::Gpsp, true),
        Some(("gpsp_frame_mixing", "disabled"))
    );
    assert_eq!(
        motion_blur_option(Core::Gpsp, false),
        Some(("gpsp_frame_mixing", "disabled"))
    );
}

#[test]
fn a_loading_core_receives_gpu_frame_blending_and_keeps_core_off() {
    let d = tmp_root_with_carts(&["Emerald"]);
    let mut e = read_extras(d.path());
    e.set_flag("motion_blur_gba", true);
    slot_store::write_extras(d.path(), &e).unwrap();
    let mut a = app_playing_in(d.path(), "Emerald");
    assert_eq!(a.frame_blend(), slot_gfx::FrameBlend::Ghosting);
    assert_eq!(
        a.take_motion_blur(),
        Some(false),
        "CPU core blending was not turned off"
    );
    assert_eq!(a.take_motion_blur(), None, "it was handed over twice");
}

#[test]
fn the_row_steps_blending_modes_and_persists_per_platform() {
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
    assert_eq!(
        a.quick_value(QuickRow::MotionBlurGba),
        Some(QuickValue::Mix)
    );
    assert!(!a.motion_blur(Platform::Gb), "GB followed GBA");
    assert_eq!(read_extras(d.path()).get("blending.gba"), Some("mix"));
    a.apply(Action::GbaDown(Btn::Right));
    assert_eq!(
        a.quick_value(QuickRow::MotionBlurGba),
        Some(QuickValue::Ghosting)
    );
    assert_eq!(read_extras(d.path()).get("blending.gba"), Some("ghosting"));
    assert_eq!(a.take_motion_blur(), Some(false));
}
