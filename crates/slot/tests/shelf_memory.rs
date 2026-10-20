// Ported from marghost/slot_modded@1660894, crates/slot/tests/shelf_memory.rs, with the marks
// and the start shelf moved to Config/extras.state.

mod common;

use common::{boot, tmp_root_with_carts, write_gb_cart};
use slot::app::App;
use slot_input::Btn;
use slot_store::{read_extras, read_slot_state, write_extras, write_slot_state, Platform, Recent};

fn press(a: &mut App, btn: Btn) {
    common::through(a, btn);
}

fn selected(a: &App) -> String {
    a.selected_stem().expect("a cart is selected").to_string()
}

fn extras_text(d: &tempfile::TempDir) -> String {
    std::fs::read_to_string(d.path().join("Config/extras.state")).unwrap_or_default()
}

fn card() -> tempfile::TempDir {
    let d = tmp_root_with_carts(&["Alpha", "Beta", "Gamma"]);
    write_gb_cart(&d, "Kirby", "KIRBY");
    write_gb_cart(&d, "Tetris", "TETRIS");
    d
}

fn with_extras(d: &tempfile::TempDir, pairs: &[(&str, &str)]) {
    let mut e = read_extras(d.path());
    for (k, v) in pairs {
        e.set(k, Some(v));
    }
    write_extras(d.path(), &e).unwrap();
}

#[test]
fn each_shelf_opens_where_it_was_left_after_a_restart() {
    let d = card();
    {
        let mut a = boot(d.path());
        assert_eq!(a.shelf_platform_name(), Some("Game Boy Advance"));
        press(&mut a, Btn::Right);
        assert_eq!(selected(&a), "Beta");
        press(&mut a, Btn::R1);
        assert_eq!(a.shelf_platform_name(), Some("Game Boy"));
        press(&mut a, Btn::Right);
        assert_eq!(selected(&a), "Tetris");
        press(&mut a, Btn::L1);
    }
    let mut a = boot(d.path());
    assert_eq!(selected(&a), "Beta");
    press(&mut a, Btn::R1);
    assert_eq!(
        selected(&a),
        "Tetris",
        "the Game Boy shelf forgot its place"
    );
}

#[test]
fn browsing_alone_writes_nothing_to_the_card() {
    let d = card();
    let mut a = boot(d.path());
    let before = extras_text(&d);
    for _ in 0..5 {
        press(&mut a, Btn::Right);
    }
    assert_eq!(
        extras_text(&d),
        before,
        "a step along the shelf wrote the card"
    );
    press(&mut a, Btn::R1);
    assert_ne!(
        extras_text(&d),
        before,
        "leaving the shelf did not remember it"
    );
}

#[test]
fn a_remembered_game_that_is_gone_opens_the_first_game() {
    let d = card();
    with_extras(&d, &[("shelf.gba", "gba/Missing")]);
    let a = boot(d.path());
    assert_eq!(selected(&a), "Alpha");
}

#[test]
fn the_start_shelf_wins_over_the_last_one() {
    let d = card();
    let mut s = read_slot_state(d.path());
    s.shelf = Some("gba".into());
    s.shelf_cart = Some(Recent {
        platform: Platform::Gba,
        stem: "Gamma".into(),
    });
    write_slot_state(d.path(), &s).unwrap();
    assert_eq!(selected(&boot(d.path())), "Gamma");

    with_extras(&d, &[("start_shelf", "gb"), ("shelf.gb", "gb/Tetris")]);
    let a = boot(d.path());
    assert_eq!(a.shelf_platform_name(), Some("Game Boy"));
    assert_eq!(selected(&a), "Tetris");
}

#[test]
fn a_start_shelf_with_nothing_on_it_leaves_the_last_shelf() {
    let d = card();
    with_extras(&d, &[("start_shelf", "favorites")]);
    let a = boot(d.path());
    assert_eq!(a.shelf_platform_name(), Some("Game Boy Advance"));
}

#[test]
fn a_start_shelf_slot_does_not_know_reads_as_last() {
    let d = card();
    with_extras(&d, &[("start_shelf", "nes")]);
    let a = boot(d.path());
    assert_eq!(a.start_shelf(), slot::app::StartShelf::Last);
}

#[test]
fn hiding_favorites_takes_the_shelf_away_but_keeps_the_stars() {
    let d = card();
    let mut s = read_slot_state(d.path());
    s.favorites = vec![Recent {
        platform: Platform::Gb,
        stem: "Kirby".into(),
    }];
    write_slot_state(d.path(), &s).unwrap();
    with_extras(&d, &[("show_favorites", "0"), ("start_shelf", "favorites")]);
    let mut a = boot(d.path());
    let mut seen = Vec::new();
    for _ in 0..4 {
        seen.extend(a.shelf_platform_name());
        press(&mut a, Btn::R1);
    }
    assert!(!seen.contains(&"Favorites"), "Favorites showed: {seen:?}");
    assert_eq!(
        read_slot_state(d.path()).favorites.len(),
        1,
        "the star was lost"
    );
}
