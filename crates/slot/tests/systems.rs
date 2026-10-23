// Fragile: the menus and buttons of the systems Phase 12 adds, NES, SNES and PlayStation.

mod common;

use std::path::Path;

use common::{tmp_root_with_carts, StubSnapshot};
use slot::app::App;
use slot_input::{Action, Btn};
use slot_store::{read_extras, write_extras, write_slot_state, Platform, SlotState};
use slot_ui::QuickRow;
use tempfile::TempDir;

/// A card with a game on each new system, playing the one asked for.
fn playing(platform: Platform) -> (TempDir, App) {
    let d = tmp_root_with_carts(&["Emerald"]);
    for (dir, file) in [
        ("NES", "Mario.nes"),
        ("SNES", "Mario.sfc"),
        ("PS", "Mario.cue"),
    ] {
        let games = d.path().join("Games").join(dir);
        std::fs::create_dir_all(&games).unwrap();
        std::fs::write(games.join(file), vec![0u8; 0x8000]).unwrap();
    }
    let a = seated(d.path(), platform);
    (d, a)
}

fn seated(root: &Path, platform: Platform) -> App {
    let stem = match platform {
        Platform::Gba => "Emerald",
        _ => "Mario",
    };
    write_slot_state(
        root,
        &SlotState {
            cart: Some(stem.to_string()),
            cart_platform: Some(platform),
            clock_set: true,
            utc_offset_min: 0,
            turbo: true,
            ..Default::default()
        },
    )
    .unwrap();
    let mut a = App::boot(root);
    a.set_snapshot(StubSnapshot::boxed());
    a.on_core_ready();
    for _ in 0..120 {
        a.update(1.0 / 60.0);
    }
    assert_eq!(
        a.playing_platform(),
        platform,
        "not playing the game asked for"
    );
    a
}

fn press(a: &mut App, btn: Btn) {
    a.apply(Action::GbaDown(btn));
    a.apply(Action::GbaUp(btn));
}

#[test]
fn each_pad_shows_only_its_own_buttons_in_game() {
    let pads = [
        (
            Platform::Gba,
            &[QuickRow::GameL, QuickRow::GameR][..],
            &[
                QuickRow::GameX,
                QuickRow::GameY,
                QuickRow::GameL2,
                QuickRow::GameR2,
            ][..],
        ),
        (
            Platform::Nes,
            &[QuickRow::GameA, QuickRow::GameTurboA][..],
            &[QuickRow::GameL, QuickRow::GameX, QuickRow::GameL2][..],
        ),
        (
            Platform::Snes,
            &[
                QuickRow::GameX,
                QuickRow::GameY,
                QuickRow::GameL,
                QuickRow::GameR,
            ][..],
            &[
                QuickRow::GameTurboA,
                QuickRow::GameTurboB,
                QuickRow::GameL2,
                QuickRow::GameR2,
            ][..],
        ),
        (
            Platform::PlayStation,
            &[QuickRow::GameX, QuickRow::GameL2, QuickRow::GameR2][..],
            &[QuickRow::GameTurboA, QuickRow::GameTurboB][..],
        ),
    ];
    for (platform, shown, hidden_rows) in pads {
        let (_d, a) = playing(platform);
        let hidden = a.in_game_hidden();
        for row in shown {
            assert!(!hidden.contains(row), "{platform:?} hides {row:?}");
        }
        for row in hidden_rows {
            assert!(hidden.contains(row), "{platform:?} shows {row:?}");
        }
    }
}

#[test]
fn a_system_without_colour_correction_or_bezels_hides_those_rows() {
    let (_d, a) = playing(Platform::Snes);
    let hidden = a.in_game_hidden();
    assert!(hidden.contains(&QuickRow::ColourCorrection));
    assert!(hidden.contains(&QuickRow::Bezel));
    assert!(!hidden.contains(&QuickRow::Shader));
    let (_d, a) = playing(Platform::Gba);
    assert!(!a.in_game_hidden().contains(&QuickRow::ColourCorrection));
}

#[test]
fn a_playstation_game_keeps_l2_and_r2_and_a_gba_game_fast_forwards_on_r2() {
    use slot_input::Action as A;
    let fires = |a: &App, action: A| {
        let mut entries = a.binding_table().entries();
        a.yield_to_pad(&mut entries);
        entries
            .iter()
            .any(|e| e.action == action && e.chord.modifier.is_none())
    };
    let (_d, a) = playing(Platform::PlayStation);
    assert!(
        !fires(&a, A::FfStart),
        "R2 fast forwards a PlayStation game"
    );
    assert!(!fires(&a, A::RewindStart), "L2 rewinds a PlayStation game");
    let chorded = |a: &App, action: A| {
        let mut entries = a.binding_table().entries();
        a.yield_to_pad(&mut entries);
        entries
            .iter()
            .any(|e| e.action == action && e.chord.modifier == Some(slot_input::Mod::Select))
    };
    assert!(
        chorded(&a, A::FfStart),
        "PlayStation fast forward is not on SELECT+R2"
    );
    assert!(
        chorded(&a, A::RewindStart),
        "PlayStation rewind is not on SELECT+L2"
    );
    let (_d, a) = playing(Platform::Snes);
    assert!(fires(&a, A::FfStart), "the SNES pad has no R2 for the game");
    let (_d, a) = playing(Platform::Gba);
    assert!(
        fires(&a, A::FfStart),
        "upstream's R2 fast forward went away"
    );
}

