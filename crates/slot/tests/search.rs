// Ported from nickclyde/slot-plus@02c5890, crates/slot/tests/search.rs. X, L1 and R1 reach the
// shelf as the bindings make them, through `common::through`.

mod common;

use slot::app::{App, Phase};
use slot_input::{Action, Btn};
use slot_ui::Key;
use tempfile::TempDir;

const GBA: &str = "Game Boy Advance";

fn card() -> TempDir {
    let d = common::tmp_root_with_carts(&[
        "Metroid Fusion",
        "Pokemon - Emerald Version (USA, Europe)",
        "Pokemon - Ruby Version",
        "Golden Sun",
    ]);
    common::write_gb_cart(&d, "Tetris", "TETRIS");
    d
}

fn press(a: &mut App, btn: Btn) {
    a.apply(Action::GbaDown(btn));
    a.apply(Action::GbaUp(btn));
}

/// A shelf key, as the bindings turn it into an action.
fn shelf_key(a: &mut App, btn: Btn) {
    common::through(a, btn);
}

fn hit(a: &mut App, want: Key) {
    for _ in 0..5 {
        for _ in 0..10 {
            if a.search().expect("the keyboard is not open").keyboard.key() == want {
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
        hit(a, if c == ' ' { Key::Space } else { Key::Char(c) });
    }
}

fn found(a: &App) -> Vec<String> {
    a.search()
        .expect("the keyboard is not open")
        .stems()
        .into_iter()
        .map(String::from)
        .collect()
}

#[test]
fn x_opens_the_keyboard_and_typing_narrows_the_matches() {
    let d = card();
    let mut a = common::boot(d.path());
    shelf_key(&mut a, Btn::X);
    assert!(a.search().is_some());
    assert!(found(&a).is_empty(), "an empty search matched everything");
    type_in(&mut a, "POK");
    assert_eq!(a.search().unwrap().query(), "POK");
    assert_eq!(
        found(&a),
        [
            "Pokemon - Emerald Version (USA, Europe)",
            "Pokemon - Ruby Version"
        ]
    );
    assert_eq!(a.search().unwrap().count_text(), "2 games");
    type_in(&mut a, " RU");
    assert_eq!(found(&a), ["Pokemon - Ruby Version"]);
    assert_eq!(a.search().unwrap().shown(), ["Pokemon Ruby Version"]);
}

#[test]
fn b_deletes_a_letter_and_closes_the_keyboard_once_it_is_empty() {
    let d = card();
    let mut a = common::boot(d.path());
    shelf_key(&mut a, Btn::X);
    type_in(&mut a, "GO");
    press(&mut a, Btn::B);
    assert_eq!(a.search().unwrap().query(), "G");
    press(&mut a, Btn::B);
    assert_eq!(a.search().unwrap().query(), "");
    press(&mut a, Btn::B);
    assert!(a.search().is_none());
    assert_eq!(a.shelf_platform_name(), Some(GBA));
}

#[test]
fn start_puts_the_matches_on_a_search_shelf_and_moves_there() {
    let d = card();
    let mut a = common::boot(d.path());
    shelf_key(&mut a, Btn::X);
    type_in(&mut a, "VERSION");
    press(&mut a, Btn::Start);
    assert!(a.search().is_none());
    assert_eq!(a.shelf_platform_name(), Some("Search"));
    assert_eq!(a.slot_text().as_deref(), Some("\"VERSION\""));
    assert_eq!(
        a.shelf_stems(),
        [
            "Pokemon - Emerald Version (USA, Europe)",
            "Pokemon - Ruby Version"
        ]
    );
    assert_eq!(
        a.selected_stem(),
        Some("Pokemon - Emerald Version (USA, Europe)")
    );
    a.apply(Action::Insert);
    assert!(
        matches!(a.phase(), Phase::Inserting { .. }),
        "a search result would not play"
    );
}

#[test]
fn the_ok_key_searches_like_start() {
    let d = card();
    let mut a = common::boot(d.path());
    shelf_key(&mut a, Btn::X);
    type_in(&mut a, "SUN");
    hit(&mut a, Key::Done);
    assert_eq!(a.shelf_stems(), ["Golden Sun"]);
}

#[test]
fn a_search_with_no_matches_shakes_and_stays_open() {
    let d = card();
    let mut a = common::boot(d.path());
    shelf_key(&mut a, Btn::X);
    type_in(&mut a, "ZZ");
    assert_eq!(a.search().unwrap().count_text(), "No games");
    press(&mut a, Btn::Start);
    assert!(a.search().is_some(), "an empty search closed the keyboard");
    assert!(a.refusal_active(a.now()));
    assert_eq!(a.shelf_platform_name(), Some(GBA));
}

#[test]
fn the_keyboard_keeps_menu_start_and_the_shoulders_to_itself() {
    let d = card();
    let mut a = common::boot(d.path());
    shelf_key(&mut a, Btn::X);
    type_in(&mut a, "AB");
    press(&mut a, Btn::L1);
    assert_eq!(a.search().unwrap().query(), "A", "L1 did not delete");
    press(&mut a, Btn::R1);
    assert_eq!(a.search().unwrap().query(), "A ", "R1 did not space");
    assert_eq!(a.shelf_platform_name(), Some(GBA));
    a.apply(Action::QuickMenu);
    assert!(
        matches!(a.phase(), Phase::Shelf),
        "MENU opened the settings over the keyboard"
    );
    assert!(a.search().is_none(), "MENU did not close the keyboard");
}

#[test]
fn b_on_the_search_shelf_clears_it_and_goes_back() {
    let d = card();
    let mut a = common::boot(d.path());
    press(&mut a, Btn::Right);
    shelf_key(&mut a, Btn::X);
    type_in(&mut a, "POK");
    press(&mut a, Btn::Start);
    press(&mut a, Btn::B);
    assert_eq!(a.shelf_platform_name(), Some(GBA));
    assert_eq!(a.selected_stem(), Some("Metroid Fusion"));
    assert!(
        !a.shelf_names().iter().any(|n| n.starts_with('"')),
        "the cleared search stayed on the ring"
    );
}

#[test]
fn x_on_the_search_shelf_edits_the_last_search() {
    let d = card();
    let mut a = common::boot(d.path());
    shelf_key(&mut a, Btn::X);
    type_in(&mut a, "POK");
    press(&mut a, Btn::Start);
    shelf_key(&mut a, Btn::X);
    assert_eq!(a.search().unwrap().query(), "POK");
    type_in(&mut a, "EMON R");
    press(&mut a, Btn::Start);
    assert_eq!(a.shelf_stems(), ["Pokemon - Ruby Version"]);
    press(&mut a, Btn::B);
    assert_eq!(
        a.shelf_platform_name(),
        Some(GBA),
        "a second search forgot where the first one started"
    );
}

#[test]
fn the_search_shelf_stays_on_the_ring_until_it_is_cleared() {
    let d = card();
    let mut a = common::boot(d.path());
    shelf_key(&mut a, Btn::X);
    type_in(&mut a, "POK");
    press(&mut a, Btn::Start);
    shelf_key(&mut a, Btn::R1);
    assert_eq!(a.shelf_platform_name(), Some(GBA));
    shelf_key(&mut a, Btn::L1);
    assert_eq!(a.slot_text().as_deref(), Some("\"POK\""));
}
