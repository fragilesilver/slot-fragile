mod common;

use slot::app::App;
use slot_input::{Action, Btn};
use slot_store::collections::{list_path, LISTS_DIR};
use slot_store::{write_carousel, Binding, CarouselAction, CarouselKeys, Device};
use slot_ui::{Key, QuickRow};
use tempfile::TempDir;

fn card() -> TempDir {
    let d = common::tmp_root_with_carts(&["Emerald", "Fusion", "Golden Sun"]);
    common::write_gb_cart(&d, "Tetris", "TETRIS");
    d
}

fn write_list(d: &TempDir, name: &str, text: &str) {
    let dir = d.path().join(LISTS_DIR);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join(format!("{name}.txt")), text).unwrap();
}

fn read_list(d: &TempDir, name: &str) -> String {
    std::fs::read_to_string(list_path(d.path(), name)).unwrap_or_default()
}

fn press(a: &mut App, btn: Btn) {
    a.apply(Action::GbaDown(btn));
    a.apply(Action::GbaUp(btn));
}

fn rows(a: &App) -> Vec<(String, Option<String>)> {
    a.overlay_page()
        .expect("Add to List is not open")
        .rows()
        .iter()
        .map(|r| (r.label.clone(), r.value.clone()))
        .collect()
}

fn row(label: &str, value: Option<&str>) -> (String, Option<String>) {
    (label.to_string(), value.map(String::from))
}

fn go_to_row(a: &mut App, label: &str) {
    for _ in 0..rows(a).len() {
        let page = a.overlay_page().unwrap();
        if page.selected().map(|r| r.label.as_str()) == Some(label) {
            return;
        }
        press(a, Btn::Down);
    }
    panic!("no {label} row");
}

fn on_shelf(a: &mut App, name: &str) {
    for _ in 0..a.shelf_names().len() + 1 {
        if a.shelf_platform_name() == Some(name) {
            return;
        }
        common::through(a, Btn::R1);
    }
    panic!("no {name} shelf in {:?}", a.shelf_names());
}

#[test]
fn each_list_is_a_shelf_of_its_games_in_written_order_after_search() {
    let d = card();
    write_list(&d, "Couch", "GB/Tetris\nFusion\nZelda\n");
    write_list(&d, "Empty", "Zelda\n");
    let mut a = common::boot(d.path());
    let names = a.shelf_names();
    assert!(names.contains(&"Couch".to_string()), "{names:?}");
    assert!(
        !names.contains(&"Empty".to_string()),
        "a list with none of the card's games is on the ring"
    );
    on_shelf(&mut a, "Couch");
    assert_eq!(a.shelf_stems(), ["Tetris", "Fusion"]);
}

#[test]
fn add_to_list_puts_the_cart_on_a_list_and_takes_it_off_again() {
    let d = card();
    write_list(&d, "Couch", "Fusion\n");
    let mut a = common::boot(d.path());
    on_shelf(&mut a, "Game Boy Advance");
    let stem = a.selected_stem().unwrap().to_string();
    assert_eq!(stem, "Emerald");
    a.apply(Action::AddToList);
    assert_eq!(rows(&a), [row("Couch", Some("No")), row("New List", None)]);
    press(&mut a, Btn::A);
    assert_eq!(rows(&a)[0], row("Couch", Some("Yes")));
    assert_eq!(read_list(&d, "Couch"), "Fusion\nGBA/Emerald\n");
    press(&mut a, Btn::B);
    assert!(a.overlay_page().is_none(), "B did not close Add to List");
    on_shelf(&mut a, "Couch");
    assert_eq!(a.shelf_stems(), ["Fusion", "Emerald"]);
    a.apply(Action::AddToList);
    assert_eq!(rows(&a)[0], row("Couch", Some("Yes")));
    assert_eq!(a.selected_stem(), Some("Fusion"));
    press(&mut a, Btn::A);
    assert_eq!(
        read_list(&d, "Couch"),
        "GBA/Emerald\n",
        "taking Fusion off kept its line or lost Emerald's"
    );
    a.apply(Action::QuickMenu);
    assert!(a.overlay_page().is_none(), "MENU did not close Add to List");
    assert_eq!(a.quick_menu(), None, "MENU went on into the settings");
    assert_eq!(a.shelf_stems(), ["Emerald"]);
}

