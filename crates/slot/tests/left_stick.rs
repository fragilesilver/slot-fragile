mod common;

use std::path::Path;

use common::{app_booting_at, tmp_root_with_carts, CLOCK_IS_SET};
use slot::app::App;
use slot_input::{Action, Btn};
use slot_store::{read_slot_state, write_slot_state, SlotState};
use slot_ui::{QuickRow, QuickValue};

fn boot(root: &Path, stick: bool) -> App {
    write_slot_state(
        root,
        &SlotState {
            clock_set: true,
            ..SlotState::default()
        },
    )
    .expect("write slot.state");
    let (mut a, _) = app_booting_at(root, CLOCK_IS_SET);
    a.set_left_stick(stick);
    a
}

fn press(a: &mut App, btn: Btn) {
    a.apply(Action::GbaDown(btn));
    a.apply(Action::GbaUp(btn));
}

#[test]
fn a_device_without_a_stick_hides_the_row_and_never_steers() {
    let d = tmp_root_with_carts(&["Emerald", "Fusion"]);
    let a = boot(d.path(), false);
    assert!(a.hidden_rows().contains(&QuickRow::LeftStick));
    assert!(a.in_game_hidden().contains(&QuickRow::LeftStick));
    assert!(!a.left_stick_dpad());
}

#[test]
fn the_stick_is_turned_off_from_gameplay_and_saved() {
    let d = tmp_root_with_carts(&["Emerald", "Fusion"]);
    let mut a = boot(d.path(), true);
    assert!(a.left_stick_dpad());
    a.apply(Action::QuickMenu);
    for _ in 0..QuickRow::Game.position(&[]) {
        press(&mut a, Btn::Down);
    }
    press(&mut a, Btn::A);
    for _ in 0..QuickRow::LeftStick.position(&a.hidden_rows()) {
        press(&mut a, Btn::Down);
    }
    assert_eq!(a.quick_menu(), Some(QuickRow::LeftStick));
    assert_eq!(a.quick_value(QuickRow::LeftStick), Some(QuickValue::On));
    press(&mut a, Btn::Right);
    assert_eq!(a.quick_value(QuickRow::LeftStick), Some(QuickValue::Off));
    assert!(!a.left_stick_dpad());
    assert!(
        !read_slot_state(d.path()).left_stick_dpad,
        "the choice was not saved"
    );
}

#[test]
fn nds_left_stick_stylus_pauses_dpad_emulation_while_playing() {
    let d = tmp_root_with_carts(&["Emerald"]);
    std::fs::create_dir_all(d.path().join("Games/NDS")).expect("NDS dir");
    std::fs::write(d.path().join("Games/NDS/Canvas.nds"), vec![0u8; 0x200]).expect("write nds");
    write_slot_state(
        d.path(),
        &SlotState {
            clock_set: true,
            cart: Some("Canvas".into()),
            cart_platform: Some(slot_store::Platform::Nds),
            ..SlotState::default()
        },
    )
    .expect("write slot.state");
    let mut a = App::boot(d.path());
    a.set_left_stick(true);
    a.set_platform(slot_store::Platform::Nds);
    a.on_core_ready();
    for _ in 0..120 {
        a.update(1.0 / 60.0);
    }
    assert!(a.left_stick_dpad());
    assert!(!a.left_stick_is_stylus());

    // Step NDS Stylus from default "Hold L2" -> "Right Stick" -> "Left Stick".
    a.step_nds_stylus(false, true);
    assert_eq!(a.nds_stylus_core_value(), "Right Analog");
    assert!(!a.left_stick_is_stylus());

    a.step_nds_stylus(false, true);
    assert_eq!(a.nds_stylus_core_value(), "Left Analog");
    assert!(a.left_stick_is_stylus());

    // Opening the in-game pause menu re-enables Left Stick navigation for menus.
    a.apply(Action::QuickMenu);
    assert!(!a.left_stick_is_stylus());
}
