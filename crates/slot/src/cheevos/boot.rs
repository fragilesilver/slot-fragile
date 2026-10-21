// Ported from nickclyde/slot-plus@c218577, crates/slot/src/cheevos/boot.rs.

use std::path::Path;

use slot_store::Platform;

use crate::root;

pub const FRAMES: u32 = 300;

const GB_BOOT_ROMS: [&str; 3] = ["gb_bios.bin", "gbc_bios.bin", "sgb_bios.bin"];

pub fn frames(root: &Path, rom: &Path, resumed: bool) -> u32 {
    if resumed || !boot_rom(root, rom) {
        0
    } else {
        FRAMES
    }
}

fn boot_rom(root: &Path, rom: &Path) -> bool {
    if Platform::Gba.accepts(rom) {
        root::has_real_bios(root)
    } else {
        let dir = root::bios_dir(root);
        GB_BOOT_ROMS.iter().any(|f| dir.join(f).is_file())
    }
}
