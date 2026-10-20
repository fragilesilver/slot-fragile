// Sleep Power Off: a device with no lid sleeps on a press of the power button, and powers off
// after the minutes set here, or never.

mod common;

use std::time::Duration;

use common::{app_playing_in, boot, panel, tmp_root_with_carts};
use slot::app::{App, Phase};
use slot_input::{Action, Btn};
use slot_store::{read_extras, write_extras, Core, Platform, StateRing};
use slot_ui::{QuickRow, QuickValue};

fn press(a: &mut App, btn: Btn) {
    a.apply(Action::GbaDown(btn));
    a.apply(Action::GbaUp(btn));
}

fn sleep_for(a: &mut App, minutes: f32) {
    for _ in 0..(minutes * 60.0 * 60.0) as usize {
        a.update(1.0 / 60.0);
    }
}

fn lidless(d: &tempfile::TempDir, sleep_off: Option<&str>) -> App {
    let mut e = read_extras(d.path());
    e.set("sleep_off_min", sleep_off);
    write_extras(d.path(), &e).unwrap();
    let mut a = app_playing_in(d.path(), "Emerald");
    a.set_lid(false);
    a.set_power(panel(d.path(), Duration::from_secs(180)).0);
    a
}

#[test]
fn the_power_button_sleeps_and_saves_the_game_first() {
    let d = tmp_root_with_carts(&["Emerald"]);
    let mut a = lidless(&d, None);
    a.apply(Action::PowerTap);
    assert!(matches!(a.phase(), Phase::Doze { .. }), "{:?}", a.phase());
    let ring = StateRing::new(d.path(), Platform::Gba, Core::Mgba, "Emerald");
    assert!(
        ring.read_resume().unwrap().is_some(),
        "slept with the game unsaved"
    );
}

#[test]
fn a_sleeping_device_powers_off_after_the_minutes_set() {
    let d = tmp_root_with_carts(&["Emerald"]);
    let mut a = lidless(&d, Some("1"));
    a.apply(Action::PowerTap);
    sleep_for(&mut a, 59.0 / 60.0);
    assert!(!a.powering_off(), "59 s is short of the minute");
    sleep_for(&mut a, 2.0 / 60.0);
    assert!(a.powering_off());
}

#[test]
fn never_sleeps_until_woken() {
    let d = tmp_root_with_carts(&["Emerald"]);
    let mut a = lidless(&d, Some("never"));
    a.apply(Action::PowerTap);
    sleep_for(&mut a, 31.0);
    assert!(!a.powering_off(), "Never powered off");
    assert!(matches!(a.phase(), Phase::Doze { .. }));
}

#[test]
fn unset_it_follows_the_lids_minutes_and_a_bad_value_does_too() {
    let d = tmp_root_with_carts(&["Emerald"]);
    for value in [None, Some("7"), Some("soon")] {
        let a = lidless(&d, value);
        assert_eq!(a.sleep_off_min(), Some(a.lid_off_min()), "{value:?}");
    }
}

#[test]
fn a_clamshell_keeps_lid_power_off_and_ignores_never() {
    let d = tmp_root_with_carts(&["Emerald"]);
    let mut a = lidless(&d, Some("never"));
    a.set_lid(true);
    assert!(a.hidden_rows().contains(&QuickRow::SleepOff));
    assert!(!a.hidden_rows().contains(&QuickRow::LidOff));
    a.apply(Action::LidClose);
    let minutes = a.lid_off_min() as f32 + 0.1;
    sleep_for(&mut a, minutes);
    assert!(a.powering_off(), "Never reached a clamshell");
}

#[test]
fn the_menu_shows_sleep_power_off_and_steps_through_never() {
    let d = tmp_root_with_carts(&["Emerald", "Fusion"]);
    let mut a = boot(d.path());
    a.set_lid(false);
    assert!(a.hidden_rows().contains(&QuickRow::LidOff));
    a.apply(Action::QuickMenu);
    for _ in 0..QuickRow::General.position(&a.hidden_rows()) {
        press(&mut a, Btn::Down);
    }
    press(&mut a, Btn::A);
    for _ in 0..QuickRow::SleepOff.position(&a.hidden_rows()) {
        press(&mut a, Btn::Down);
    }
    assert_eq!(a.quick_menu(), Some(QuickRow::SleepOff));
    let three = QuickValue::LID_OFF
        .into_iter()
        .find(|v| v.minutes() == Some(a.lid_off_min()));
    assert_eq!(a.quick_value(QuickRow::SleepOff), three);
    for _ in 0..QuickValue::LID_OFF.len() {
        press(&mut a, Btn::Left);
        if a.quick_value(QuickRow::SleepOff) == Some(QuickValue::Never) {
            break;
        }
    }
    assert_eq!(a.quick_value(QuickRow::SleepOff), Some(QuickValue::Never));
    assert_eq!(read_extras(d.path()).get("sleep_off_min"), Some("never"));
    press(&mut a, Btn::Left);
    assert_eq!(
        a.quick_value(QuickRow::SleepOff),
        QuickValue::LID_OFF.last().copied(),
        "Left from Never does not go round to the longest"
    );
}
