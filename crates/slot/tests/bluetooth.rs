// Main menu → Network → Bluetooth, against the pretend radio (crate::bluetooth::SimBt):
// 8BitDo Pro 2 controller, WH-1000XM4 headphones, and Stubborn speaker.

mod common;

use std::time::{Duration, Instant};

use common::{boot, tmp_root_with_carts};
use slot::app::App;
use slot_input::{Action, Btn};
use slot_store::read_extras;
use slot_ui::{QuickRow, QuickValue};

fn press(a: &mut App, btn: Btn) {
    a.apply(Action::GbaDown(btn));
    a.apply(Action::GbaUp(btn));
}

fn go(a: &mut App, row: QuickRow) {
    for _ in 0..15 {
        if a.quick_menu() == Some(row) {
            return;
        }
        press(a, Btn::Down);
    }
    panic!("no {row:?} on the page, at {:?}", a.quick_menu());
}

fn network_page(a: &mut App) {
    a.apply(Action::QuickMenu);
    go(a, QuickRow::Network);
    press(a, Btn::A);
}

fn until(a: &mut App, what: &str, done: impl Fn(&App) -> bool) {
    let deadline = Instant::now() + Duration::from_secs(8);
    while !done(a) {
        assert!(Instant::now() < deadline, "waited too long for {what}");
        a.update(0.016);
        std::thread::sleep(Duration::from_millis(10));
    }
}

fn rows(a: &App) -> Vec<(String, Option<String>)> {
    a.overlay_page()
        .expect("the Bluetooth Devices page is not open")
        .rows()
        .iter()
        .map(|r| (r.label.clone(), r.value.clone()))
        .collect()
}

fn lit(a: &mut App, name: &str) {
    for _ in 0..8 {
        let page = a.overlay_page().unwrap();
        if page.selected().map(|r| r.label.as_str()) == Some(name) {
            return;
        }
        press(a, Btn::Down);
    }
    panic!("no {name} in {:?}", rows(a));
}

fn on_device_list() -> (tempfile::TempDir, App) {
    let d = tmp_root_with_carts(&["Emerald", "Fusion"]);
    let mut a = boot(d.path());
    network_page(&mut a);
    go(&mut a, QuickRow::Bluetooth);
    assert_eq!(a.quick_value(QuickRow::Bluetooth), Some(QuickValue::Off));
    // Devices row is hidden while Bluetooth is off
    assert!(
        !QuickRow::NETWORK[..QuickRow::NETWORK.len()]
            .iter()
            .any(|&r| r == QuickRow::BtDevices && !a.hidden_rows().contains(&r)),
        "BtDevices should be hidden when Bluetooth is off"
    );
    // Turn on
    press(&mut a, Btn::Right);
    assert_eq!(read_extras(d.path()).get("bluetooth"), Some("1"));
    assert_eq!(a.quick_value(QuickRow::Bluetooth), Some(QuickValue::On));
    // Devices row appears
    go(&mut a, QuickRow::BtDevices);
    press(&mut a, Btn::A);
    until(&mut a, "search for nearby devices", |a| {
        a.bt_view().devices.len() == 3
    });
    (d, a)
}

#[test]
fn bluetooth_toggle_saves_to_extras_and_shows_devices() {
    let (_d, a) = on_device_list();
    assert!(a.overlay_page().is_some());
}

#[test]
fn bluetooth_searches_and_pairs_headphones_routing_audio() {
    let (_d, mut a) = on_device_list();
    lit(&mut a, "WH-1000XM4");
    press(&mut a, Btn::A);
    until(&mut a, "headphones to connect", |a| {
        a.bt_view().audio().is_some()
    });
    let r = rows(&a);
    assert_eq!(r[0].0, "WH-1000XM4");
    assert_eq!(r[0].1.as_deref(), Some("Connected"));
}

#[test]
fn bluetooth_device_that_will_not_pair_shows_error() {
    let (_d, mut a) = on_device_list();
    lit(&mut a, "Stubborn");
    press(&mut a, Btn::A);
    until(&mut a, "error to show", |a| a.bt_view().error.is_some());
    assert_eq!(
        a.bt_view().error.as_deref(),
        Some("Could not connect to Stubborn")
    );
}

#[test]
fn bluetooth_x_forgets_paired_device() {
    let (_d, mut a) = on_device_list();
    lit(&mut a, "8BitDo Pro 2");
    press(&mut a, Btn::A);
    until(&mut a, "controller to pair", |a| {
        a.bt_view().devices.iter().any(|d| d.paired)
    });
    press(&mut a, Btn::X);
    until(&mut a, "controller to be forgotten", |a| {
        !a.bt_view().devices.iter().any(|d| d.paired)
    });
}

#[test]
fn bluetooth_b_closes_overlay_back_to_quick_menu() {
    let (_d, mut a) = on_device_list();
    assert!(a.overlay_page().is_some());
    press(&mut a, Btn::B);
    assert!(a.overlay_page().is_none());
    assert_eq!(a.quick_menu(), Some(QuickRow::BtDevices));
}
