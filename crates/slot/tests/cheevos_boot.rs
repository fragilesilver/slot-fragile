// Ported from nickclyde/slot-plus@c218577, crates/slot/tests/cheevos_boot.rs.

#![cfg(feature = "cheevos")]

use std::path::Path;

use slot::cheevos::boot::{frames, FRAMES};

fn card(bios: &[(&str, Vec<u8>)]) -> tempfile::TempDir {
    let root = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(root.path().join("BIOS")).unwrap();
    for (name, bytes) in bios {
        std::fs::write(root.path().join("BIOS").join(name), bytes).unwrap();
    }
    root
}

fn gba_bios() -> Vec<u8> {
    let mut bytes = vec![0u8; 16 * 1024];
    bytes[0] = 0x18;
    bytes
}

#[test]
fn a_fresh_gba_boot_waits_out_the_bios_intro() {
    let root = card(&[("gba_bios.bin", gba_bios())]);
    assert_eq!(
        frames(root.path(), Path::new("Games/GBA/Zero.gba"), false),
        FRAMES
    );
}

#[test]
fn a_resumed_game_never_shows_the_bios() {
    let root = card(&[("gba_bios.bin", gba_bios()), ("gbc_bios.bin", vec![1])]);
    assert_eq!(
        frames(root.path(), Path::new("Games/GBA/Zero.gba"), true),
        0
    );
    assert_eq!(
        frames(root.path(), Path::new("Games/GBC/Oracle.gbc"), true),
        0
    );
}

#[test]
fn without_a_bios_there_is_no_intro_to_wait_for() {
    let root = card(&[]);
    assert_eq!(
        frames(root.path(), Path::new("Games/GBA/Zero.gba"), false),
        0
    );
    assert_eq!(
        frames(root.path(), Path::new("Games/GB/Tetris.gb"), false),
        0
    );
}

#[test]
fn a_bad_gba_bios_is_skipped_by_the_core_so_nothing_waits() {
    let root = card(&[("gba_bios.bin", vec![0u8; 100])]);
    assert_eq!(
        frames(root.path(), Path::new("Games/GBA/Zero.gba"), false),
        0
    );
}

#[test]
fn a_game_boy_boot_rom_holds_gb_and_gbc_games_but_not_gba() {
    let root = card(&[("gb_bios.bin", vec![1; 256])]);
    assert_eq!(
        frames(root.path(), Path::new("Games/GB/Tetris.gb"), false),
        FRAMES
    );
    assert_eq!(
        frames(root.path(), Path::new("Games/GBC/Oracle.gbc"), false),
        FRAMES
    );
    assert_eq!(
        frames(root.path(), Path::new("Games/GBA/Zero.gba"), false),
        0
    );
}
