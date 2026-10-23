// Per-game settings: the in-game Settings → This Game page, and how a game's own values sit
// beside the settings menu's.

mod common;

use common::{app_playing_in, tmp_root_with_carts};
use slot::app::App;
use slot_input::{Action, Btn};
use slot_store::{read_slot_state, GameProfiles, Platform, ProfileField, Shader};
use slot_ui::{QuickRow, QuickValue, Toast};
use tempfile::TempDir;

fn press(a: &mut App, btn: Btn) {
    a.apply(Action::GbaDown(btn));
    a.apply(Action::GbaUp(btn));
}

fn go_to(a: &mut App, row: QuickRow) {
    a.apply(Action::QuickMenu);
    let path: &[QuickRow] = match row.page() {
        p if p == QuickRow::THIS_GAME => &[QuickRow::Settings, QuickRow::PlayThisGame],
        p if p == QuickRow::PLAY_SCREEN => &[QuickRow::Settings, QuickRow::PlayScreen],
        _ => &[QuickRow::Settings, QuickRow::PlayGameplay],
    };
    for step in path.iter().copied().chain([row]) {
        for _ in 0..12 {
            if a.in_game_menu() == Some(step) {
                break;
            }
            press(a, Btn::Down);
        }
        assert_eq!(a.in_game_menu(), Some(step), "never reached {step:?}");
        if step != row {
            press(a, Btn::A);
        }
    }
}

fn card() -> TempDir {
    tmp_root_with_carts(&["Emerald", "Fusion"])
}

fn own(d: &TempDir, stem: &str) -> slot_store::GameProfile {
    GameProfiles::read(d.path()).get(Platform::Gba, stem)
}

fn with_own(d: &TempDir, stem: &str, fields: &[ProfileField]) {
    let mut p = GameProfiles::read(d.path());
    for f in fields {
        p.set(Platform::Gba, stem, *f);
    }
    p.write(d.path()).unwrap();
}

#[test]
fn this_game_changes_only_the_game_and_it_comes_back_with_it() {
    let d = card();
    let mut a = app_playing_in(d.path(), "Emerald");
    let global = a.ff_speed();
    go_to(&mut a, QuickRow::ThisFfSpeed);
    assert_eq!(
        a.quick_value(QuickRow::ThisFfSpeed),
        QuickValue::speed(global)
    );
    press(&mut a, Btn::Left);
    let mine = a.ff_speed();
    assert_ne!(mine, global, "the row did not move the speed");
    assert_eq!(own(&d, "Emerald").ff_speed, Some(mine));
    assert_eq!(
        read_slot_state(d.path()).ff_speed,
        global,
        "the default moved too"
    );

    let other = app_playing_in(d.path(), "Fusion");
    assert_eq!(other.ff_speed(), global, "Fusion took Emerald's speed");
    let back = app_playing_in(d.path(), "Emerald");
    assert_eq!(back.ff_speed(), mine, "Emerald forgot its speed");
}

#[test]
fn a_game_with_its_own_shader_has_the_screen_row_and_shortcut_change_it() {
    let d = card();
    with_own(&d, "Emerald", &[ProfileField::Shader(Shader::Grid)]);
    let mut a = app_playing_in(d.path(), "Emerald");
    let global = read_slot_state(d.path()).shader_gba;
    assert_eq!(a.screen_shader(), Shader::Grid);
    go_to(&mut a, QuickRow::Shader);
    assert_eq!(a.quick_value(QuickRow::Shader), Some(QuickValue::Grid));
    press(&mut a, Btn::Right);
    assert_eq!(own(&d, "Emerald").shader, Some(Shader::Dot));
    assert_eq!(
        read_slot_state(d.path()).shader_gba,
        global,
        "the default moved"
    );

    a.apply(Action::QuickMenu);
    assert_eq!(a.in_game_menu(), None);
    a.apply(Action::ShaderPrev);
    assert_eq!(own(&d, "Emerald").shader, Some(Shader::Grid));
    assert_eq!(a.toast(), Some(Toast::ShaderGrid));
    assert_eq!(read_slot_state(d.path()).shader_gba, global);
}

#[test]
fn a_game_without_its_own_leaves_the_screen_row_to_the_default() {
    let d = card();
    let mut a = app_playing_in(d.path(), "Emerald");
    let before = read_slot_state(d.path()).shader_gba;
    go_to(&mut a, QuickRow::Shader);
    press(&mut a, Btn::Right);
    assert_ne!(read_slot_state(d.path()).shader_gba, before);
    assert_eq!(
        own(&d, "Emerald").shader,
        None,
        "upstream's row wrote a profile"
    );
}

#[test]
fn use_defaults_clears_the_game_and_reapplies_the_menus_colour() {
    let d = card();
    with_own(
        &d,
        "Emerald",
        &[ProfileField::FfSpeed(6), ProfileField::Colour(true)],
    );
    let mut a = app_playing_in(d.path(), "Emerald");
    assert!(
        a.colour_correction(),
        "the game's own colour is not in force"
    );
    assert!(
        a.colour_correction_for(Platform::Gba),
        "the core would open without it"
    );
    go_to(&mut a, QuickRow::ThisDefaults);
    assert_eq!(a.quick_value(QuickRow::ThisDefaults), Some(QuickValue::Off));
    press(&mut a, Btn::Right);
    assert_eq!(a.quick_value(QuickRow::ThisDefaults), Some(QuickValue::On));
    assert!(own(&d, "Emerald").is_empty());
    let menu = read_slot_state(d.path());
    assert_eq!(a.ff_speed(), menu.ff_speed);
    assert_eq!(a.colour_correction(), menu.colour_correction_gba);
    assert_eq!(a.take_colour_correction(), Some(menu.colour_correction_gba));
}

#[test]
fn this_games_colour_row_is_its_own_and_reaches_the_core() {
    let d = card();
    let mut a = app_playing_in(d.path(), "Emerald");
    let menu = read_slot_state(d.path()).colour_correction_gba;
    go_to(&mut a, QuickRow::ThisColour);
    press(&mut a, Btn::Right);
    assert_eq!(own(&d, "Emerald").colour, Some(!menu));
    assert_eq!(a.take_colour_correction(), Some(!menu));
    assert_eq!(read_slot_state(d.path()).colour_correction_gba, menu);
}

/// Fragile: This Game → Label keeps the game's own label style, and Defaults gives it back.
#[test]
fn this_game_keeps_its_own_label_style() {
    let d = card();
    let mut a = app_playing_in(d.path(), "Emerald");
    go_to(&mut a, QuickRow::ThisLabel);
    assert_eq!(
        a.named_value(QuickRow::ThisLabel).as_deref(),
        Some("Default")
    );
    press(&mut a, Btn::Left);
    assert_eq!(a.named_value(QuickRow::ThisLabel).as_deref(), Some("Logo"));
    assert_eq!(own(&d, "Emerald").label, Some(slot_store::LabelStyle::Logo));
    assert_eq!(own(&d, "Fusion").label, None, "Fusion took Emerald's style");
    while a.in_game_menu() != Some(QuickRow::ThisDefaults) {
        press(&mut a, Btn::Down);
    }
    press(&mut a, Btn::Right);
    assert_eq!(own(&d, "Emerald").label, None);
}
