mod common;

use slot::app::App;
use slot::video_mode::VideoMode;
use slot_store::{read_extras, BezelPick, Platform};
use slot_ui::QuickRow;
use std::path::Path;

fn write_png(dir: &Path, name: &str, w: u32, h: u32, data: &[u8]) {
    std::fs::create_dir_all(dir).unwrap();
    let f = std::fs::File::create(dir.join(name)).unwrap();
    let mut e = png::Encoder::new(std::io::BufWriter::new(f), w, h);
    e.set_color(png::ColorType::Rgba);
    e.set_depth(png::BitDepth::Eight);
    e.write_header().unwrap().write_image_data(data).unwrap();
}

fn app_with_nes_cart(d: &tempfile::TempDir) -> App {
    slot::root::ensure(d.path());
    let nes_game = d.path().join("Games/NES/Mario.nes");
    if !nes_game.exists() {
        std::fs::write(&nes_game, b"fake nes rom").unwrap();
    }
    let mut a = App::boot(d.path());
    a.set_panel((640, 480));
    a
}

#[test]
fn nes_overlay_appears_when_file_exists_and_persists() {
    let d = tempfile::tempdir().unwrap();
    let mut a = app_with_nes_cart(&d);

    // Initially Bezels/NES is empty: overlay row should be hidden.
    assert!(
        a.hidden_rows().contains(&QuickRow::NesBezel),
        "NesBezel should be hidden when Bezels/NES is empty"
    );

    // Write a PNG overlay to Bezels/NES:
    let nes_dir = d.path().join("Bezels/NES");
    let pixel = [255u8, 0, 0, 128];
    write_png(&nes_dir, "Scanlines.png", 1, 1, &pixel);

    // Now the row appears:
    assert!(
        !a.hidden_rows().contains(&QuickRow::NesBezel),
        "NesBezel should appear when Bezels/NES has a PNG"
    );

    // Pick the overlay:
    a.set_platform_bezel(Platform::Nes, BezelPick::File("Scanlines.png".into()));
    assert_eq!(
        a.resolved_bezel(Platform::Nes),
        BezelPick::File("Scanlines.png".into())
    );

    // Extras should persist it:
    let extras = read_extras(d.path());
    assert_eq!(extras.get("bezel.nes"), Some("Scanlines.png"));

    // Rebooting app preserves the pick:
    let a2 = app_with_nes_cart(&d);
    assert_eq!(
        a2.resolved_bezel(Platform::Nes),
        BezelPick::File("Scanlines.png".into())
    );
}

#[test]
fn overlay_is_hidden_in_stretch_mode() {
    let d = tempfile::tempdir().unwrap();
    let mut a = app_with_nes_cart(&d);

    let nes_dir = d.path().join("Bezels/NES");
    write_png(&nes_dir, "CRT.png", 1, 1, &[0, 255, 0, 255]);
    a.set_platform_bezel(Platform::Nes, BezelPick::File("CRT.png".into()));

    // Simulate texture loaded:
    a.set_game_bezel(Some(slot_gfx::TexId::from_raw(42)));
    assert_eq!(a.shown_game_bezel(), Some(slot_gfx::TexId::from_raw(42)));

    // In Stretch mode, overlay must be hidden:
    a.set_video_mode(VideoMode::Stretch);
    assert_eq!(a.shown_game_bezel(), None);
}

#[test]
fn opaque_fullscreen_overlay_has_no_coverage_limit() {
    let d = tempfile::tempdir().unwrap();
    slot::root::ensure(d.path());
    let snes_dir = d.path().join("Bezels/SNES");

    // Full 640x480 opaque image (alpha = 255 everywhere, full panel coverage):
    let raw = vec![128u8; 640 * 480 * 4];
    write_png(&snes_dir, "ShadowMask.png", 640, 480, &raw);

    let list = slot::bezel::list(d.path(), Platform::Snes);
    assert!(list.contains(&"ShadowMask.png".to_string()));

    let mut a = App::boot(d.path());
    a.set_panel((640, 480));
    a.set_platform_bezel(Platform::Snes, BezelPick::File("ShadowMask.png".into()));
    assert_eq!(
        a.resolved_bezel(Platform::Snes),
        BezelPick::File("ShadowMask.png".into())
    );
}

#[test]
fn gba_integer_companion_resolution() {
    let d = tempfile::tempdir().unwrap();
    slot::root::ensure(d.path());
    let gba_dir = d.path().join("Bezels/GBA");

    write_png(&gba_dir, "Border.png", 2, 2, &[0; 16]);
    write_png(&gba_dir, "Border 2x.png", 2, 2, &[0; 16]);

    // Companion must be hidden from list():
    let files = slot::bezel::list(d.path(), Platform::Gba);
    assert!(files.contains(&"Border.png".to_string()));
    assert!(!files.contains(&"Border 2x.png".to_string()));

    // for_integer should pick the 2x companion on GBA:
    let companion = slot::bezel::for_integer(
        d.path(),
        Platform::Gba,
        BezelPick::File("Border.png".into()),
    );
    assert_eq!(companion, BezelPick::File("Border 2x.png".into()));

    // When companion does not exist, returns None:
    let non_existent =
        slot::bezel::for_integer(d.path(), Platform::Gba, BezelPick::File("Other.png".into()));
    assert_eq!(non_existent, BezelPick::None);
}

#[test]
fn native_systems_hide_overlay_in_integer_scale() {
    let d = tempfile::tempdir().unwrap();
    slot::root::ensure(d.path());

    for p in [
        Platform::Nes,
        Platform::Snes,
        Platform::PlayStation,
        Platform::Nds,
    ] {
        let pick = slot::bezel::for_integer(d.path(), p, BezelPick::File("Custom.png".into()));
        assert_eq!(
            pick,
            BezelPick::None,
            "Native platform {p:?} should hide overlay in integer scale"
        );
    }
}

#[test]
fn ps1_folder_alias_fallback() {
    let d = tempfile::tempdir().unwrap();
    slot::root::ensure(d.path());

    // User puts overlay in Bezels/PS1 instead of Bezels/PS:
    let ps1_dir = d.path().join("Bezels/PS1");
    write_png(&ps1_dir, "CrtGrid.png", 1, 1, &[0; 4]);

    let files = slot::bezel::list(d.path(), Platform::PlayStation);
    assert!(
        files.contains(&"CrtGrid.png".to_string()),
        "Bezels/PS1 folder should be found when Bezels/PS does not have it"
    );

    let path = slot::bezel::file_path(d.path(), Platform::PlayStation, "CrtGrid.png");
    assert!(path.is_file(), "Resolved path should exist in Bezels/PS1");
}
