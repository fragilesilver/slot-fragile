// Ported from nickclyde/slot-plus@29b8a81, crates/slot-cheevos/src/game.rs.

use std::ffi::c_char;
use std::path::Path;

pub const GB: u32 = 4;
pub const GBA: u32 = 5;
pub const GBC: u32 = 6;

const CGB_FLAG: usize = 0x143;

pub fn console_for(rom_path: &Path, rom: &[u8]) -> u32 {
    let ext = rom_path
        .extension()
        .and_then(|e| e.to_str())
        .map(str::to_ascii_lowercase);
    match ext.as_deref() {
        Some("gba") => GBA,
        _ if rom.get(CGB_FLAG).is_some_and(|f| f & 0x80 != 0) => GBC,
        _ => GB,
    }
}

pub fn hash(console: u32, rom: &[u8]) -> Option<String> {
    let mut out = [0 as c_char; 33];
    let ok = unsafe {
        crate::ffi::rc_hash_generate_from_buffer(out.as_mut_ptr(), console, rom.as_ptr(), rom.len())
    };
    (ok != 0).then(|| unsafe { crate::ffi::text(out.as_ptr()) })
}
