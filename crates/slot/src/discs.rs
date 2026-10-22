//! Fragile: a game on more than one disc (Phase 12), and the BIOS a disc system boots from.

use std::path::Path;

use slot_store::Platform;

/// A disc in the drive (from 0) and how many there are, in one number the emulator thread can
/// share: the disc high, the count low, 0 for a game whose core swaps none.
pub fn pack(discs: Option<(u32, u32)>) -> u16 {
    match discs {
        Some((at, count)) if count > 0 => (at.min(255) as u16) << 8 | count.min(255) as u16,
        _ => 0,
    }
}

pub fn unpack(packed: u16) -> Option<(u8, u8)> {
    let count = (packed & 0xff) as u8;
    (count > 0).then_some(((packed >> 8) as u8, count))
}

/// Whether one of the files the platform's core boots best from is in the BIOS folder, whatever
/// its case. None for a platform whose core needs none.
pub fn has_bios(root: &Path, platform: Platform) -> Option<bool> {
    let wanted = platform.spec().bios;
    if wanted.is_empty() {
        return None;
    }
    let present = slot_store::listing(&crate::root::bios_dir(root));
    Some(
        wanted
            .iter()
            .any(|name| present.contains_key(&name.to_lowercase())),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_disc_and_its_count_survive_the_trip() {
        assert_eq!(unpack(pack(Some((2, 3)))), Some((2, 3)));
        assert_eq!(unpack(pack(Some((0, 1)))), Some((0, 1)));
        assert_eq!(unpack(pack(None)), None);
        assert_eq!(unpack(pack(Some((0, 0)))), None);
    }

    #[test]
    fn a_bios_is_found_whatever_its_case() {
        let d = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(d.path().join("BIOS")).unwrap();
        assert_eq!(has_bios(d.path(), Platform::PlayStation), Some(false));
        std::fs::write(d.path().join("BIOS/SCPH5501.BIN"), b"").unwrap();
        assert_eq!(has_bios(d.path(), Platform::PlayStation), Some(true));
        assert_eq!(has_bios(d.path(), Platform::Snes), None);
    }
}
