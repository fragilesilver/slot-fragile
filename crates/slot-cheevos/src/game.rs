// Ported from nickclyde/slot-plus@29b8a81, crates/slot-cheevos/src/game.rs.

use std::ffi::{c_char, CString};
use std::path::Path;

pub const SNES: u32 = 3;
pub const GB: u32 = 4;
pub const GBA: u32 = 5;
pub const GBC: u32 = 6;
pub const NES: u32 = 7;
pub const PS1: u32 = 12;
pub const NDS: u32 = 18;

const CGB_FLAG: usize = 0x143;

pub fn console_for(rom_path: &Path, rom: &[u8]) -> u32 {
    let ext = rom_path
        .extension()
        .and_then(|e| e.to_str())
        .map(str::to_ascii_lowercase);
    match ext.as_deref() {
        Some("gba") => GBA,
        Some("nes" | "unf" | "unif" | "fds") => NES,
        Some("sfc" | "smc" | "swc" | "fig" | "bs") => SNES,
        Some("cue" | "chd" | "pbp" | "m3u" | "iso" | "img") => PS1,
        Some("nds" | "srl" | "dsi") => NDS,
        _ => {
            if rom_path.iter().any(|c| c == "PS" || c == "PS1") {
                PS1
            } else if rom_path.iter().any(|c| c == "NES") {
                NES
            } else if rom_path.iter().any(|c| c == "SNES") {
                SNES
            } else if rom_path.iter().any(|c| c == "NDS") {
                NDS
            } else if rom_path.iter().any(|c| c == "GBA") {
                GBA
            } else if rom.get(CGB_FLAG).is_some_and(|f| f & 0x80 != 0) {
                GBC
            } else {
                GB
            }
        }
    }
}

pub fn hash(console: u32, rom: &[u8]) -> Option<String> {
    let mut out = [0 as c_char; 33];
    let ok = unsafe {
        crate::ffi::rc_hash_generate_from_buffer(out.as_mut_ptr(), console, rom.as_ptr(), rom.len())
    };
    (ok != 0).then(|| unsafe { crate::ffi::text(out.as_ptr()) })
}

pub fn hash_file(console: u32, path: &Path) -> Option<String> {
    let path_str = path.to_str()?;
    let cpath = CString::new(path_str).ok()?;
    let mut out = [0 as c_char; 33];
    let ok = unsafe {
        crate::ffi::rc_hash_generate_from_file(out.as_mut_ptr(), console, cpath.as_ptr())
    };
    (ok != 0).then(|| unsafe { crate::ffi::text(out.as_ptr()) })
}