fn hit(a: &mut App, want: Key) {
    for _ in 0..5 {
        for _ in 0..10 {
            if a.list_keyboard_key() == Some(want) {
                return press(a, Btn::A);
            }
            press(a, Btn::Right);
        }
        press(a, Btn::Down);
    }
    panic!("no {want:?} on the keyboard");
}

#[test]
fn a_new_list_is_named_on_the_keyboard_and_gets_a_shelf_with_the_cart() {
    let d = card();
    write_list(&d, "Couch", "Fusion\n");
    let mut a = common::boot(d.path());
    on_shelf(&mut a, "Game Boy");
    a.apply(Action::AddToList);
    go_to_row(&mut a, "New List");
    press(&mut a, Btn::A);
    assert_eq!(a.list_naming(), Some(""));
    for c in "ARCADE".chars() {
        hit(&mut a, Key::Char(c));
    }
    press(&mut a, Btn::Start);
    assert_eq!(a.list_naming(), None);
    assert_eq!(read_list(&d, "ARCADE"), "GB/Tetris\n");
    assert_eq!(
        rows(&a),
        [
            row("ARCADE", Some("Yes")),
            row("Couch", Some("No")),
            row("New List", None)
        ],
        "the new list is not in name order, or not lit"
    );
    assert_eq!(
        a.overlay_page()
            .unwrap()
            .selected()
            .map(|r| r.label.as_str()),
        Some("ARCADE")
    );
    press(&mut a, Btn::B);
    assert_eq!(a.shelf_platform_name(), Some("Game Boy"));
    on_shelf(&mut a, "ARCADE");
    assert_eq!(a.shelf_stems(), ["Tetris"]);
}

#[test]
fn a_blank_name_shakes_and_b_on_an_empty_name_goes_back_to_the_lists() {
    let d = card();
    let mut a = common::boot(d.path());
    a.apply(Action::AddToList);
    assert_eq!(rows(&a), [row("New List", None)]);
    press(&mut a, Btn::A);
    press(&mut a, Btn::Y);
    press(&mut a, Btn::Start);
    assert_eq!(a.list_naming(), Some(" "), "a blank name made a list");
    assert!(a.refusal_active(a.now()));
    press(&mut a, Btn::B);
    press(&mut a, Btn::B);
    assert_eq!(a.list_naming(), None);
    assert_eq!(rows(&a), [row("New List", None)]);
    assert!(std::fs::read_dir(d.path().join(LISTS_DIR))
        .unwrap()
        .next()
        .is_none());
}

#[test]
fn the_shelf_keys_have_no_clash_as_they_come() {
    let d = card();
    let a = common::boot(d.path());
    for row in QuickRow::SHELF_KEYS {
        assert_eq!(a.clash(row), None, "{row:?}");
    }
    assert_eq!(
        CarouselKeys::default().get(CarouselAction::Search),
        Binding::plain(Device::X)
    );
}

#[test]
fn the_press_that_opens_a_page_is_not_also_the_pages() {
    let d = card();
    let mut keys = CarouselKeys::default();
    keys.set(CarouselAction::AddToList, Binding::plain(Device::B));
    write_carousel(d.path(), &keys).unwrap();
    let mut a = common::boot(d.path());
    common::through(&mut a, Btn::B);
    assert!(
        a.overlay_page().is_some(),
        "the B that opened the page closed it"
    );
    common::through(&mut a, Btn::B);
    assert!(a.overlay_page().is_none());
    common::through(&mut a, Btn::X);
    assert_eq!(
        a.search().map(|s| s.keyboard.layer()),
        Some(slot_ui::Layer::Upper),
        "the X that opened the search also pressed Shift"
    );
}

#[test]
fn while_add_to_list_is_up_the_shoulders_do_not_change_shelves() {
    let d = card();
    write_list(&d, "Couch", "Fusion\n");
    let mut a = common::boot(d.path());
    let shelf = a.shelf_platform_name();
    a.apply(Action::AddToList);
    common::through(&mut a, Btn::R1);
    common::through(&mut a, Btn::Y);
    assert_eq!(a.shelf_platform_name(), shelf);
    assert!(a.overlay_page().is_some());
    assert!(
        !slot_store::read_slot_state(d.path()).is_favorite(slot_store::Platform::Gba, "Emerald"),
        "Y favorited the cart under the page"
    );
}
