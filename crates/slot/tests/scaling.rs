mod common;

use std::path::Path;

use common::{app_playing_in, boot, tmp_root_with_carts, write_gb_cart};
use slot::app::App;
use slot_input::{Action, Btn};
use slot_store::{
    read_extras, write_extras, write_slot_state, ExtrasState, Platform, Scaling, SlotState,
};
use slot_ui::QuickRow;
use tempfile::TempDir;

fn card() -> TempDir {
    let d = tmp_root_with_carts(&["Emerald"]);
    write_gb_cart(&d, "Tetris", "TETRIS");
    for (dir, file) in [
        ("NES", "Mario.nes"),
        ("SNES", "Mario.sfc"),
        ("PS", "Mario.cue"),
    ] {
        let games = d.path().join("Games").join(dir);
        std::fs::create_dir_all(&games).unwrap();
        std::fs::write(games.join(file), vec![0u8; 0x8000]).unwrap();
    }
    d
}

fn set_scaling(d: &TempDir, platform: Platform, scaling: Scaling) {
    let mut e = read_extras(d.path());
    slot_store::set_scaling(&mut e, platform, scaling);
    write_extras(d.path(), &e).unwrap();
}

fn playing(d: &TempDir, stem: &str, panel: (u32, u32)) -> App {
    let mut a = app_playing_in(d.path(), stem);
    a.set_panel(panel);
    a
}

fn seated(root: &Path, platform: Platform, panel: (u32, u32)) -> App {
    let stem = match platform {
        Platform::Gba => "Emerald",
        Platform::Gb => "Tetris",
        _ => "Mario",
    };
    write_slot_state(
        root,
        &SlotState {
            cart: Some(stem.to_string()),
            cart_platform: Some(platform),
            clock_set: true,
            ..Default::default()
        },
    )
    .unwrap();
    let mut a = boot(root);
    a.set_panel(panel);
    a
}

#[test]
fn picture_rect_per_mode_per_system_at_640x480_720x480_and_720x720() {
    let d = card();

    // 1. GBA (240x160)
    // 640x480 (4:3 panel):
    // - Fit: game_rect (640x427), integer_picture is None, sharp is false
    set_scaling(&d, Platform::Gba, Scaling::Fit);
    let a = playing(&d, "Emerald", (640, 480));
    assert_eq!(a.integer_picture(), None);
    assert!(!a.sharp_bilinear());
    assert_eq!(slot_gfx::game_rect((640, 480), false), (0, 27, 640, 427));

    // - Sharp Fit: game_rect (640x427), integer_picture is None, sharp is true
    set_scaling(&d, Platform::Gba, Scaling::SharpFit);
    let a = playing(&d, "Emerald", (640, 480));
    assert_eq!(a.integer_picture(), None);
    assert!(a.sharp_bilinear());

    // - Integer: integer_rect 2x (480x320), integer_picture is Some((240, 160)), sharp is false
    set_scaling(&d, Platform::Gba, Scaling::Integer);
    let a = playing(&d, "Emerald", (640, 480));
    assert_eq!(a.integer_picture(), Some((240, 160)));
    assert!(!a.sharp_bilinear());
    assert_eq!(
        slot_gfx::integer_rect((640, 480), (240, 160)),
        (80, 80, 480, 320)
    );

    // 720x480 (3:2 native): framed_panel is false, so Integer does not squeeze
    set_scaling(&d, Platform::Gba, Scaling::Integer);
    let a = playing(&d, "Emerald", (720, 480));
    assert_eq!(a.integer_picture(), None);

    // 720x720 (1:1 square panel):
    set_scaling(&d, Platform::Gba, Scaling::Fit);
    let a = playing(&d, "Emerald", (720, 720));
    assert_eq!(a.integer_picture(), None);
    assert_eq!(slot_gfx::game_rect((720, 720), false), (0, 120, 720, 480));

    // 2. Game Boy (160x144)
    // 640x480:
    set_scaling(&d, Platform::Gb, Scaling::Fit);
    let a = playing(&d, "Tetris", (640, 480));
    assert_eq!(a.integer_picture(), None);
    assert!(!a.sharp_bilinear());

    set_scaling(&d, Platform::Gb, Scaling::SharpFit);
    let a = playing(&d, "Tetris", (640, 480));
    assert_eq!(a.integer_picture(), None);
    assert!(a.sharp_bilinear());

    set_scaling(&d, Platform::Gb, Scaling::Integer);
    let a = playing(&d, "Tetris", (640, 480));
    assert_eq!(a.integer_picture(), Some((160, 144)));
    assert_eq!(
        slot_gfx::integer_rect((640, 480), (160, 144)),
        (80, 24, 480, 432)
    );

    // 3. SNES (Native system: 256x224)
    set_scaling(&d, Platform::Snes, Scaling::Fit);
    let a = seated(d.path(), Platform::Snes, (640, 480));
    assert!(!a.integer_native());
    assert_eq!(a.integer_picture(), None);

    set_scaling(&d, Platform::Snes, Scaling::Integer);
    let a = seated(d.path(), Platform::Snes, (640, 480));
    assert!(a.integer_native());
    assert_eq!(
        slot_gfx::integer_rect((640, 480), (256, 224)),
        (64, 16, 512, 448)
    );
}

