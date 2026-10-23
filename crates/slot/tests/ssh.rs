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
    for _ in 0..16 {
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
fn ssh_toggle_persists_and_controls_hidden_password() {
    let d = tmp_root_with_carts(&["Emerald", "Fusion"]);
    let mut a = boot(d.path());
    network_page(&mut a);

    go(&mut a, QuickRow::SshPower);
    assert_eq!(a.quick_value(QuickRow::SshPower), Some(QuickValue::Off));
    assert!(
        a.hidden_rows().contains(&QuickRow::SshPassword),
        "Password row should be hidden while SSH is off"
    );

    // Turn SSH on
    press(&mut a, Btn::Right);
    assert_eq!(read_extras(d.path()).get("ssh"), Some("1"));
    assert_eq!(a.quick_value(QuickRow::SshPower), Some(QuickValue::On));
    assert!(
        !a.hidden_rows().contains(&QuickRow::SshPassword),
        "Password row should be visible while SSH is on"
    );

    // Turn SSH off
    press(&mut a, Btn::Left);
    assert_eq!(read_extras(d.path()).get("ssh"), Some("0"));
    assert_eq!(a.quick_value(QuickRow::SshPower), Some(QuickValue::Off));
    assert!(
        a.hidden_rows().contains(&QuickRow::SshPassword),
        "Password row should be hidden again"
    );
}

#[test]
fn ssh_password_reads_from_baseos_conf() {
    let d = tmp_root_with_carts(&["Emerald", "Fusion"]);
    std::fs::write(
        d.path().join("baseos.conf"),
        "hostname=rg35xx\nssh_password=custom_secret\n",
    )
    .unwrap();

    let mut a = boot(d.path());
    network_page(&mut a);
    go(&mut a, QuickRow::SshPower);
    press(&mut a, Btn::Right); // turn on SSH so password is shown

    assert_eq!(a.ssh_password(), "custom_secret");
    assert_eq!(
        a.named_value(QuickRow::SshPassword).as_deref(),
        Some("custom_secret")
    );
}

#[test]
fn ssh_password_interactive_editing_updates_all_places() {
    let d = tmp_root_with_carts(&["Emerald", "Fusion"]);
    let mut a = boot(d.path());
    network_page(&mut a);
    go(&mut a, QuickRow::SshPower);
    press(&mut a, Btn::Right); // turn on SSH

    go(&mut a, QuickRow::SshPassword);
    assert_eq!(
        a.named_value(QuickRow::SshPassword).as_deref(),
        Some("root")
    );

    // Open keyboard
    press(&mut a, Btn::A);
    assert_eq!(a.network_password(), Some("root"));

    // Backspace delete "root" (4 letters)
    for _ in 0..4 {
        press(&mut a, Btn::B);
    }
    assert_eq!(a.network_password(), Some(""));

    // Type new password "safe"
    type_in(&mut a, "safe");
    assert_eq!(a.network_password(), Some("safe"));

    // Confirm with Start
    press(&mut a, Btn::Start);
    assert!(
        a.network_password().is_none(),
        "keyboard should close on confirm"
    );

    // Check updated in App, extras.state, and baseos.conf
    assert_eq!(a.ssh_password(), "safe");
    assert_eq!(
        a.named_value(QuickRow::SshPassword).as_deref(),
        Some("safe")
    );
    assert_eq!(read_extras(d.path()).get("ssh_password"), Some("safe"));

    let conf = std::fs::read_to_string(d.path().join("baseos.conf")).unwrap();
    assert!(conf.contains("ssh_password=safe"));
}

#[test]
fn ssh_empty_password_rejected_by_keyboard() {
    let d = tmp_root_with_carts(&["Emerald", "Fusion"]);
    let mut a = boot(d.path());
    network_page(&mut a);
    go(&mut a, QuickRow::SshPower);
    press(&mut a, Btn::Right);

    go(&mut a, QuickRow::SshPassword);
    press(&mut a, Btn::A);

    // Backspace delete "root"
    for _ in 0..4 {
        press(&mut a, Btn::B);
    }
    assert_eq!(a.network_password(), Some(""));

    // Confirm with Start on empty text
    press(&mut a, Btn::Start);
    assert_eq!(
        a.network_password(),
        Some(""),
        "keyboard should remain open when password is empty"
    );

    // Cancel with B on empty text
    press(&mut a, Btn::B);
    assert!(
        a.network_password().is_none(),
        "keyboard cancels on B when empty"
    );
}

#[test]
fn ssh_dynamic_connection_string() {
    let d = tmp_root_with_carts(&["Emerald", "Fusion"]);
    let mut a = boot(d.path());
    network_page(&mut a);

    // Wi-Fi is off: no connection string
    assert_eq!(a.ssh_connection_string(), None);

    // Turn Wi-Fi on
    go(&mut a, QuickRow::WifiPower);
    press(&mut a, Btn::Right);
    until(&mut a, "the scan", |a| !a.wifi_view().networks.is_empty());

    // Join open network "Cafe Guest"
    go(&mut a, QuickRow::WifiNetworks);
    press(&mut a, Btn::A);

    for _ in 0..8 {
        let page = a.overlay_page().unwrap();
        if page.selected().map(|r| r.label.as_str()) == Some("Cafe Guest") {
            break;
        }
        press(&mut a, Btn::Down);
    }
    press(&mut a, Btn::A);
    until(&mut a, "the join", |a| a.wifi_view().connected.is_some());

    assert_eq!(a.wifi_view().ip.as_deref(), Some("192.168.1.42"));
    assert_eq!(
        a.ssh_connection_string().as_deref(),
        Some("ssh root@192.168.1.42 (port 22, SFTP enabled)")
    );
}