#[test]
fn the_screen_page_has_a_page_for_each_new_system() {
    let (d, mut a) = playing(Platform::Nes);
    assert!(QuickRow::SCREEN.contains(&QuickRow::ScreenNes));
    assert_eq!(QuickRow::ScreenSnes.child(), Some(QuickRow::SnesShader));
    assert_eq!(QuickRow::PsShader.parent(), Some(QuickRow::ScreenPs));
    a.apply(Action::QuickMenu);
    for _ in 0..4 {
        if a.in_game_menu() == Some(QuickRow::Settings) {
            break;
        }
        press(&mut a, Btn::Down);
    }
    press(&mut a, Btn::A);
    assert_eq!(a.in_game_menu(), Some(QuickRow::PlayScreen));
    press(&mut a, Btn::A);
    assert_eq!(a.in_game_menu(), Some(QuickRow::Shader));
    let before = a.quick_value(QuickRow::Shader);
    press(&mut a, Btn::Right);
    let after = a.quick_value(QuickRow::Shader);
    assert_ne!(before, after, "the NES Shader row did not move");
    assert_eq!(a.quick_value(QuickRow::NesShader), after);
    assert!(
        read_extras(d.path()).get("shader.nes").is_some(),
        "the NES shader was not kept in extras.state"
    );
}

#[test]
fn integer_scale_on_a_native_system_asks_for_whole_square_pixels() {
    let (d, a) = playing(Platform::Snes);
    assert!(!a.integer_native());
    let mut extras = read_extras(d.path());
    extras.set_flag("integer_snes", true);
    write_extras(d.path(), &extras).unwrap();
    let a = seated(d.path(), Platform::Snes);
    assert!(a.integer_native());
    assert_eq!(
        a.integer_picture(),
        None,
        "the Game Boy canvas path took a SNES game"
    );
    let a = seated(d.path(), Platform::Nes);
    assert!(!a.integer_native(), "the SNES setting reached the NES");
}

#[test]
fn two_buttons_clash_only_on_a_pad_that_has_both() {
    use slot_store::GameButton as G;
    assert!(
        !G::on_one_pad(G::TurboA, G::X),
        "no pad has turbo and its own X"
    );
    assert!(!G::on_one_pad(G::TurboB, G::Y));
    assert!(G::on_one_pad(G::X, G::A), "the SNES pad has both");
    assert!(G::on_one_pad(G::L2, G::R), "the PlayStation pad has both");
    assert!(G::on_one_pad(G::TurboA, G::L), "the GBA pad has both");
}

#[test]
fn select_and_r2_fast_forward_a_playstation_game_while_held() {
    use slot_input::{Action as A, Context, Gestures, RawEvent};
    let (_d, a) = playing(Platform::PlayStation);
    let mut entries = a.binding_table().entries();
    a.yield_to_pad(&mut entries);
    let mut g = Gestures::new();
    g.set_entries(entries);
    g.set_context(Context::Game);
    let mut seen = Vec::new();
    for (at, ev) in [
        (0, RawEvent::Down(Btn::Select)),
        (40, RawEvent::Down(Btn::R2)),
        (400, RawEvent::Up(Btn::R2)),
        (440, RawEvent::Up(Btn::Select)),
        (900, RawEvent::Down(Btn::R2)),
        (950, RawEvent::Up(Btn::R2)),
    ] {
        seen.extend(g.feed(ev, at));
        seen.extend(g.tick(at));
    }
    let ff: Vec<_> = seen
        .iter()
        .filter(|x| matches!(x, A::FfStart | A::FfStop))
        .collect();
    assert_eq!(ff, [&A::FfStart, &A::FfStop], "{seen:?}");
}

#[test]
fn the_playstation_guide_tells_of_select_r2_and_lists_its_own_buttons() {
    let (_d, mut a) = playing(Platform::PlayStation);
    a.apply(Action::QuickMenu);
    for row in [QuickRow::Settings, QuickRow::PlayGuide] {
        for _ in 0..12 {
            if a.in_game_menu() == Some(row) {
                break;
            }
            press(&mut a, Btn::Down);
        }
        press(&mut a, Btn::A);
    }
    let page = a.overlay_page().expect("the guide did not open").clone();
    let value = |label: &str| {
        page.rows()
            .iter()
            .find(|r| r.label == label)
            .and_then(|r| r.value.clone())
    };
    assert_eq!(value("Fast Forward").as_deref(), Some("SEL+R2"));
    assert_eq!(value("Rewind").as_deref(), Some("SEL+L2"));
    assert!(value("R2 Button").is_some(), "the pad's R2 is not listed");
    assert!(value("Turbo A").is_none(), "a PlayStation pad has no turbo");
}

#[test]
fn every_menu_value_answers_on_every_new_system() {
    for platform in [Platform::Nes, Platform::Snes, Platform::PlayStation] {
        let (_d, a) = playing(platform);
        // The menu works out every row's value as it draws; a row that stood for itself
        // (Color Correction on the NES) asked itself forever.
        let values: Vec<_> = QuickRow::ALL.iter().map(|r| a.quick_value(*r)).collect();
        assert_eq!(values.len(), QuickRow::ALL.len(), "{platform:?}");
        assert_eq!(
            a.quick_value(QuickRow::ColourCorrection),
            None,
            "{platform:?}"
        );
    }
}
