// The real Gambatte core, when SLOT_GAMBATTE_HOST names a build of it for this computer (the
// card's is for the device): every option Fragile sets is one the core declares, for every
// palette, and a game runs. Skipped without it.

use std::path::PathBuf;
use std::sync::Mutex;

use slot_retro::{ButtonMask, LibretroCore, RetroCore, GBA_H, GBA_W};
use slot_store::{Core, GbPalette, Platform};

/// One core can be open at a time in a process.
static ONE_CORE: Mutex<()> = Mutex::new(());

fn core_path() -> Option<PathBuf> {
    let p = PathBuf::from(std::env::var_os("SLOT_GAMBATTE_HOST")?);
    p.exists().then_some(p)
}

/// A 32 KiB ROM-only cart whose code loops on the spot, with a header Gambatte accepts.
fn rom() -> Vec<u8> {
    let mut rom = vec![0u8; 0x8000];
    rom[0x100..0x104].copy_from_slice(&[0x00, 0xc3, 0x50, 0x01]);
    rom[0x134..0x13a].copy_from_slice(b"FRAGIL");
    rom[0x150..0x153].copy_from_slice(&[0xc3, 0x50, 0x01]);
    let check = rom[0x134..0x14d]
        .iter()
        .fold(0u8, |c, b| c.wrapping_sub(*b).wrapping_sub(1));
    rom[0x14d] = check;
    rom
}

#[test]
fn every_option_fragile_gives_gambatte_is_one_it_declares() {
    let Some(path) = core_path() else {
        eprintln!("skipping: SLOT_GAMBATTE_HOST is not set");
        return;
    };
    let _one = ONE_CORE.lock().unwrap_or_else(|e| e.into_inner());
    let d = tempfile::tempdir().unwrap();
    let rom_path = d.path().join("Fragile.gb");
    std::fs::write(&rom_path, rom()).unwrap();
    let mut core =
        LibretroCore::open_with_options(&path, d.path(), d.path(), slot::gambatte::BEFORE_INIT)
            .expect("open gambatte");
    let declared = core.declared_options().clone();
    assert!(!declared.is_empty(), "Gambatte declared no options");
    for palette in GbPalette::all().map(Some).chain([None]) {
        for colour in [false, true] {
            slot::core::apply_core_options(
                &mut core,
                Core::Gambatte,
                Platform::Gb,
                "",
                false,
                colour,
                palette,
            );
            for on in [false, true] {
                let (key, value) = slot::core::motion_blur_option(Core::Gambatte, on);
                core.set_option(key, value);
            }
            for (key, value) in core.options() {
                let values = declared
                    .get(&key)
                    .unwrap_or_else(|| panic!("Gambatte has no option {key}"));
                assert!(
                    values.contains(&value),
                    "Gambatte's {key} has no {value:?} ({:?})",
                    palette.map(GbPalette::core_name)
                );
            }
        }
    }
    core.load(&rom_path).expect("load the rom");
    for _ in 0..30 {
        core.run_frame(ButtonMask::default());
    }
    assert_eq!(
        core.video_size(),
        (GBA_W, GBA_H),
        "a Game Boy left its canvas"
    );
    assert_eq!(core.video_xrgb8888().len(), (GBA_W * GBA_H * 4) as usize);
}

/// The colour at the middle of the Game Boy's picture in the 240x160 canvas.
fn middle(core: &LibretroCore) -> [u8; 3] {
    let at = ((GBA_H / 2 * GBA_W + GBA_W / 2) * 4) as usize;
    let px = &core.video_xrgb8888()[at..at + 3];
    [px[0], px[1], px[2]]
}

#[test]
fn a_palette_changed_while_playing_recolours_the_game() {
    let Some(path) = core_path() else {
        eprintln!("skipping: SLOT_GAMBATTE_HOST is not set");
        return;
    };
    let _one = ONE_CORE.lock().unwrap_or_else(|e| e.into_inner());
    let d = tempfile::tempdir().unwrap();
    let rom_path = d.path().join("Fragile.gb");
    std::fs::write(&rom_path, rom()).unwrap();
    let mut core =
        LibretroCore::open_with_options(&path, d.path(), d.path(), slot::gambatte::BEFORE_INIT)
            .expect("open gambatte");
    let named = |name: &str| GbPalette::all().find(|p| p.core_name() == name).unwrap();
    let mut seen = Vec::new();
    // The test cart shows only a palette's lightest shade, so it switches to Grayscale and back.
    for name in ["DMG Green", "Grayscale", "DMG Green"] {
        for (key, value) in slot::gambatte::palette_options(Core::Gambatte, named(name)) {
            core.set_option(key, &value);
        }
        if seen.is_empty() {
            core.load(&rom_path).expect("load the rom");
        }
        for _ in 0..10 {
            core.run_frame(ButtonMask::default());
        }
        seen.push(middle(&core));
    }
    assert_ne!(
        seen[0], seen[1],
        "Grayscale did not recolour the game: {seen:?}"
    );
    assert_ne!(
        seen[1], seen[2],
        "GBC Red did not recolour the game: {seen:?}"
    );
}
