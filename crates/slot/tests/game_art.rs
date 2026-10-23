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

/// The pretend servers' box is all this red; their logo is white.
const BOX_RED: [u8; 3] = [200, 40, 40];

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
    root.join(slot_store::SCRAPED_LABELS)
        .join(dir)
        .join(format!("{stem}.png"))
}

fn own_label(root: &Path, dir: &str, stem: &str) -> std::path::PathBuf {
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
            ("Label Style".into(), Some("Cart Label".into())),
            ("Automatic".into(), Some("On".into())),
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
        std::fs::write(own_label(d.path(), "GBA", stem), b"").unwrap();
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

fn pixel(png: &Path) -> [u8; 3] {
    let decoder = png::Decoder::new(std::fs::File::open(png).unwrap());
    let mut reader = decoder.read_info().unwrap();
    let mut buf = vec![0; reader.output_buffer_size()];
    let info = reader.next_frame(&mut buf).unwrap();
    let at = ((info.height / 2 * info.width + 2) * 4) as usize;
    [buf[at], buf[at + 1], buf[at + 2]]
}

fn value(a: &App, label: &str) -> Option<String> {
    rows(a).into_iter().find(|r| r.0 == label).and_then(|r| r.1)
}

fn to_row(a: &mut App, label: &str) {
    for _ in 0..12 {
        if a.overlay_page()
            .and_then(|p| p.selected())
            .map(|r| r.label.as_str())
            == Some(label)
        {
            return;
        }
        press(a, Btn::Down);
    }
    panic!("no {label} in {:?}", rows(a));
}

fn idle(a: &mut App, what: &str) {
    until(a, what, |a| {
        !a.scrape_view().running && a.scrape_view().total > 0
    });
}

/// Fragile: a label is the cart's own by default, the logo with no scan of it; Label Style draws
/// every one Game Art made again as the box from the art kept on the card, and the player's own
/// label is never touched.
#[test]
fn label_style_draws_game_arts_labels_again_and_leaves_the_players_own() {
    let d = card(&[
        ("GBA", "whatever.gba", &[0; 8]),
        ("GBA", "unknown.gba", &[1; 8]),
    ]);
    let mut a = boot(d.path());
    game_art(&mut a);
    press(&mut a, Btn::A);
    press(&mut a, Btn::A);
    idle(&mut a, "the first look");
    let made = label(d.path(), "GBA", "whatever");
    assert_ne!(
        pixel(&made),
        BOX_RED,
        "with no label scan the label is the box, not the logo"
    );
    std::fs::create_dir_all(d.path().join("Labels/GBA")).unwrap();
    std::fs::write(own_label(d.path(), "GBA", "unknown"), b"mine").unwrap();

    to_row(&mut a, "Label Style");
    let looked = a.scrape_view().made.len();
    assert_eq!(value(&a, "Label Style").as_deref(), Some("Cart Label"));
    press(&mut a, Btn::Right);
    assert_eq!(value(&a, "Label Style").as_deref(), Some("Box Art"));
    until(&mut a, "the redraw", |a| {
        a.scrape_view().made.len() > looked
    });
    idle(&mut a, "the redraw");
    assert_eq!(
        pixel(&made),
        BOX_RED,
        "the label was not drawn again as the box"
    );
    assert_eq!(
        std::fs::read(own_label(d.path(), "GBA", "unknown")).unwrap(),
        b"mine"
    );
    assert_eq!(
        slot_store::read_extras(d.path()).get("label_style"),
        Some("art")
    );
}

/// Fragile: Automatic finds what is missing on its own, once a game, while the internet is up.
#[test]
fn automatic_finds_missing_art_once_a_game() {
    let d = card(&[
        ("GBA", "whatever.gba", &[0; 8]),
        ("GBA", "unknown.gba", &[1; 8]),
    ]);
    let mut a = boot(d.path());
    let scraper = slot::scrape::Scraper::spawn(Box::new(slot::scrape::SimFetch), d.path());
    a.use_scraper(scraper, true);
    until(&mut a, "Automatic", |a| {
        !a.scrape_view().running && a.scrape_view().done == 2
    });
    assert!(label(d.path(), "GBA", "whatever").exists());
    for _ in 0..20 {
        a.update(0.016);
        std::thread::sleep(Duration::from_millis(5));
    }
    assert_eq!(
        a.scrape_view().total,
        2,
        "the game with no art was asked for again"
    );
}

/// Fragile: after boot a label drawn in another style than its game now takes is drawn again
/// from the kept art, and one already in its style is left as it is.
#[test]
fn a_label_drawn_in_an_old_style_is_drawn_again_after_boot() {
    let d = card(&[
        ("GBA", "whatever.gba", &[0; 8]),
        ("GBA", "other.gba", &[1; 8]),
    ]);
    let art = d.path().join(slot::scrape::ART).join("GBA");
    std::fs::create_dir_all(&art).unwrap();
    std::fs::create_dir_all(d.path().join(slot_store::SCRAPED_LABELS).join("GBA")).unwrap();
    for (stem, style) in [("whatever", "art"), ("other", "label")] {
        std::fs::write(art.join(format!("{stem}.looked")), b"").unwrap();
        std::fs::write(art.join(format!("{stem}.style")), style).unwrap();
        std::fs::write(label(d.path(), "GBA", stem), b"old").unwrap();
    }
    std::fs::write(
        art.join("whatever.logo.png"),
        slot::scrape::SimFetch::logo(),
    )
    .unwrap();
    let mut a = boot(d.path());
    until(&mut a, "the check", |a| {
        !a.scrape_view().running && a.scrape_view().done == 2
    });
    assert!(
        std::fs::read(label(d.path(), "GBA", "whatever"))
            .unwrap()
            .starts_with(b"\x89PNG"),
        "the old style's label was not drawn again"
    );
    assert_eq!(
        std::fs::read_to_string(art.join("whatever.style")).unwrap(),
        "label"
    );
    assert_eq!(
        std::fs::read(label(d.path(), "GBA", "other")).unwrap(),
        b"old"
    );
    assert_eq!(a.scrape_view().made.len(), 1);
}

/// Fragile: a game looked for before ScreenScraper could be asked is looked for again once it
/// can be, for the label and disc scans only it has.
#[test]
fn a_developer_id_added_later_looks_each_game_up_again() {
    let d = card(&[
        ("GBA", "whatever.gba", &[0; 8]),
        ("GBA", "other.gba", &[1; 8]),
    ]);
    let mut a = boot(d.path());
    game_art(&mut a);
    press(&mut a, Btn::A);
    press(&mut a, Btn::A);
    idle(&mut a, "the first look");
    let looked = d.path().join(slot::scrape::ART).join("GBA/whatever.looked");
    assert_eq!(std::fs::read_to_string(&looked).unwrap(), "");

    std::fs::write(
        d.path().join("Config/screenscraper.conf"),
        "devid=d\ndevpassword=p\n",
    )
    .unwrap();
    let mut a = boot(d.path());
    until(&mut a, "the second look", |_| {
        std::fs::read_to_string(&looked).unwrap() == "screenscraper"
    });
    assert!(
        label(d.path(), "GBA", "whatever").exists(),
        "the libretro art was lost"
    );
}
