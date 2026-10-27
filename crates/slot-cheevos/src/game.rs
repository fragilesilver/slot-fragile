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
        Some("zip") => {
            if let Ok(file) = std::fs::File::open(rom_path) {
                if let Ok(mut archive) = zip::ZipArchive::new(file) {
                    for i in 0..archive.len() {
                        if let Ok(entry) = archive.by_index(i) {
                            if entry.is_dir() {
                                continue;
                            }
                            let Ok(entry_name) = entry.name() else {
                                continue;
                            };
                            let name = entry_name.to_ascii_lowercase();
                            if name.ends_with(".nds") || name.ends_with(".srl") || name.ends_with(".dsi") {
                                return NDS;
                            } else if name.ends_with(".gba") {
                                return GBA;
                            } else if name.ends_with(".gbc") {
                                return GBC;
                            } else if name.ends_with(".gb") {
                                return GB;
                            } else if name.ends_with(".nes") || name.ends_with(".unf") || name.ends_with(".fds") {
                                return NES;
                            } else if name.ends_with(".sfc") || name.ends_with(".smc") {
                                return SNES;
                            } else if name.ends_with(".cue") || name.ends_with(".chd") || name.ends_with(".pbp") {
                                return PS1;
                            }
                        }
                    }
                }
            }
            if rom_path.iter().any(|c| c == "NDS") {
                NDS
            } else if rom_path.iter().any(|c| c == "PS" || c == "PS1") {
                PS1
            } else if rom_path.iter().any(|c| c == "NES") {
                NES
            } else if rom_path.iter().any(|c| c == "SNES") {
                SNES
            } else if rom_path.iter().any(|c| c == "GBA") {
                GBA
            } else {
                GB
            }
        }
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

fn is_zip_file(path: &Path) -> bool {
    if path
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case("zip"))
    {
        return true;
    }
    if let Ok(mut f) = std::fs::File::open(path) {
        let mut magic = [0u8; 4];
        if std::io::Read::read_exact(&mut f, &mut magic).is_ok() {
            return magic == [0x50, 0x4B, 0x03, 0x04];
        }
    }
    false
}

fn extract_rom_from_zip(path: &Path, console: u32) -> Option<Vec<u8>> {
    let file = std::fs::File::open(path).ok()?;
    let mut archive = zip::ZipArchive::new(file).ok()?;

    let expected_exts: &[&str] = match console {
        NDS => &["nds", "srl", "dsi"],
        GBA => &["gba"],
        GB => &["gb"],
        GBC => &["gbc", "gb"],
        NES => &["nes", "unf", "unif", "fds"],
        SNES => &["sfc", "smc", "swc", "fig", "bs"],
        PS1 => &["cue", "bin", "chd", "pbp", "iso", "img"],
        _ => &[],
    };

    let mut chosen_idx = None;
    for i in 0..archive.len() {
        if let Ok(entry) = archive.by_index(i) {
            if entry.is_dir() {
                continue;
            }
            let Ok(entry_name) = entry.name() else {
                continue;
            };
            let name = entry_name.to_ascii_lowercase();
            if expected_exts.iter().any(|ext| name.ends_with(ext)) {
                chosen_idx = Some(i);
                break;
            }
        }
    }

    if chosen_idx.is_none() {
        let mut max_size = 0;
        for i in 0..archive.len() {
            if let Ok(entry) = archive.by_index(i) {
                if !entry.is_dir() && entry.size() > max_size {
                    max_size = entry.size();
                    chosen_idx = Some(i);
                }
            }
        }
    }

    let idx = chosen_idx?;
    let mut entry = archive.by_index(idx).ok()?;
    let mut bytes = Vec::with_capacity(entry.size() as usize);
    std::io::Read::read_to_end(&mut entry, &mut bytes).ok()?;
    Some(bytes)
}

pub fn hash_file(console: u32, path: &Path) -> Option<String> {
    crate::cdreader::init();
    if is_zip_file(path) {
        if let Some(bytes) = extract_rom_from_zip(path, console) {
            return hash(console, &bytes);
        }
    }
    let path_str = path.to_str()?;
    let cpath = CString::new(path_str).ok()?;
    let mut out = [0 as c_char; 33];
    let ok = unsafe {
        crate::ffi::rc_hash_generate_from_file(out.as_mut_ptr(), console, cpath.as_ptr())
    };
    (ok != 0).then(|| unsafe { crate::ffi::text(out.as_ptr()) })
}
