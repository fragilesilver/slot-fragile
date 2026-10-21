mod common;

use common::{app_playing_in, boot, tmp_root_with_carts, write_gb_cart};
use slot::app::App;
use slot_input::{Action, Btn};
use slot_store::{read_extras, write_extras, BezelPick, Platform};
use slot_ui::{QuickRow, QuickValue};
use tempfile::TempDir;

fn card() -> TempDir {
    let d = tmp_root_with_carts(&["Emerald"]);
    write_gb_cart(&d, "Tetris", "TETRIS");
    d
}

fn set(d: &TempDir, key: &str, on: bool) {
    let mut e = read_extras(d.path());
    e.set_flag(key, on);
    write_extras(d.path(), &e).unwrap();
}

fn playing(d: &TempDir, stem: &str, panel: (u32, u32)) -> App {
    let mut a = app_playing_in(d.path(), stem);
    a.set_panel(panel);
    a
}

#[test]
fn a_game_boy_game_is_drawn_at_3x_on_a_4_by_3_panel_only_when_asked() {
    let d = card();
    assert_eq!(playing(&d, "Tetris", (640, 480)).integer_picture(), None);
    set(&d, "integer_gb", true);
    assert_eq!(
        playing(&d, "Tetris", (640, 480)).integer_picture(),
        Some((160, 144))
    );
    assert_eq!(
        playing(&d, "Tetris", (720, 480)).integer_picture(),
        None,
        "a 3:2 panel has no room for a whole multiple"
    );
    assert_eq!(
        playing(&d, "Emerald", (640, 480)).integer_picture(),
        None,
        "the GB setting reached a GBA game"
    );
}

#[test]
fn a_gba_game_at_2x_gives_up_the_sp_frame() {
    let d = card();
    let a = playing(&d, "Emerald", (640, 480));
    assert!(a.sp_frame(), "a 4:3 panel starts on the SP frame");
    set(&d, "integer_gba", true);
    let a = playing(&d, "Emerald", (640, 480));
    assert_eq!(a.integer_picture(), Some((240, 160)));
    assert!(
        !a.sp_frame(),
        "the SP frame's window would cut the 2x picture"
    );
    assert_eq!(
        slot_gfx::integer_rect((640, 480), (240, 160)),
        (80, 80, 480, 320)
    );
}

#[test]
fn a_bezel_shows_at_3x_only_when_the_card_has_its_3x_version() {
    let d = card();
    let gb = d.path().join("Bezels/GB");
    std::fs::write(gb.join("Classic.png"), b"png").unwrap();
    std::fs::write(gb.join("Classic 3x.png"), b"png").unwrap();
    std::fs::write(gb.join("Pocket.png"), b"png").unwrap();
    let pick =
        |name: &str| slot::bezel::for_integer(d.path(), Platform::Gb, BezelPick::File(name.into()));
    assert_eq!(
        pick("Classic.png"),
        BezelPick::File("Classic 3x.png".into())
    );
    assert_eq!(pick("Pocket.png"), BezelPick::None);
    assert_eq!(
        slot::bezel::for_integer(d.path(), Platform::Gb, BezelPick::Default),
        BezelPick::None
    );
    assert_eq!(
        slot::bezel::list(d.path(), Platform::Gb),
        ["Classic.png", "Pocket.png"],
        "the 3x version is offered as a bezel of its own"
    );
}

#[test]
fn the_row_on_each_consoles_screen_page_flips_and_saves() {
    let d = card();
    let mut a = boot(d.path());
    a.apply(Action::QuickMenu);
    let walk = |a: &mut App, row: QuickRow| {
        for _ in 0..row.position(&a.hidden_rows()) {
            a.apply(Action::GbaDown(Btn::Down));
        }
    };
    walk(&mut a, QuickRow::Screen);
    a.apply(Action::GbaDown(Btn::A));
    walk(&mut a, QuickRow::ScreenGb);
    a.apply(Action::GbaDown(Btn::A));
    walk(&mut a, QuickRow::IntegerGb);
    assert_eq!(a.quick_menu(), Some(QuickRow::IntegerGb));
    assert_eq!(a.quick_value(QuickRow::IntegerGb), Some(QuickValue::Off));
    a.apply(Action::GbaDown(Btn::Right));
    assert!(a.integer_scale(Platform::Gb));
    assert!(!a.integer_scale(Platform::Gbc), "GBC followed GB");
    assert_eq!(read_extras(d.path()).get("integer_gb"), Some("1"));
}
