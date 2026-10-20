mod common;

use common::{boot, tmp_root_with_carts};
use slot::app::{App, Phase};
use slot_input::{Action, Btn};
use slot_ui::QuickRow;

fn press(a: &mut App, btn: Btn) {
    common::through(a, btn);
}

fn on_about() -> (tempfile::TempDir, App) {
    let d = tmp_root_with_carts(&["Emerald", "Fusion", "Ruby"]);
    let mut a = boot(d.path());
    a.apply(Action::QuickMenu);
    for _ in 0..QuickRow::About.position(&a.hidden_rows()) {
        press(&mut a, Btn::Down);
    }
    press(&mut a, Btn::A);
    assert!(matches!(a.phase(), Phase::About), "{:?}", a.phase());
    (d, a)
}

fn value(a: &App, label: &str) -> Option<String> {
    a.overlay_page()?
        .rows()
        .iter()
        .find(|r| r.label == label)
        .and_then(|r| r.value.clone())
}

#[test]
fn a_on_the_label_opens_details_and_b_goes_back_to_the_label() {
    let (_d, mut a) = on_about();
    assert!(a.overlay_page().is_none());
    press(&mut a, Btn::A);
    assert_eq!(
        value(&a, "Version"),
        Some(format!("Fragile (slot {})", slot::build_info::SLOT_RELEASE))
    );
    assert_eq!(value(&a, "Games").as_deref(), Some("3"));
    assert!(value(&a, "Build").is_some_and(|b| !b.is_empty()));
    press(&mut a, Btn::B);
    assert!(a.overlay_page().is_none(), "B left Details open");
    assert!(matches!(a.phase(), Phase::About), "B went past the label");
}

#[test]
fn the_card_the_games_are_on_reports_its_free_space() {
    let (_d, mut a) = on_about();
    press(&mut a, Btn::A);
    let page = a.overlay_page().expect("no details");
    let card = page
        .rows()
        .iter()
        .find(|r| r.label.ends_with("Card"))
        .expect("no card row");
    assert!(
        card.value
            .as_deref()
            .is_some_and(|v| v.ends_with("GB free")),
        "{card:?}"
    );
}

#[test]
fn up_and_down_walk_the_rows_and_go_round() {
    let (_d, mut a) = on_about();
    press(&mut a, Btn::A);
    let n = a.overlay_page().unwrap().rows().len();
    press(&mut a, Btn::Up);
    assert_eq!(a.overlay_page().unwrap().cursor(), n - 1);
    press(&mut a, Btn::Down);
    assert_eq!(a.overlay_page().unwrap().cursor(), 0);
}

#[test]
fn the_menu_button_closes_details_onto_the_menu() {
    let (_d, mut a) = on_about();
    press(&mut a, Btn::A);
    a.apply(Action::QuickMenu);
    assert!(a.overlay_page().is_none());
    assert_eq!(a.quick_menu(), Some(QuickRow::About));
}

#[test]
fn x_on_details_does_not_restart() {
    let (_d, mut a) = on_about();
    press(&mut a, Btn::A);
    press(&mut a, Btn::X);
    assert!(a.overlay_page().is_some(), "X closed Details");
    assert!(matches!(a.phase(), Phase::About));
}

#[test]
fn the_slot_release_is_the_newest_in_the_changelog() {
    let log = std::fs::read_to_string(common::repo_root().join("CHANGELOG.md")).unwrap();
    let newest = log
        .lines()
        .find_map(|l| l.strip_prefix("## ["))
        .and_then(|l| l.split(']').next())
        .expect("no release heading in CHANGELOG.md");
    assert_eq!(slot::build_info::SLOT_RELEASE, newest);
}
