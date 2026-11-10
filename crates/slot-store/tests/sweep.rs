use std::fs;
use tempfile::tempdir;

use slot_store::{clean_filename, sweep_names, Platform};

#[test]
fn clean_filename_detects_and_fixes_trailing_dots_and_spaces() {
    assert_eq!(clean_filename("Pokemon .gba"), Some("Pokemon.gba".into()));
    assert_eq!(clean_filename("Pokemon..gba"), Some("Pokemon.gba".into()));
    assert_eq!(clean_filename(" Pokemon.gba"), Some("Pokemon.gba".into()));
    assert_eq!(clean_filename("Pokemon.gba."), Some("Pokemon.gba".into()));
    assert_eq!(clean_filename("Pokemon . gba"), Some("Pokemon. gba".into()));
    assert_eq!(clean_filename("Pokemon.gba"), None);
}

#[test]
fn sweep_names_renames_roms_and_matching_sidecars() {
    let dir = tempdir().unwrap();
    let root = dir.path();

    let gba_games = root.join("Games").join(Platform::Gba.dir_name());
    fs::create_dir_all(&gba_games).unwrap();

    let gba_saves = root.join("Saves").join(Platform::Gba.dir_name());
    fs::create_dir_all(&gba_saves).unwrap();

    let gba_labels = root.join("Labels").join(Platform::Gba.dir_name());
    fs::create_dir_all(&gba_labels).unwrap();

    // Create a messy ROM and its save and label
    let bad_rom = gba_games.join("Emerald .gba");
    fs::write(&bad_rom, b"ROMDATA").unwrap();

    let bad_save = gba_saves.join("Emerald .sav");
    fs::write(&bad_save, b"SAVEDATA").unwrap();

    let bad_label = gba_labels.join("Emerald .png");
    fs::write(&bad_label, b"PNGDATA").unwrap();

    let swept = sweep_names(root).unwrap();
    assert_eq!(swept, 1);

    // Bad files should now be gone, clean files should exist
    assert!(!bad_rom.exists());
    assert!(!bad_save.exists());
    assert!(!bad_label.exists());

    let clean_rom = gba_games.join("Emerald.gba");
    let clean_save = gba_saves.join("Emerald.sav");
    let clean_label = gba_labels.join("Emerald.png");

    assert!(clean_rom.exists());
    assert!(clean_save.exists());
    assert!(clean_label.exists());
    assert_eq!(fs::read(&clean_rom).unwrap(), b"ROMDATA");
    assert_eq!(fs::read(&clean_save).unwrap(), b"SAVEDATA");
    assert_eq!(fs::read(&clean_label).unwrap(), b"PNGDATA");
}
