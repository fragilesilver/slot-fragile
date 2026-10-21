mod common;

use slot::app::{App, GameMenu, Phase};
use slot_input::{Action, Btn};
use slot_store::{core_for_platform, Core, Platform, StateRing};
use tempfile::TempDir;

#[test]
fn game_boy_games_start_on_gambatte_and_gba_games_on_mgba() {
    let d = tempfile::tempdir().unwrap();
    assert_eq!(
        core_for_platform(d.path(), "Tetris", Platform::Gb),
        Core::Gambatte
    );
    assert_eq!(
        core_for_platform(d.path(), "Zelda", Platform::Gbc),
        Core::Gambatte
    );
    assert_eq!(
        core_for_platform(d.path(), "Emerald", Platform::Gba),
        Core::Mgba
    );
    slot_store::write_selected_core(d.path(), "Tetris", Core::Mgba).unwrap();
    assert_eq!(
        core_for_platform(d.path(), "Tetris", Platform::Gb),
        Core::Mgba
    );
    slot_store::write_selected_core(d.path(), "Emerald", Core::Gambatte).unwrap();
    assert_eq!(
        core_for_platform(d.path(), "Emerald", Platform::Gba),
        Core::Mgba,
        "a GBA game was put on Gambatte"
    );
}

#[test]
fn start_on_a_game_boy_cart_offers_gambatte_and_mgba() {
    let d = common::tmp_root_with_gb_carts(&["Tetris", "Zelda"]);
    let mut a = common::boot(d.path());
    a.apply(Action::GbaDown(Btn::Start));
    assert_eq!(a.core_picker(), Some(Core::Gambatte));
    assert_eq!(a.core_picker_choices(), Some([Core::Gambatte, Core::Mgba]));
    let d = common::tmp_root_with_carts(&["Emerald", "Fusion"]);
    let mut a = common::boot(d.path());
    a.apply(Action::GbaDown(Btn::Start));
    assert_eq!(
        a.core_picker_choices(),
        Some([Core::Mgba, Core::Gpsp]),
        "the GBA's picker changed"
    );
}

fn playing_gb_on_gambatte() -> (App, TempDir) {
    let d = common::tmp_root_with_gb_carts(&["Tetris", "Zelda"]);
    let mut app = common::boot(d.path());
    app.apply(Action::Insert);
    app.set_core(Core::Gambatte);
    app.on_core_ready();
    for _ in 0..120 {
        app.update(1.0 / 60.0);
    }
    assert!(matches!(app.phase(), Phase::Playing { .. }), "never seated");
    (app, d)
}

#[test]
fn a_game_on_gambatte_can_open_the_link_screen() {
    let (mut app, _d) = playing_gb_on_gambatte();
    app.apply(Action::GameMenu);
    assert!(
        matches!(app.game_menu(), Some(GameMenu::Pick(_))),
        "Gambatte refused the link it hands to mGBA"
    );
}

#[test]
fn linking_a_gambatte_game_opens_it_on_mgba_and_keeps_the_resumes_as_states() {
    let (mut app, d) = playing_gb_on_gambatte();
    let stem = app.seated_cart().unwrap().stem.clone();
    let rings = [Core::Gambatte, Core::Mgba]
        .map(|core| StateRing::new(d.path(), Platform::Gb, core, &stem));
    for ring in &rings {
        ring.write_resume(b"state").unwrap();
    }
    assert_eq!(
        app.core_to_open(&stem, Platform::Gb, Core::Gambatte),
        (Core::Gambatte, false),
        "a game not linking was moved off Gambatte"
    );
    app.apply(Action::GameMenu);
    app.apply(Action::GbaDown(Btn::A));
    assert!(app.link_player().is_some(), "picking a role set no player");
    assert_eq!(
        app.core_to_open(&stem, Platform::Gb, Core::Gambatte),
        (Core::Mgba, true)
    );
    for ring in &rings {
        assert_eq!(
            ring.read_resume().unwrap(),
            None,
            "a resume was left to load"
        );
        assert_eq!(
            ring.list().unwrap().len(),
            1,
            "a resume was not kept as a state"
        );
    }
    assert_eq!(
        app.core_to_open(&stem, Platform::Gba, Core::Gpsp),
        (Core::Gpsp, false),
        "a gpSP game was moved off the core it links on"
    );
}
