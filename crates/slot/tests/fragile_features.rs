mod common;

use common::{clocked, tmp_root_with_carts};
use slot::app::{App, Phase};
use slot_input::{Action, Btn};
use slot_store::{read_slot_state, AutoLoad, KidMode};
use slot_ui::{QuickRow, QuickValue};

fn press(a: &mut App, b: Btn) {
    a.apply(Action::GbaDown(b));
    a.apply(Action::GbaUp(b));
}

fn open_at(a: &mut App, row: QuickRow) {
    a.apply(Action::QuickMenu);
    let mut path = vec![row];
    while let Some(parent) = path.last().and_then(|r| r.parent()) {
        path.push(parent);
    }
    for (depth, step) in path.iter().rev().enumerate() {
        if depth > 0 {
            press(a, Btn::A);
        }
        for _ in 0..step.position(&a.hidden_rows()) {
            press(a, Btn::Down);
        }
    }
}

#[test]
fn auto_load_row_cycles_values_and_persists() {
    let d = tmp_root_with_carts(&["Emerald", "Fusion"]);
    clocked(d.path());
    let mut a = App::boot(d.path());

    open_at(&mut a, QuickRow::AutoLoad);
    assert_eq!(a.quick_menu(), Some(QuickRow::AutoLoad));

    // Default is Always
    assert_eq!(a.quick_value(QuickRow::AutoLoad), Some(QuickValue::Always));

    // Step right -> PowerOn
    press(&mut a, Btn::Right);
    assert_eq!(a.quick_value(QuickRow::AutoLoad), Some(QuickValue::PowerOn));
    assert_eq!(read_slot_state(d.path()).auto_load, AutoLoad::PowerOn);

    // Step right -> Never
    press(&mut a, Btn::Right);
    assert_eq!(a.quick_value(QuickRow::AutoLoad), Some(QuickValue::Never));
    assert_eq!(read_slot_state(d.path()).auto_load, AutoLoad::Never);

    // Step right -> wraps back to Always
    press(&mut a, Btn::Right);
    assert_eq!(a.quick_value(QuickRow::AutoLoad), Some(QuickValue::Always));
    assert_eq!(read_slot_state(d.path()).auto_load, AutoLoad::Always);
}

#[test]
fn kid_mode_row_cycles_values_and_locks_menus_until_unlocked() {
    let d = tmp_root_with_carts(&["Emerald", "Fusion"]);
    clocked(d.path());
    let mut a = App::boot(d.path());

    open_at(&mut a, QuickRow::KidMode);
    assert_eq!(a.quick_menu(), Some(QuickRow::KidMode));

    // Default is Off
    assert_eq!(a.quick_value(QuickRow::KidMode), Some(QuickValue::Off));

    // Step right -> OneShelf
    press(&mut a, Btn::Right);
    assert_eq!(a.quick_value(QuickRow::KidMode), Some(QuickValue::OneShelf));
    assert_eq!(read_slot_state(d.path()).kid_mode, KidMode::OneShelf);

    // Exit menu back to shelf
    press(&mut a, Btn::B);
    press(&mut a, Btn::B);
    assert!(matches!(a.phase(), Phase::Shelf));

    // When KidMode is active, Menu/QuickMenu cannot be opened!
    press(&mut a, Btn::Menu);
    assert!(matches!(a.phase(), Phase::Shelf));
    a.apply(Action::QuickMenu);
    assert!(matches!(a.phase(), Phase::Shelf));

    // And on OneShelf, shelf switching is ignored
    let shelf_before = a.shelf_at();
    a.apply(Action::ShelfRight);
    a.apply(Action::NextShelf);
    assert_eq!(a.shelf_at(), shelf_before);

    // Unlocking via Start+Select+L2+R2 chord unlocks KidMode
    a.apply(Action::UnlockKidMode);
    assert_eq!(read_slot_state(d.path()).kid_mode, KidMode::Off);

    // Now QuickMenu can open again!
    a.apply(Action::QuickMenu);
    assert!(matches!(a.phase(), Phase::QuickMenu { .. }));
}

#[test]
fn local_rtc_tz_is_configured_from_utc_offset() {
    slot::app::tell_libc_the_offset(480); // UTC+8
    assert_eq!(std::env::var("TZ").unwrap_or_default(), "UTC-8");

    slot::app::tell_libc_the_offset(-300); // UTC-5
    assert_eq!(std::env::var("TZ").unwrap_or_default(), "UTC+5");

    slot::app::tell_libc_the_offset(330); // UTC+5:30
    assert_eq!(std::env::var("TZ").unwrap_or_default(), "UTC-5:30");
}
