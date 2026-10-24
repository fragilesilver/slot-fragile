mod common;

use common::{app_playing_in, tmp_root_with_carts};
use slot::app::App;
use slot_input::{Action, Btn};
use slot_store::{load_shader_params, read_extras, write_extras, Platform};
use slot_ui::QuickRow;

const TEST_PARAM_SHADER: &str = r#"
#pragma parameter BLEND "Frame blend" 0.5 0.0 1.0 0.05
#pragma parameter GRID "Grid strength" 0.25 0.0 1.0 0.05

#if defined(VERTEX)
attribute vec4 VertexCoord;
attribute vec4 TexCoord;
varying vec4 TEX0;
uniform mat4 MVPMatrix;
void main() {
    gl_Position = MVPMatrix * VertexCoord;
    TEX0 = TexCoord;
}
#elif defined(FRAGMENT)
#ifdef GL_ES
precision mediump float;
#endif
uniform sampler2D Texture;
varying vec4 TEX0;
#ifdef PARAMETER_UNIFORM
uniform float BLEND;
uniform float GRID;
#else
#define BLEND 0.5
#define GRID 0.25
#endif
void main() {
    gl_FragColor = texture2D(Texture, TEX0.xy) * BLEND * (1.0 - GRID);
}
#endif
"#;

const TEST_PLAIN_SHADER: &str = r#"
#if defined(VERTEX)
attribute vec4 VertexCoord;
void main() { gl_Position = VertexCoord; }
#elif defined(FRAGMENT)
void main() { gl_FragColor = vec4(1.0); }
#endif
"#;

fn press(a: &mut App, btn: Btn) {
    a.apply(Action::GbaDown(btn));
    a.apply(Action::GbaUp(btn));
}

fn go_to(a: &mut App, row: QuickRow) {
    a.apply(Action::QuickMenu);
    let path: &[QuickRow] = match row.page() {
        p if p == QuickRow::PLAY => &[],
        p if p == QuickRow::SETTINGS => &[QuickRow::Settings],
        p if p == QuickRow::PLAY_SCREEN => &[QuickRow::Settings, QuickRow::PlayScreen],
        _ => &[QuickRow::Settings, QuickRow::PlayGameplay],
    };
    for step in path.iter().copied().chain([row]) {
        for _ in 0..12 {
            if a.in_game_menu() == Some(step) {
                break;
            }
            press(a, Btn::Down);
        }
        assert_eq!(a.in_game_menu(), Some(step), "never reached {step:?}");
        if step != row {
            press(a, Btn::A);
        }
    }
}

#[test]
fn shader_settings_row_hidden_when_shader_is_off_or_has_no_parameters() {
    let d = tmp_root_with_carts(&["Emerald"]);
    std::fs::create_dir_all(d.path().join("Shaders")).unwrap();
    std::fs::write(d.path().join("Shaders/Plain.glsl"), TEST_PLAIN_SHADER).unwrap();
    std::fs::write(d.path().join("Shaders/Tuned.glsl"), TEST_PARAM_SHADER).unwrap();

    let mut a = app_playing_in(d.path(), "Emerald");
    go_to(&mut a, QuickRow::CardShader);
    assert_eq!(a.in_game_menu(), Some(QuickRow::CardShader));

    // 1. With Card Shader Off: ShaderSettings is hidden
    assert!(a.in_game_hidden().contains(&QuickRow::ShaderSettings));

    // 2. Step Card Shader to "Plain" (no params): ShaderSettings remains hidden
    assert_eq!(a.named_value(QuickRow::CardShader).as_deref(), Some("Off"));
    press(&mut a, Btn::Right);
    assert_eq!(
        a.named_value(QuickRow::CardShader).as_deref(),
        Some("Plain")
    );
    assert!(a.in_game_hidden().contains(&QuickRow::ShaderSettings));

    // 3. Step Card Shader to "Tuned" (has params): ShaderSettings becomes visible!
    press(&mut a, Btn::Right);
    assert_eq!(
        a.named_value(QuickRow::CardShader).as_deref(),
        Some("Tuned")
    );
    assert!(!a.in_game_hidden().contains(&QuickRow::ShaderSettings));

    // 4. Step Card Shader back to "Off": ShaderSettings hidden again
    press(&mut a, Btn::Right);
    assert_eq!(a.named_value(QuickRow::CardShader).as_deref(), Some("Off"));
    assert!(a.in_game_hidden().contains(&QuickRow::ShaderSettings));
}

