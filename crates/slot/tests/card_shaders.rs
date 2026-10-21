mod common;

use common::{app_playing_in, boot, tmp_root_with_carts, write_gb_cart};
use slot::app::App;
use slot_input::{Action, Btn};
use slot_store::{list_shaders, read_extras, shader_path, Platform};
use slot_ui::QuickRow;

fn shaders(d: &tempfile::TempDir, names: &[&str]) {
    for name in names {
        std::fs::write(d.path().join("Shaders").join(name), "// shader").unwrap();
    }
}

#[test]
fn the_card_lists_its_glsl_files_by_name_and_finds_any_spelling() {
    let d = tmp_root_with_carts(&["Emerald"]);
    shaders(
        &d,
        &[
            "zfast.glsl",
            "Blend.GLSL",
            "._zfast.glsl",
            "notes.txt",
            "agb.glsl",
        ],
    );
    assert_eq!(list_shaders(d.path()), ["agb", "Blend", "zfast"]);
    assert!(shader_path(d.path(), "Blend").is_some_and(|p| p.ends_with("Blend.GLSL")));
    assert_eq!(shader_path(d.path(), "missing"), None);
}

#[test]
fn the_card_shader_row_steps_through_off_and_the_files_and_saves() {
    let d = tmp_root_with_carts(&["Emerald", "Fusion"]);
    shaders(&d, &["Ghosting.glsl", "Blend Grid.glsl"]);
    let mut a = boot(d.path());
    a.apply(Action::QuickMenu);
    let walk = |a: &mut App, row: QuickRow| {
        for _ in 0..row.position(&a.hidden_rows()) {
            a.apply(Action::GbaDown(Btn::Down));
        }
    };
    walk(&mut a, QuickRow::Screen);
    a.apply(Action::GbaDown(Btn::A));
    walk(&mut a, QuickRow::ScreenGba);
    a.apply(Action::GbaDown(Btn::A));
    walk(&mut a, QuickRow::CardShaderGba);
    assert_eq!(a.quick_menu(), Some(QuickRow::CardShaderGba));
    assert_eq!(
        a.named_value(QuickRow::CardShaderGba).as_deref(),
        Some("Off")
    );
    let mut seen = Vec::new();
    for _ in 0..3 {
        a.apply(Action::GbaDown(Btn::Right));
        seen.push(a.named_value(QuickRow::CardShaderGba).unwrap());
    }
    assert_eq!(seen, ["Blend Grid", "Ghosting", "Off"]);
    a.apply(Action::GbaDown(Btn::Left));
    assert_eq!(
        read_extras(d.path()).get("card_shader_gba"),
        Some("Ghosting")
    );
    assert_eq!(
        a.card_shader_for(Platform::Gb),
        None,
        "the GB page followed the GBA one"
    );
}

#[test]
fn the_playing_console_decides_the_card_shader() {
    let d = tmp_root_with_carts(&["Emerald"]);
    write_gb_cart(&d, "Tetris", "TETRIS");
    let mut e = read_extras(d.path());
    e.set("card_shader_gb", Some("Ghosting"));
    slot_store::write_extras(d.path(), &e).unwrap();
    assert_eq!(
        app_playing_in(d.path(), "Tetris").card_shader().as_deref(),
        Some("Ghosting")
    );
    assert_eq!(app_playing_in(d.path(), "Emerald").card_shader(), None);
}
