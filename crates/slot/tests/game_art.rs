// Fragile: Main menu → Network → Game Art (Phase 13), against the pretend art servers
// (crate::scrape::SimFetch): a GBA ROM of eight zero bytes is "Pretend Game (USA)", with a logo
// and a box; anything else is unknown.

mod common;

use std::path::Path;
use std::time::{Duration, Instant};

use common::boot;
use slot::app::App;
use slot_input::{Action, Btn};
use slot_ui::{Key, QuickRow};

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

fn game_art(a: &mut App) {
    a.apply(Action::QuickMenu);
    go(a, QuickRow::Network);
    press(a, Btn::A);
    go(a, QuickRow::GameArt);
}

fn until(a: &mut App, what: &str, done: impl Fn(&App) -> bool) {
    let deadline = Instant::now() + Duration::from_secs(10);
    while !done(a) {
        assert!(Instant::now() < deadline, "waited too long for {what}");
        a.update(0.016);
        std::thread::sleep(Duration::from_millis(5));
    }
}

fn rows(a: &App) -> Vec<(String, Option<String>)> {
    a.overlay_page()
        .expect("the Game Art page is not open")
        .rows()
        .iter()
        .map(|r| (r.label.clone(), r.value.clone()))
        .collect()
}

fn card(games: &[(&str, &str, &[u8])]) -> tempfile::TempDir {
    let d = tempfile::tempdir().unwrap();
    slot::root::ensure(d.path());
    for (dir, file, bytes) in games {
        let at = d.path().join("Games").join(dir);
        std::fs::create_dir_all(&at).unwrap();
        std::fs::write(at.join(file), bytes).unwrap();
    }
    d
}

fn label(root: &Path, dir: &str, stem: &str) -> std::path::PathBuf {
    root.join("Labels").join(dir).join(format!("{stem}.png"))
}

#[test]
fn the_page_counts_each_systems_labels_and_finds_the_missing_ones() {
    let d = card(&[
        ("GBA", "whatever.gba", &[0; 8]),
        ("GBA", "unknown.gba", &[1; 8]),
        ("GB", "mystery.gb", &[2; 8]),
    ]);
    let mut a = boot(d.path());
    game_art(&mut a);
    assert_eq!(a.named_value(QuickRow::GameArt).as_deref(), Some("0 of 3"));
    press(&mut a, Btn::A);
    let shown = rows(&a);
    assert_eq!(shown[0].0, "Art from libretro-thumbnails");
    assert_eq!(
        shown[1..],
        [
            ("All systems".into(), Some("0 of 3".into())),
            ("Game Boy Advance".into(), Some("0 of 2".into())),
            ("Game Boy".into(), Some("0 of 1".into())),
            ("ScreenScraper".into(), Some("Sign in".into())),
        ]
    );
    assert_eq!(
        a.overlay_page().unwrap().cursor(),
        1,
        "All systems is not lit"
    );

    press(&mut a, Btn::Down);
    press(&mut a, Btn::A);
    until(&mut a, "the GBA games", |a| {
        let v = a.scrape_view();
        !v.running && v.done == 2
    });
    assert_eq!(a.scrape_view().missed, 1);
    let made = label(d.path(), "GBA", "whatever");
    assert!(std::fs::read(&made).unwrap().starts_with(b"\x89PNG"));
    assert!(
        !label(d.path(), "GB", "mystery").exists(),
        "the Game Boy was looked for too"
    );
    until(&mut a, "the counts", |a| {
        rows(a).contains(&("Game Boy Advance".into(), Some("1 of 2".into())))
    });
    assert_eq!(rows(&a)[0].0, "Added 1 label to the shelves");
    assert_eq!(a.named_value(QuickRow::GameArt).as_deref(), Some("1 of 3"));

    press(&mut a, Btn::B);
    assert!(a.overlay_page().is_none());
    assert_eq!(a.quick_menu(), Some(QuickRow::GameArt));
}

#[test]
fn a_system_with_every_label_says_so() {
    let d = card(&[
        ("GBA", "whatever.gba", &[0; 8]),
        ("GBA", "other.gba", &[1; 8]),
    ]);
    std::fs::create_dir_all(d.path().join("Labels/GBA")).unwrap();
    for stem in ["whatever", "other"] {
        std::fs::write(label(d.path(), "GBA", stem), b"").unwrap();
    }
    let mut a = boot(d.path());
    game_art(&mut a);
    press(&mut a, Btn::A);
    press(&mut a, Btn::A);
    assert_eq!(rows(&a)[0].0, "Every game here has a label");
    assert!(!a.scrape_view().running);
    press(&mut a, Btn::Down);
    assert_ne!(
        rows(&a)[0].0,
        "Every game here has a label",
        "the note stayed"
    );
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
fn screenscraper_signs_in_on_the_keyboard_and_out_with_no_name() {
    let d = card(&[
        ("GBA", "whatever.gba", &[0; 8]),
        ("GBA", "other.gba", &[1; 8]),
    ]);
    let mut a = boot(d.path());
    game_art(&mut a);
    press(&mut a, Btn::A);
    let last = rows(&a).len() - 1;
    for _ in 1..last {
        press(&mut a, Btn::Down);
    }
    press(&mut a, Btn::A);
    assert!(
        a.network_keyboard_key().is_some(),
        "no keyboard for the username"
    );
    type_in(&mut a, "ME");
    press(&mut a, Btn::Start);
    type_in(&mut a, "pw");
    press(&mut a, Btn::Start);
    assert!(a.network_keyboard_key().is_none(), "the keyboard stayed up");
    let conf = std::fs::read_to_string(d.path().join("Config/screenscraper.conf")).unwrap();
    assert!(
        conf.contains("ssid=ME") && conf.contains("sspassword=pw"),
        "{conf}"
    );
    until(&mut a, "the row", |a| {
        rows(a).last() == Some(&("ScreenScraper".into(), Some("ME".into())))
    });

    press(&mut a, Btn::A);
    for _ in 0..2 {
        press(&mut a, Btn::B);
    }
    press(&mut a, Btn::Start);
    let conf = std::fs::read_to_string(d.path().join("Config/screenscraper.conf")).unwrap();
    assert!(!conf.contains("ssid"), "signing out kept the login: {conf}");
}