#[test]
fn scaling_migration_from_old_integer_flags() {
    let d = card();

    // Absent entries: GBA defaults to SharpFit, others default to Fit
    let a = boot(d.path());
    assert_eq!(a.scaling(Platform::Gba), Scaling::SharpFit);
    assert_eq!(a.scaling(Platform::Gb), Scaling::Fit);
    assert_eq!(a.scaling(Platform::Gbc), Scaling::Fit);
    assert_eq!(a.scaling(Platform::Nes), Scaling::Fit);
    assert_eq!(a.scaling(Platform::Snes), Scaling::Fit);
    assert_eq!(a.scaling(Platform::PlayStation), Scaling::Fit);

    // Old integer_<platform> flag reads as Integer
    let mut extras = ExtrasState::default();
    extras.set_flag("integer_gba", true);
    extras.set_flag("integer_gb", true);
    extras.set_flag("integer_snes", true);
    write_extras(d.path(), &extras).unwrap();

    let a = boot(d.path());
    assert_eq!(a.scaling(Platform::Gba), Scaling::Integer);
    assert_eq!(a.scaling(Platform::Gb), Scaling::Integer);
    assert_eq!(a.scaling(Platform::Snes), Scaling::Integer);
    assert_eq!(a.scaling(Platform::Gbc), Scaling::Fit);
}

#[test]
fn scaling_stepping_and_named_values_on_screen_menu() {
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
    walk(&mut a, QuickRow::ScreenGba);
    a.apply(Action::GbaDown(Btn::A));
    walk(&mut a, QuickRow::IntegerGba);

    assert_eq!(a.quick_menu(), Some(QuickRow::IntegerGba));
    // GBA starts at Sharp Fit
    assert_eq!(
        a.named_value(QuickRow::IntegerGba),
        Some("Sharp Fit".into())
    );
    assert_eq!(a.scaling(Platform::Gba), Scaling::SharpFit);

    // Step right -> Integer
    a.apply(Action::GbaDown(Btn::Right));
    assert_eq!(a.named_value(QuickRow::IntegerGba), Some("Integer".into()));
    assert_eq!(a.scaling(Platform::Gba), Scaling::Integer);

    // Step right -> Fit
    a.apply(Action::GbaDown(Btn::Right));
    assert_eq!(a.named_value(QuickRow::IntegerGba), Some("Fit".into()));
    assert_eq!(a.scaling(Platform::Gba), Scaling::Fit);

    // Step right -> Fill
    a.apply(Action::GbaDown(Btn::Right));
    assert_eq!(a.named_value(QuickRow::IntegerGba), Some("Fill".into()));
    assert_eq!(a.scaling(Platform::Gba), Scaling::Fill);

    // Step right -> Sharp Fit
    a.apply(Action::GbaDown(Btn::Right));
    assert_eq!(
        a.named_value(QuickRow::IntegerGba),
        Some("Sharp Fit".into())
    );
    assert_eq!(a.scaling(Platform::Gba), Scaling::SharpFit);

    // Step left -> Fill
    a.apply(Action::GbaDown(Btn::Left));
    assert_eq!(a.named_value(QuickRow::IntegerGba), Some("Fill".into()));
    assert_eq!(a.scaling(Platform::Gba), Scaling::Fill);
}
