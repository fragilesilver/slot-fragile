// Ported from nickclyde/slot-plus@02c5890, crates/slot-store/tests/collections.rs.

use slot_store::collections::{hits, parse_entries, read_lists, Index, LISTS_DIR};
use slot_store::{Cart, Platform};
use std::path::PathBuf;

fn cart(platform: Platform, stem: &str) -> Cart {
    Cart {
        platform,
        stem: stem.to_string(),
        rom: PathBuf::from(format!("Games/{}/{stem}.gba", platform.dir_name())),
        label: None,
        title: String::new(),
        code: String::new(),
        shell: None,
    }
}

fn card() -> Vec<Cart> {
    vec![
        cart(Platform::Gba, "Metroid Fusion (USA)"),
        cart(Platform::Gba, "Pokemon - Emerald Version (USA, Europe)"),
        cart(Platform::Gba, "Tetris"),
        cart(Platform::Gb, "Tetris"),
        cart(Platform::Gbc, "Mr. Driller"),
    ]
}

fn key(platform: Platform, stem: &str) -> Option<(Platform, String)> {
    Some((platform, stem.to_string()))
}

#[test]
fn a_list_line_finds_its_cart_by_platform_and_stem_by_bare_stem_or_by_rom_name() {
    let index = Index::new(&card());
    let fusion = key(Platform::Gba, "Metroid Fusion (USA)");
    assert_eq!(index.resolve("GBA/Metroid Fusion (USA)"), fusion);
    assert_eq!(index.resolve("Metroid Fusion (USA)"), fusion);
    assert_eq!(index.resolve("Metroid Fusion (USA).gba"), fusion);
    assert_eq!(index.resolve("Games/GBA/Metroid Fusion (USA).gba"), fusion);
    assert_eq!(
        index.resolve(r"E:\Games\GBA\Metroid Fusion (USA).gba"),
        fusion
    );
    assert_eq!(
        index.resolve("Mr. Driller"),
        key(Platform::Gbc, "Mr. Driller")
    );
}

#[test]
fn list_lines_match_whatever_their_case() {
    let index = Index::new(&card());
    assert_eq!(
        index.resolve("gba/metroid fusion (usa).GBA"),
        key(Platform::Gba, "Metroid Fusion (USA)")
    );
}

#[test]
fn a_line_without_its_region_tag_finds_the_only_cart_it_could_mean() {
    let index = Index::new(&card());
    assert_eq!(
        index.resolve("Metroid Fusion"),
        key(Platform::Gba, "Metroid Fusion (USA)")
    );
    assert_eq!(
        index.resolve("pokemon emerald version"),
        key(Platform::Gba, "Pokemon - Emerald Version (USA, Europe)")
    );
}

#[test]
fn a_bare_stem_on_two_platforms_picks_the_game_boy_advance_one() {
    let index = Index::new(&card());
    assert_eq!(index.resolve("Tetris"), key(Platform::Gba, "Tetris"));
    assert_eq!(index.resolve("GB/Tetris"), key(Platform::Gb, "Tetris"));
    assert_eq!(index.resolve("Tetris.gb"), key(Platform::Gba, "Tetris"));
}

#[test]
fn comments_blank_lines_and_a_byte_order_mark_are_ignored() {
    let index = Index::new(&card());
    assert_eq!(index.resolve("# Tetris"), None);
    assert_eq!(index.resolve("   "), None);
    assert_eq!(
        index.resolve("\u{feff}Tetris"),
        key(Platform::Gba, "Tetris")
    );
    let entries = parse_entries("\u{feff}GB/Tetris\n\n# mine\n", &index);
    assert_eq!(hits(&entries), vec![(Platform::Gb, "Tetris".to_string())]);
}

#[test]
fn a_line_naming_no_cart_on_the_card_is_skipped() {
    let index = Index::new(&card());
    let entries = parse_entries("# mine\nZelda\ngba/tetris\n", &index);
    assert_eq!(hits(&entries), vec![(Platform::Gba, "Tetris".to_string())]);
}

#[test]
fn a_list_keeps_its_written_order_and_drops_repeats() {
    let index = Index::new(&card());
    let entries = parse_entries("Tetris\nMr. Driller\nGBA/Tetris\nGB/Tetris\n", &index);
    assert_eq!(
        hits(&entries),
        vec![
            (Platform::Gba, "Tetris".to_string()),
            (Platform::Gbc, "Mr. Driller".to_string()),
            (Platform::Gb, "Tetris".to_string()),
        ]
    );
}

