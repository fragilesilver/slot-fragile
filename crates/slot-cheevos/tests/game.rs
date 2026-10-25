// Ported from nickclyde/slot-plus@29b8a81, crates/slot-cheevos/tests/game.rs.

use std::path::Path;

use slot_cheevos::game::{console_for, hash, hash_file, GB, GBA, GBC, NDS, NES, PS1, SNES};

fn gb_rom(cgb: u8) -> Vec<u8> {
    let mut rom = vec![0u8; 0x8000];
    rom[0x143] = cgb;
    rom
}

#[test]
fn a_gba_file_is_a_gba_game() {
    assert_eq!(console_for(Path::new("Games/GBA/x.GBA"), &[]), GBA);
}

#[test]
fn consoles_for_all_fragile_systems() {
    assert_eq!(console_for(Path::new("Games/NES/mario.nes"), &[]), NES);
    assert_eq!(console_for(Path::new("Games/SNES/mario.sfc"), &[]), SNES);
    assert_eq!(console_for(Path::new("Games/SNES/mario.smc"), &[]), SNES);
    assert_eq!(console_for(Path::new("Games/PS/crash.chd"), &[]), PS1);
    assert_eq!(console_for(Path::new("Games/PS/crash.cue"), &[]), PS1);
    assert_eq!(console_for(Path::new("Games/PS/track.bin"), &[]), PS1);
    assert_eq!(console_for(Path::new("Games/NDS/pokemon.nds"), &[]), NDS);
}

#[test]
fn the_cgb_flag_picks_game_boy_color() {
    let p = Path::new("Games/GB/x.gb");
    assert_eq!(console_for(p, &gb_rom(0x00)), GB);
    assert_eq!(console_for(p, &gb_rom(0x80)), GBC);
    assert_eq!(console_for(p, &gb_rom(0xc0)), GBC);
    assert_eq!(console_for(Path::new("x.gbc"), &[]), GB);
}

#[test]
fn hashes_file_from_disk_matches_buffer_hash() {
    let rom: Vec<u8> = (0..0x8000u32).map(|i| (i * 7 + 3) as u8).collect();
    let tmp = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(tmp.path(), &rom).unwrap();
    assert_eq!(
        hash_file(GBA, tmp.path()).as_deref(),
        Some("3db4f65017c15021ae233458aa7e0702")
    );
}

#[test]
fn hashes_the_whole_rom_with_md5() {
    let rom: Vec<u8> = (0..0x8000u32).map(|i| (i * 7 + 3) as u8).collect();
    assert_eq!(
        hash(GBA, &rom).as_deref(),
        Some("3db4f65017c15021ae233458aa7e0702")
    );
    assert_eq!(
        hash(GB, &rom).as_deref(),
        Some("3db4f65017c15021ae233458aa7e0702")
    );
    assert_eq!(
        hash(GBC, &rom).as_deref(),
        Some("3db4f65017c15021ae233458aa7e0702")
    );
}
