mod common;

use common::{app_playing_in, boot, tmp_root_with_carts};
use slot::app::App;
use slot_input::{Action, Btn};
use slot_store::{read_shortcuts, write_shortcuts, Binding, Device, Modifier, Shortcut};
use slot_ui::QuickRow;

fn press(a: &mut App, btn: Btn) {
    a.apply(Action::GbaDown(btn));
    a.apply(Action::GbaUp(btn));
}

fn value(a: &App, label: &str) -> Option<String> {
    a.overlay_page()?
        .rows()
        .iter()
        .find(|r| r.label == label)
        .and_then(|r| r.value.clone())
}

fn open_from_controls(a: &mut App) {
    a.apply(Action::QuickMenu);
    for row in [QuickRow::Controls, QuickRow::ControlsGuide] {
        for _ in 0..row.position(&a.hidden_rows()) {
            press(a, Btn::Down);
        }
        press(a, Btn::A);
    }
}

#[test]
fn the_guide_opens_from_controls_and_b_goes_back_to_its_row() {
    let d = tmp_root_with_carts(&["Emerald", "Fusion"]);
    let mut a = boot(d.path());
    open_from_controls(&mut a);
    assert!(
        a.overlay_page().is_some(),
        "A on Button Guide opened nothing"
    );
    let save = QuickRow::ScSave.label();
    assert_eq!(value(&a, save).as_deref(), Some("SEL+R1"));
    assert_eq!(value(&a, "Eject and save").as_deref(), Some("Hold MENU"));
    press(&mut a, Btn::Down);
    assert_eq!(a.overlay_page().unwrap().cursor(), 1);
    press(&mut a, Btn::B);
    assert!(a.overlay_page().is_none());
    assert_eq!(a.quick_menu(), Some(QuickRow::ControlsGuide));
}

#[test]
fn a_remapped_shortcut_shows_its_new_buttons() {
    let d = tmp_root_with_carts(&["Emerald", "Fusion"]);
    let mut keys = read_shortcuts(d.path());
    keys.set(
        Shortcut::SaveState,
        Some(Binding {
            modifier: Some(Modifier::Button(Device::Start)),
            button: Device::X,
        }),
    );
    keys.set(
        Shortcut::FastForward,
        Some(Binding {
            modifier: None,
            button: Device::L1,
        }),
    );
    keys.set(Shortcut::LoadState, None);
    write_shortcuts(d.path(), &keys).unwrap();
    let mut a = boot(d.path());
    open_from_controls(&mut a);
    assert_eq!(
        value(&a, QuickRow::ScSave.label()).as_deref(),
        Some("START+X")
    );
    assert_eq!(
        value(&a, QuickRow::ScLoad.label()).as_deref(),
        Some("Not set")
    );
    assert_eq!(
        value(&a, "Lock fast forward").as_deref(),
        Some("Double tap L1")
    );
}

#[test]
fn turbo_rows_show_only_with_turbo_on() {
    for turbo in [false, true] {
        let d = tmp_root_with_carts(&["Emerald", "Fusion"]);
        let mut state = slot_store::read_slot_state(d.path());
        state.turbo = turbo;
        slot_store::write_slot_state(d.path(), &state).unwrap();
        let mut a = boot(d.path());
        open_from_controls(&mut a);
        assert_eq!(
            value(&a, QuickRow::GameTurboA.label()).is_some(),
            turbo,
            "Turbo {turbo}"
        );
    }
}

#[test]
fn in_game_the_guide_opens_from_settings_and_menu_closes_it() {
    let d = tmp_root_with_carts(&["Emerald", "Fusion"]);
    let mut a = app_playing_in(d.path(), "Emerald");
    a.apply(Action::QuickMenu);
    for row in [QuickRow::Settings, QuickRow::PlayGuide] {
        for _ in 0..12 {
            if a.in_game_menu() == Some(row) {
                break;
            }
            press(&mut a, Btn::Down);
        }
        assert_eq!(a.in_game_menu(), Some(row));
        press(&mut a, Btn::A);
    }
    let page = a.overlay_page().expect("the in-game guide did not open");
    assert_eq!(
        page.selected().map(|r| r.label.as_str()),
        Some("\u{2014} In Game \u{2014}"),
        "in game the guide did not open on its In Game part"
    );
    a.apply(Action::QuickMenu);
    assert!(a.overlay_page().is_none());
    assert_eq!(
        a.in_game_menu(),
        Some(QuickRow::PlayGuide),
        "MENU went past the guide"
    );
}
