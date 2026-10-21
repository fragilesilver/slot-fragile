#![cfg(feature = "cheevos")]

// Network → RetroAchievements and Account, against a fake server on this machine, so nothing
// reaches retroachievements.org.

mod common;

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::time::{Duration, Instant};

use common::{app_playing_in, boot, tmp_root_with_carts};
use slot::app::App;
use slot_input::{Action, Btn};
use slot_store::read_extras;
use slot_ui::{Key, QuickRow, QuickValue};

const LOGIN: &str = r#"{"Success":true,"User":"Player","Token":"ApiToken","Score":1,"SoftcoreScore":1,"Messages":0,"Permissions":1,"AccountType":"Registered"}"#;

fn serve() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    std::thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            let mut reader = BufReader::new(stream.try_clone().unwrap());
            let mut len = 0;
            loop {
                let mut line = String::new();
                if reader.read_line(&mut line).unwrap_or(0) == 0 || line == "\r\n" {
                    break;
                }
                if let Some(v) = line.to_ascii_lowercase().strip_prefix("content-length:") {
                    len = v.trim().parse().unwrap_or(0);
                }
            }
            let mut body = vec![0; len];
            let _ = reader.read_exact(&mut body);
            let body = String::from_utf8_lossy(&body).into_owned();
            let reply = match body.contains("r=login2") {
                true => LOGIN,
                false => r#"{"Success":true}"#,
            };
            let mut out = stream;
            let _ = write!(
                out,
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                reply.len(),
                reply
            );
        }
    });
    format!("http://{addr}")
}

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
    panic!("no {row:?}, at {:?}", a.quick_menu());
}

fn network_page(a: &mut App) {
    a.apply(Action::QuickMenu);
    go(a, QuickRow::Network);
    press(a, Btn::A);
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
fn achievements_start_on_and_the_switch_is_kept() {
    let d = tmp_root_with_carts(&["Emerald", "Fusion"]);
    let mut a = boot(d.path());
    network_page(&mut a);
    assert_eq!(a.quick_value(QuickRow::Cheevos), Some(QuickValue::On));
    assert_eq!(
        a.named_value(QuickRow::CheevosAccount).as_deref(),
        Some("Sign in")
    );
    go(&mut a, QuickRow::Cheevos);
    press(&mut a, Btn::Right);
    assert_eq!(read_extras(d.path()).get("cheevos"), Some("0"));
}

#[test]
fn signing_in_on_the_keyboard_writes_the_account_and_keeps_only_the_token() {
    let d = tmp_root_with_carts(&["Emerald", "Fusion"]);
    let host = serve();
    std::fs::write(
        d.path().join("Config/retroachievements.txt"),
        format!("host={host}\n"),
    )
    .unwrap();
    let mut a = boot(d.path());
    network_page(&mut a);
    go(&mut a, QuickRow::CheevosAccount);
    press(&mut a, Btn::A);
    assert_eq!(
        a.network_password(),
        Some(""),
        "no keyboard for the username"
    );
    press(&mut a, Btn::Start);
    assert_eq!(a.network_password(), Some(""), "an empty username went on");
    type_in(&mut a, "PLAYER");
    press(&mut a, Btn::Start);
    assert_eq!(
        a.network_password(),
        Some(""),
        "no keyboard for the password"
    );
    type_in(&mut a, "secret");
    press(&mut a, Btn::Start);
    assert_eq!(a.network_password(), None);
    let end = Instant::now() + Duration::from_secs(10);
    while a.named_value(QuickRow::CheevosAccount).as_deref() != Some("Player") {
        assert!(Instant::now() < end, "never signed in");
        std::thread::sleep(Duration::from_millis(20));
    }
    let kept = std::fs::read_to_string(d.path().join("Config/retroachievements.txt")).unwrap();
    let keys = slot_store::ini::parse(&kept);
    assert_eq!(keys.get("username").map(String::as_str), Some("PLAYER"));
    assert_eq!(keys.get("token").map(String::as_str), Some("ApiToken"));
    assert_eq!(keys.get("host"), Some(&host), "the host line was lost");
    assert!(!keys.contains_key("password"));
    assert!(
        !kept.contains("secret"),
        "the password stayed on the card: {kept}"
    );
}

#[test]
fn the_achievements_row_waits_for_a_game_the_server_knows() {
    let d = tmp_root_with_carts(&["Emerald", "Fusion"]);
    let a = app_playing_in(d.path(), "Emerald");
    assert!(a.in_game_hidden().contains(&QuickRow::PlayAchievements));
}
