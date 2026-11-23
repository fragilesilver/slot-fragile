use slot_store::Platform;

#[test]
fn every_platform_has_a_directory() {
    let names: Vec<&str> = Platform::ALL.iter().map(|p| p.dir_name()).collect();
    assert_eq!(
        names,
        vec![
            "GBA",
            "GB",
            "GBC",
            "NES",
            "SNES",
            "PS",
            "NDS",
            "PokeDaisy",
            "PokeDaisy",
            "PokeDaisy",
        ]
    );
}

#[test]
fn every_platform_is_in_the_ring_once() {
    // Fragile: the 7 core systems plus PokeDaisy's 3 hardware variants.
    assert_eq!(
        Platform::ALL,
        [
            Platform::Gba,
            Platform::Gb,
            Platform::Gbc,
            Platform::Nes,
            Platform::Snes,
            Platform::PlayStation,
            Platform::Nds,
            Platform::PdGba,
            Platform::PdGb,
            Platform::PdGbc,
        ]
    );
    for (i, p) in Platform::ALL.iter().enumerate() {
        assert!(
            !Platform::ALL[..i].contains(p),
            "{p:?} is named twice in the ring"
        );
    }
}

#[test]
fn pokedaisy_maps_to_underlying_hardware_and_shared_saves() {
    assert_eq!(Platform::PdGba.hardware(), Platform::Gba);
    assert_eq!(Platform::PdGb.hardware(), Platform::Gb);
    assert_eq!(Platform::PdGbc.hardware(), Platform::Gbc);
    assert_eq!(Platform::PdGba.save_dir_name(), "GBA");
    assert_eq!(Platform::PdGb.save_dir_name(), "GB");
    assert_eq!(Platform::PdGbc.save_dir_name(), "GBC");
    assert_eq!(Platform::PdGbc.shelf(), Platform::PdGba);
    assert!(Platform::PdGba.is_pokedaisy());
    assert!(!Platform::Gba.is_pokedaisy());
}

#[test]
fn each_platform_takes_only_its_own_extensions() {
    assert!(Platform::Gba.accepts(std::path::Path::new("Metroid Fusion.gba")));
    assert!(!Platform::Gba.accepts(std::path::Path::new("Tetris.gb")));
    assert!(Platform::Gb.accepts(std::path::Path::new("Tetris.gb")));
    assert!(Platform::Gb.accepts(std::path::Path::new("Tetris.gbc")));
    assert!(!Platform::Gb.accepts(std::path::Path::new("Metroid Fusion.gba")));
    assert!(Platform::Gba.accepts(std::path::Path::new("Shrek.GBA")));
    assert!(Platform::Nds.accepts(std::path::Path::new("Pokemon Platinum.nds")));
    assert!(Platform::Nds.accepts(std::path::Path::new("Pokemon Platinum.zip")));
    assert!(!Platform::Nds.accepts(std::path::Path::new("Pokemon Platinum.gba")));
}
