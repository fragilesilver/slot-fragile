// Ported from marghost/slot_modded@1706a16, crates/slot/tests/battery_warnings.rs; the badge
// tests at the end are Fragile's.

mod common;

use common::{app_playing_in, boot, tmp_root_with_carts};
use slot::app::{App, Phase};
use slot_input::Action;
use slot_power::{Battery, Charge};
use slot_ui::{Draw, QuickRow, QuickValue, TexId, Toast};

fn draining(percent: u8) -> Battery {
    Battery {
        percent,
        charge: Charge::Discharging,
    }
}

fn charging(percent: u8) -> Battery {
    Battery {
        percent,
        charge: Charge::Charging,
    }
}

fn later(a: &mut App, now: &mut u64) {
    *now += 20_000;
    a.tick_ms(*now);
}

#[test]
fn fifteen_percent_warns_once() {
    let d = tmp_root_with_carts(&["Emerald"]);
    let mut a = app_playing_in(d.path(), "Emerald");
    let mut now = 0;
    a.on_battery(draining(16));
    assert_eq!(a.toast(), None);
    a.on_battery(draining(15));
    assert_eq!(a.toast(), Some(Toast::BatteryLow15));
    later(&mut a, &mut now);
    a.on_battery(draining(14));
    a.on_battery(draining(12));
    assert_eq!(a.toast(), None, "the 15% warning came back");
}

#[test]
fn ten_percent_warns_after_fifteen() {
    let d = tmp_root_with_carts(&["Emerald"]);
    let mut a = app_playing_in(d.path(), "Emerald");
    let mut now = 0;
    a.on_battery(draining(15));
    later(&mut a, &mut now);
    a.on_battery(draining(10));
    assert_eq!(a.toast(), Some(Toast::BatteryLow10));
    later(&mut a, &mut now);
    a.on_battery(draining(8));
    assert_eq!(a.toast(), None);
}

#[test]
fn a_reading_past_both_levels_shows_only_the_lower_one() {
    let d = tmp_root_with_carts(&["Emerald"]);
    let mut a = app_playing_in(d.path(), "Emerald");
    let mut now = 0;
    a.on_battery(draining(9));
    assert_eq!(a.toast(), Some(Toast::BatteryLow10));
    later(&mut a, &mut now);
    a.on_battery(draining(9));
    assert_eq!(a.toast(), None);
}

#[test]
fn charging_never_warns_and_brings_the_warnings_back() {
    let d = tmp_root_with_carts(&["Emerald"]);
    let mut a = app_playing_in(d.path(), "Emerald");
    let mut now = 0;
    a.on_battery(charging(12));
    assert_eq!(a.toast(), None);
    a.on_battery(draining(12));
    assert_eq!(a.toast(), Some(Toast::BatteryLow15));
    later(&mut a, &mut now);
    a.on_battery(charging(13));
    a.on_battery(draining(14));
    assert_eq!(a.toast(), Some(Toast::BatteryLow15));
}

#[test]
fn a_shut_lid_holds_the_warning_until_it_opens() {
    let d = tmp_root_with_carts(&["Emerald"]);
    let mut a = app_playing_in(d.path(), "Emerald");
    a.apply(Action::LidClose);
    a.on_battery(draining(14));
    assert_eq!(a.toast(), None);
    a.apply(Action::LidOpen);
    a.on_battery(draining(14));
    assert_eq!(a.toast(), Some(Toast::BatteryLow15));
}

#[test]
fn the_power_off_level_shows_no_warning() {
    let d = tmp_root_with_carts(&["Emerald"]);
    let mut a = app_playing_in(d.path(), "Emerald");
    a.on_battery(draining(4));
    assert_eq!(a.toast(), None);
    assert!(a.powering_off());
}

const PERCENT: usize = 900;

fn with_faces(a: &mut App, root: &std::path::Path) {
    a.set_battery_percent_face(TexId::from_raw(PERCENT), 40);
    let (power, _) = common::panel_with_battery(root, std::time::Duration::from_secs(300), 0, 64);
    a.set_power(power);
    for _ in 0..180 {
        a.update(1.0 / 60.0);
    }
    assert!(a.battery().is_some(), "the panel gave no reading");
}

fn badge_drawn(a: &App) -> bool {
    let mut out = Vec::new();
    a.draw(&mut out);
    out.iter()
        .any(|d| matches!(d, Draw::Tex { tex, .. } if *tex == TexId::from_raw(PERCENT)))
}

#[test]
fn the_badge_is_off_until_general_turns_it_on() {
    let d = tmp_root_with_carts(&["Emerald", "Fusion"]);
    let mut a = boot(d.path());
    assert!(!a.battery_badge());
    assert!(matches!(a.phase(), Phase::Shelf));
    a.apply(Action::QuickMenu);
    let hidden = a.hidden_rows();
    for _ in 0..QuickRow::General.position(&hidden) {
        a.apply(Action::GbaDown(slot_input::Btn::Down));
    }
    a.apply(Action::GbaDown(slot_input::Btn::A));
    for _ in 0..QuickRow::BatteryBadge.position(&a.hidden_rows()) {
        a.apply(Action::GbaDown(slot_input::Btn::Down));
    }
    assert_eq!(a.quick_menu(), Some(QuickRow::BatteryBadge));
    assert_eq!(a.quick_value(QuickRow::BatteryBadge), Some(QuickValue::Off));
    a.apply(Action::GbaDown(slot_input::Btn::Right));
    assert!(a.battery_badge());
    assert_eq!(
        slot_store::read_extras(d.path()).get("battery_badge"),
        Some("1")
    );
}

#[test]
fn the_badge_shows_over_a_game_bottom_right() {
    let d = tmp_root_with_carts(&["Emerald"]);
    let mut e = slot_store::read_extras(d.path());
    e.set_flag("battery_badge", true);
    slot_store::write_extras(d.path(), &e).unwrap();
    let mut a = app_playing_in(d.path(), "Emerald");
    with_faces(&mut a, d.path());
    let mut out = Vec::new();
    a.draw(&mut out);
    let at = out.iter().find_map(|d| match *d {
        Draw::Tex { x, y, tex, .. } if tex == TexId::from_raw(PERCENT) => Some((x, y)),
        _ => None,
    });
    let (x, y) = at.expect("no badge over the game");
    let canvas = a.canvas();
    assert!(
        x > canvas.w / 2.0 && y > canvas.h / 2.0,
        "the badge is at {x},{y}"
    );
}

#[test]
fn the_badge_stays_off_when_the_setting_is() {
    let d = tmp_root_with_carts(&["Emerald"]);
    let mut a = app_playing_in(d.path(), "Emerald");
    with_faces(&mut a, d.path());
    assert!(!badge_drawn(&a));
}
