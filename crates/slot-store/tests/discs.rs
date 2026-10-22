// Fragile: the shelf's view of games on discs (Phase 12).

mod common;

use common::tmp_root;
use slot_store::{scan, Platform};
use tempfile::TempDir;

fn put(d: &TempDir, rel: &str, body: &str) {
    let path = d.path().join("Games/PS").join(rel);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, body).unwrap();
}

fn stems(d: &TempDir) -> Vec<String> {
    scan(d.path())
        .unwrap()
        .into_iter()
        .filter(|c| c.platform == Platform::PlayStation)
        .map(|c| c.stem)
        .collect()
}

#[test]
fn a_game_in_a_folder_of_its_own_is_on_the_shelf() {
    let d = tmp_root();
    put(
        &d,
        "Crash Bandicoot/Crash Bandicoot.cue",
        "FILE \"Crash Bandicoot.bin\" BINARY",
    );
    put(&d, "Crash Bandicoot/Crash Bandicoot.bin", "");
    put(&d, "Spyro.chd", "");
    assert_eq!(stems(&d), ["Crash Bandicoot", "Spyro"]);
}

#[test]
fn an_m3u_stands_for_the_discs_it_lists() {
    let d = tmp_root();
    put(
        &d,
        "Final Fantasy VII/Final Fantasy VII.m3u",
        "Final Fantasy VII (Disc 1).chd\nFinal Fantasy VII (Disc 2).CHD\nfinal fantasy vii (disc 3).chd\n",
    );
    for n in 1..=3 {
        put(
            &d,
            &format!("Final Fantasy VII/Final Fantasy VII (Disc {n}).chd"),
            "",
        );
    }
    put(
        &d,
        "Metal Gear Solid.m3u",
        "MGS/Metal Gear Solid (Disc 1).cue\nMGS/Metal Gear Solid (Disc 2).cue",
    );
    put(&d, "MGS/Metal Gear Solid (Disc 1).cue", "");
    put(&d, "MGS/Metal Gear Solid (Disc 2).cue", "");
    assert_eq!(stems(&d), ["Final Fantasy VII", "Metal Gear Solid"]);
}

#[test]
fn discs_no_m3u_lists_stay_on_the_shelf() {
    let d = tmp_root();
    put(&d, "Parasite Eve/Parasite Eve (Disc 1).cue", "");
    put(&d, "Parasite Eve/Parasite Eve (Disc 2).cue", "");
    assert_eq!(
        stems(&d),
        ["Parasite Eve (Disc 1)", "Parasite Eve (Disc 2)"]
    );
}

#[test]
fn only_disc_systems_look_in_folders() {
    let d = tmp_root();
    let deep = d.path().join("Games/GBA/Deep/Advance Wars.gba");
    std::fs::create_dir_all(deep.parent().unwrap()).unwrap();
    std::fs::write(&deep, vec![0u8; 0x100]).unwrap();
    assert!(scan(d.path()).unwrap().is_empty());
}