#[test]
fn shader_settings_overlay_navigation_tuning_and_reset() {
    let d = tmp_root_with_carts(&["Emerald"]);
    std::fs::create_dir_all(d.path().join("Shaders")).unwrap();
    std::fs::write(d.path().join("Shaders/Tuned.glsl"), TEST_PARAM_SHADER).unwrap();

    let mut e = read_extras(d.path());
    e.set("card_shader_gba", Some("Tuned"));
    write_extras(d.path(), &e).unwrap();

    let mut a = app_playing_in(d.path(), "Emerald");
    go_to(&mut a, QuickRow::ShaderSettings);
    assert_eq!(a.in_game_menu(), Some(QuickRow::ShaderSettings));
    press(&mut a, Btn::A);

    // Overlay is open
    let page = a.overlay_page().expect("overlay page is open");
    assert_eq!(page.cursor(), 0);
    let rows = page.shown();
    assert_eq!(rows.len(), 3);
    assert_eq!(rows[0].label, "Frame blend");
    assert_eq!(rows[0].value.as_deref(), Some("0.50"));
    assert_eq!(rows[1].label, "Grid strength");
    assert_eq!(rows[1].value.as_deref(), Some("0.25"));
    assert_eq!(rows[2].label, "Reset");
    assert_eq!(rows[2].value, None);

    // 1. Tune "Frame blend" right by 1 step (0.05 -> 0.55)
    press(&mut a, Btn::Right);
    let page = a.overlay_page().unwrap();
    assert_eq!(page.shown()[0].value.as_deref(), Some("0.55"));

    // Check uniform queue
    let changes = a.drain_shader_param_changes();
    assert_eq!(changes, vec![("BLEND".to_string(), 0.55)]);

    // Check persisted file
    let saved = load_shader_params(d.path(), "Tuned");
    assert_eq!(saved.get("BLEND"), Some(&0.55));

    // 2. Tune "Frame blend" left by 2 steps (0.55 -> 0.45)
    press(&mut a, Btn::Left);
    press(&mut a, Btn::Left);
    let page = a.overlay_page().unwrap();
    assert_eq!(page.shown()[0].value.as_deref(), Some("0.45"));

    // 3. Move cursor down to "Reset" and press A
    press(&mut a, Btn::Down); // cursor = 1 ("Grid strength")
    assert_eq!(a.overlay_page().unwrap().cursor(), 1);
    press(&mut a, Btn::Down); // cursor = 2 ("Reset")
    assert_eq!(a.overlay_page().unwrap().cursor(), 2);
    press(&mut a, Btn::A); // Reset triggered!

    // Verify reset to defaults
    let page = a.overlay_page().unwrap();
    assert_eq!(page.shown()[0].value.as_deref(), Some("0.50"));
    assert_eq!(page.shown()[1].value.as_deref(), Some("0.25"));

    let saved = load_shader_params(d.path(), "Tuned");
    assert_eq!(saved.get("BLEND"), Some(&0.50));
    assert_eq!(saved.get("GRID"), Some(&0.25));

    // 4. Press B to close overlay
    press(&mut a, Btn::B);
    assert!(a.overlay_page().is_none());
    assert_eq!(a.in_game_menu(), Some(QuickRow::ShaderSettings));
}

#[test]
fn shader_settings_shared_across_platforms() {
    let d = tmp_root_with_carts(&["Emerald"]);
    common::write_gb_cart(&d, "Tetris", "TETRIS");
    std::fs::create_dir_all(d.path().join("Shaders")).unwrap();
    std::fs::write(d.path().join("Shaders/Tuned.glsl"), TEST_PARAM_SHADER).unwrap();

    // Set custom parameter on GBA
    let mut e = read_extras(d.path());
    e.set("card_shader_gba", Some("Tuned"));
    e.set("card_shader_gb", Some("Tuned"));
    write_extras(d.path(), &e).unwrap();

    let mut a = app_playing_in(d.path(), "Emerald");
    go_to(&mut a, QuickRow::ShaderSettings);
    press(&mut a, Btn::A);

    // Adjust BLEND to 0.70 (4 steps right)
    for _ in 0..4 {
        press(&mut a, Btn::Right);
    }
    assert_eq!(
        a.overlay_page().unwrap().shown()[0].value.as_deref(),
        Some("0.70")
    );
    press(&mut a, Btn::B);

    // Now boot Game Boy game with same shader
    let mut a_gb = app_playing_in(d.path(), "Tetris");
    assert_eq!(a_gb.card_shader_for(Platform::Gb).as_deref(), Some("Tuned"));
    go_to(&mut a_gb, QuickRow::ShaderSettings);
    press(&mut a_gb, Btn::A);

    // Verify it shares the same adjusted BLEND value (0.70)
    let page = a_gb.overlay_page().unwrap();
    assert_eq!(page.shown()[0].value.as_deref(), Some("0.70"));
}