#[test]
fn each_text_file_in_the_lists_folder_is_a_list_named_after_it_in_name_order() {
    let root = tempfile::tempdir().unwrap();
    let dir = root.path().join(LISTS_DIR);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("Zelda.txt"), "Tetris\n").unwrap();
    std::fs::write(dir.join("couch co-op.TXT"), "Mr. Driller\n").unwrap();
    std::fs::write(dir.join(".hidden.txt"), "Tetris\n").unwrap();
    std::fs::write(dir.join("notes.md"), "Tetris\n").unwrap();
    let lists = read_lists(root.path(), &Index::new(&card()));
    let names: Vec<&str> = lists.iter().map(|l| l.name.as_str()).collect();
    assert_eq!(names, ["couch co-op", "Zelda"]);
    assert_eq!(lists[1].carts, vec![(Platform::Gba, "Tetris".to_string())]);
}

// --- extras: writing lists ---

use slot_store::collections::{add_to_list, in_list, list_name, list_path, remove_from_list};

#[test]
fn a_cart_added_to_a_list_is_found_again_and_taken_out_with_the_lines_that_name_it() {
    let root = tempfile::tempdir().unwrap();
    let index = Index::new(&card());
    let tetris = (Platform::Gb, "Tetris".to_string());
    let driller = (Platform::Gbc, "Mr. Driller".to_string());
    std::fs::create_dir_all(root.path().join(LISTS_DIR)).unwrap();
    std::fs::write(list_path(root.path(), "Mine"), "# kept\nMr. Driller").unwrap();
    add_to_list(root.path(), "Mine", &tetris).unwrap();
    assert!(in_list(root.path(), "Mine", &tetris, &index));
    assert_eq!(
        std::fs::read_to_string(list_path(root.path(), "Mine")).unwrap(),
        "# kept\nMr. Driller\nGB/Tetris\n"
    );
    remove_from_list(root.path(), "Mine", &driller, &index).unwrap();
    assert!(!in_list(root.path(), "Mine", &driller, &index));
    assert_eq!(
        std::fs::read_to_string(list_path(root.path(), "Mine")).unwrap(),
        "# kept\nGB/Tetris\n"
    );
}

#[test]
fn adding_to_a_list_that_is_not_there_makes_it() {
    let root = tempfile::tempdir().unwrap();
    let tetris = (Platform::Gba, "Tetris".to_string());
    add_to_list(root.path(), "New", &tetris).unwrap();
    let lists = read_lists(root.path(), &Index::new(&card()));
    assert_eq!(lists.len(), 1);
    assert_eq!(lists[0].name, "New");
    assert_eq!(lists[0].carts, vec![tetris]);
}

#[test]
fn a_typed_list_name_loses_what_a_card_cannot_name_a_file() {
    assert_eq!(
        list_name("  Couch: co-op?  ").as_deref(),
        Some("Couch co-op")
    );
    assert_eq!(list_name("../a/b").as_deref(), Some("ab"));
    assert_eq!(list_name(" ** "), None);
}

use slot_store::collections::{delete_list, rename_list};

#[test]
fn a_list_renames_and_keeps_its_lines_and_will_not_take_another_lists_name() {
    let root = tempfile::tempdir().unwrap();
    let tetris = (Platform::Gb, "Tetris".to_string());
    add_to_list(root.path(), "Couch", &tetris).unwrap();
    add_to_list(root.path(), "Arcade", &tetris).unwrap();
    rename_list(root.path(), "Couch", "Sofa").unwrap();
    assert!(!list_path(root.path(), "Couch").exists());
    assert_eq!(
        std::fs::read_to_string(list_path(root.path(), "Sofa")).unwrap(),
        "GB/Tetris\n"
    );
    assert!(rename_list(root.path(), "Sofa", "Arcade").is_err());
    rename_list(root.path(), "Sofa", "SOFA").unwrap();
    let names: Vec<String> = read_lists(root.path(), &Index::new(&card()))
        .into_iter()
        .map(|l| l.name)
        .collect();
    assert_eq!(names, ["Arcade", "SOFA"]);
}

#[test]
fn deleting_a_list_takes_only_its_file() {
    let root = tempfile::tempdir().unwrap();
    add_to_list(root.path(), "Couch", &(Platform::Gb, "Tetris".to_string())).unwrap();
    delete_list(root.path(), "Couch").unwrap();
    assert!(read_lists(root.path(), &Index::new(&card())).is_empty());
}
