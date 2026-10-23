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

/// What the shelf reads ahead of a game, and how much of it: all of a cart, but for a game on
/// discs only the first disc's start, which is what booting it reads; an .m3u's first disc.
pub fn prefetched(rom: &Path) -> (std::path::PathBuf, u64) {
    const DISC_START: u64 = 32 << 20;
    let is = |ext: &str| {
        rom.extension()
            .and_then(|e| e.to_str())
            .is_some_and(|e| e.eq_ignore_ascii_case(ext))
    };
    if is("m3u") {
        if let Some(first) = slot_store::discs::members(rom).into_iter().next() {
            return (first, DISC_START);
        }
    }
    if ["cue", "chd", "pbp", "bin", "iso"].into_iter().any(is) {
        return (rom.to_path_buf(), DISC_START);
    }
    (rom.to_path_buf(), u64::MAX)
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
    fn an_m3u_reads_ahead_its_first_disc_and_a_disc_only_its_start() {
        let d = tempfile::tempdir().unwrap();
        let m3u = d.path().join("Game.m3u");
        std::fs::write(
            &m3u,
            ".hidden/Game (Disc 1).chd\n.hidden/Game (Disc 2).chd\n",
        )
        .unwrap();
        let (path, most) = prefetched(&m3u);
        assert_eq!(path, d.path().join(".hidden/Game (Disc 1).chd"));
        assert_eq!(most, 32 << 20);
        let gba = d.path().join("Emerald.gba");
        assert_eq!(prefetched(&gba), (gba.clone(), u64::MAX));
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
