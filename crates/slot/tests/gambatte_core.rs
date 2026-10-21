// The real Gambatte core, when SLOT_GAMBATTE_HOST names a build of it for this computer (the
// card's is for the device): every option Fragile sets is one the core declares, for every
// palette, and a game runs. Skipped without it.

use std::path::PathBuf;

use slot_retro::{ButtonMask, LibretroCore, RetroCore, GBA_H, GBA_W};
use slot_store::{Core, GbPalette, Platform};

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
