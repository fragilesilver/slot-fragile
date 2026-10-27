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

fn create_dummy_nds_rom() -> Vec<u8> {
    let mut rom = vec![0u8; 0x1000];
    // Game title
    rom[0..12].copy_from_slice(b"TESTNDSGAME\0");
    // ARM9 offset = 0x200, size = 0x400
    rom[0x20..0x24].copy_from_slice(&0x200u32.to_le_bytes());
    rom[0x2C..0x30].copy_from_slice(&0x400u32.to_le_bytes());
    // ARM7 offset = 0x600, size = 0x200
    rom[0x30..0x34].copy_from_slice(&0x600u32.to_le_bytes());
    rom[0x3C..0x40].copy_from_slice(&0x200u32.to_le_bytes());
    // Fill ARM9 and ARM7 code regions with deterministic pattern
    for (i, b) in rom[0x200..0x600].iter_mut().enumerate() {
        *b = (i * 13 + 7) as u8;
    }
    for (i, b) in rom[0x600..0x800].iter_mut().enumerate() {
        *b = (i * 17 + 11) as u8;
    }
    rom
}

#[test]
fn hashes_nds_rom_and_zipped_nds_rom_identically() {
    let rom = create_dummy_nds_rom();
    let dir = tempfile::tempdir().unwrap();
    let nds_path = dir.path().join("game.nds");
    std::fs::write(&nds_path, &rom).unwrap();

    let raw_hash = hash_file(NDS, &nds_path);
    assert!(raw_hash.is_some(), "raw NDS hash failed");
    let buf_hash = hash(NDS, &rom);
    assert_eq!(raw_hash, buf_hash, "file hash should match buffer hash");

    // Package into .zip
    let zip_path = dir.path().join("game.zip");
    let zip_file = std::fs::File::create(&zip_path).unwrap();
    let mut zip_writer = zip::ZipWriter::new(zip_file);
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);
    use std::io::Write;
    zip_writer.start_file("game.nds", options).unwrap();
    zip_writer.write_all(&rom).unwrap();
    zip_writer.finish().unwrap();

    let zip_hash = hash_file(NDS, &zip_path);
    assert_eq!(zip_hash, raw_hash, "zipped NDS hash must match raw NDS ROM hash");

    // Also verify console_for resolves NDS from inside the zip
    assert_eq!(console_for(&zip_path, &[]), NDS);
}

#[test]
fn hashes_ps1_disc_image() {
    let dir = tempfile::tempdir().unwrap();
    let bin_path = dir.path().join("game.bin");
    let cue_path = dir.path().join("game.cue");

    let mut iso = vec![0u8; 20 * 2048];
    // Sector 16: PVD
    let pvd = 16 * 2048;
    iso[pvd] = 1;
    iso[pvd + 1..pvd + 6].copy_from_slice(b"CD001");
    iso[pvd + 6] = 1;
    iso[pvd + 128..pvd + 130].copy_from_slice(&2048u16.to_le_bytes());
    // Root Directory Record
    let root_rec = pvd + 156;
    iso[root_rec] = 34;
    iso[root_rec + 2..root_rec + 6].copy_from_slice(&17u32.to_le_bytes());
    iso[root_rec + 10..root_rec + 14].copy_from_slice(&2048u32.to_le_bytes());

    // Sector 17: Root Directory contents
    let d = 17 * 2048;
    // Entry 1: SYSTEM.CNF;1 -> Sector 18, size 50
    let entry1 = d;
    let name1 = b"SYSTEM.CNF;1";
    let rec_len1 = 33 + name1.len() + if name1.len() % 2 == 0 { 1 } else { 0 };
    iso[entry1] = rec_len1 as u8;
    iso[entry1 + 2..entry1 + 6].copy_from_slice(&18u32.to_le_bytes());
    iso[entry1 + 10..entry1 + 14].copy_from_slice(&50u32.to_le_bytes());
    iso[entry1 + 32] = name1.len() as u8;
    iso[entry1 + 33..entry1 + 33 + name1.len()].copy_from_slice(name1);

    // Entry 2: SLUS_001.23;1 -> Sector 19, size 2048
    let entry2 = entry1 + rec_len1;
    let name2 = b"SLUS_001.23;1";
    let rec_len2 = 33 + name2.len() + if name2.len() % 2 == 0 { 1 } else { 0 };
    iso[entry2] = rec_len2 as u8;
    iso[entry2 + 2..entry2 + 6].copy_from_slice(&19u32.to_le_bytes());
    iso[entry2 + 10..entry2 + 14].copy_from_slice(&2048u32.to_le_bytes());
    iso[entry2 + 32] = name2.len() as u8;
    iso[entry2 + 33..entry2 + 33 + name2.len()].copy_from_slice(name2);

    // Sector 18: SYSTEM.CNF
    let cnf = 18 * 2048;
    let cnf_content = b"BOOT = cdrom:\\SLUS_001.23;1\r\n";
    iso[cnf..cnf + cnf_content.len()].copy_from_slice(cnf_content);

    // Sector 19: SLUS_001.23 executable
    let exe = 19 * 2048;
    iso[exe..exe + 8].copy_from_slice(b"PS-X EXE");
    iso[exe + 28..exe + 32].copy_from_slice(&0u32.to_le_bytes());

    std::fs::write(&bin_path, &iso).unwrap();
    let cue_content = format!(
        "FILE \"{}\" BINARY\n  TRACK 01 MODE1/2048\n    INDEX 01 00:00:00\n",
        bin_path.file_name().unwrap().to_str().unwrap()
    );
    std::fs::write(&cue_path, cue_content).unwrap();

    let ps1_hash = hash_file(PS1, &cue_path);
    assert!(ps1_hash.is_some(), "PS1 cue/bin hash should succeed");

    // Also test .m3u playlist referencing the cue
    let m3u_path = dir.path().join("game.m3u");
    std::fs::write(&m3u_path, "game.cue\n").unwrap();
    let m3u_hash = hash_file(PS1, &m3u_path);
    assert_eq!(m3u_hash, ps1_hash, "m3u playlist hash must match disc hash");
}
