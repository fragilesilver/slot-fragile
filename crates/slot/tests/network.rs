// Main menu → Network, against the pretend radio (crate::wifi::SimWifi): Cafe Guest is open,
// Home-5G takes the password "password", Office-EAP cannot be joined.

mod common;

use std::time::{Duration, Instant};

use common::{boot, tmp_root_with_carts};
use slot::app::App;
use slot_input::{Action, Btn};
use slot_store::read_extras;
use slot_ui::{Key, QuickRow, QuickValue};

fn press(a: &mut App, btn: Btn) {
    a.apply(Action::GbaDown(btn));
    a.apply(Action::GbaUp(btn));
}

fn go(a: &mut App, row: QuickRow) {
    for _ in 0..12 {
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
    assert_eq!(a.quick_menu(), Some(QuickRow::WifiPower));
}

fn until(a: &mut App, what: &str, done: impl Fn(&App) -> bool) {
    let deadline = Instant::now() + Duration::from_secs(8);
    while !done(a) {
        assert!(Instant::now() < deadline, "waited too long for {what}");
        a.update(0.016);
        std::thread::sleep(Duration::from_millis(10));
    }
}

fn wifi_on(a: &mut App) {
    network_page(a);
    press(a, Btn::Right);
    until(a, "the scan", |a| !a.wifi_view().networks.is_empty());
}

fn rows(a: &App) -> Vec<(String, Option<String>)> {
    a.overlay_page()
        .expect("the Networks page is not open")
        .rows()
        .iter()
        .map(|r| (r.label.clone(), r.value.clone()))
        .collect()
}

fn lit(a: &mut App, ssid: &str) {
    for _ in 0..8 {
        let page = a.overlay_page().unwrap();
        if page.selected().map(|r| r.label.as_str()) == Some(ssid) {
            return;
        }
        press(a, Btn::Down);
    }
    panic!("no {ssid} in {:?}", rows(a));
}

#[test]
fn the_page_shows_only_the_switch_and_the_clock_while_the_wifi_is_off() {
    let d = tmp_root_with_carts(&["Emerald", "Fusion"]);
    let mut a = boot(d.path());
    network_page(&mut a);
    assert_eq!(a.quick_value(QuickRow::WifiPower), Some(QuickValue::Off));
    for row in [
        QuickRow::WifiNetworks,
        QuickRow::WifiStatus,
        QuickRow::WifiIp,
    ] {
        assert!(
            a.hidden_rows().contains(&row),
            "{row:?} shows with the Wi-Fi off"
        );
    }
    assert_eq!(a.named_value(QuickRow::TimeSync).as_deref(), Some("Off"));
    go(&mut a, QuickRow::TimeSync);
    press(&mut a, Btn::Right);
    assert_eq!(
        a.named_value(QuickRow::TimeSync).as_deref(),
        Some("On, waiting")
    );
}

#[test]
fn an_open_network_joins_from_the_list_and_the_page_says_where() {
    let d = tmp_root_with_carts(&["Emerald", "Fusion"]);
    let mut a = boot(d.path());
    wifi_on(&mut a);
    assert_eq!(read_extras(d.path()).get("wifi"), Some("1"));
    assert_eq!(
        a.named_value(QuickRow::WifiStatus).as_deref(),
        Some("Not connected")
    );
    go(&mut a, QuickRow::WifiNetworks);
    press(&mut a, Btn::A);
    let names: Vec<String> = rows(&a).into_iter().map(|r| r.0).collect();
    assert_eq!(names, ["Home-5G", "Cafe Guest", "Office-EAP", "Neighbour"]);
    lit(&mut a, "Cafe Guest");
    press(&mut a, Btn::A);
    until(&mut a, "the join", |a| a.wifi_view().connected.is_some());
    assert_eq!(
        a.named_value(QuickRow::WifiStatus).as_deref(),
        Some("Cafe Guest")
    );
    assert_eq!(
        a.named_value(QuickRow::WifiIp).as_deref(),
        Some("192.168.1.42")
    );
    until(&mut a, "the row", |a| {
        rows(a).contains(&("Cafe Guest".into(), Some("Connected".into())))
    });
}

fn hit(a: &mut App, want: Key) {
    for _ in 0..5 {
        for _ in 0..10 {
            if a.network_keyboard_key() == Some(want) {
                return press(a, Btn::A);
            }
            press(a, Btn::Right);
        }
        press(a, Btn::Down);
    }
    panic!("no {want:?} on the keyboard");
}

fn type_in(a: &mut App, text: &str) {
    for c in text.chars() {
        hit(a, Key::Char(c));
    }
}

#[test]
fn a_secured_network_asks_for_its_password_and_a_short_one_is_refused() {
    let d = tmp_root_with_carts(&["Emerald", "Fusion"]);
    let mut a = boot(d.path());
    wifi_on(&mut a);
    go(&mut a, QuickRow::WifiNetworks);
    press(&mut a, Btn::A);
    lit(&mut a, "Home-5G");
    press(&mut a, Btn::A);
    assert_eq!(a.network_password(), Some(""), "no keyboard for a password");
    type_in(&mut a, "pass");
    press(&mut a, Btn::Start);
    assert_eq!(
        a.network_password(),
        Some("pass"),
        "a 4 letter password went to the radio"
    );
    type_in(&mut a, "word");
    press(&mut a, Btn::Start);
    assert_eq!(a.network_password(), None);
    until(&mut a, "the join", |a| a.wifi_view().connected.is_some());
    assert_eq!(a.wifi_view().connected.as_deref(), Some("Home-5G"));
    until(&mut a, "Saved", |a| {
        a.wifi_view()
            .networks
            .iter()
            .any(|n| n.ssid == "Home-5G" && n.saved)
    });
    press(&mut a, Btn::X);
    until(&mut a, "forgetting", |a| a.wifi_view().connected.is_none());
    press(&mut a, Btn::B);
    assert!(a.overlay_page().is_none());
    assert_eq!(a.quick_menu(), Some(QuickRow::WifiNetworks));
}

#[test]
fn the_wifi_comes_back_on_at_boot_when_it_was_left_on() {
    let d = tmp_root_with_carts(&["Emerald", "Fusion"]);
    let mut e = read_extras(d.path());
    e.set_flag("wifi", true);
    slot_store::write_extras(d.path(), &e).unwrap();
    let mut a = boot(d.path());
    until(&mut a, "the radio", |a| a.wifi_view().on);
}
