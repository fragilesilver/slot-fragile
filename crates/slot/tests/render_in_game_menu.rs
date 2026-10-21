#![cfg(any(target_os = "macos", target_os = "linux"))]

mod common;

use std::collections::VecDeque;
use std::time::{Duration, Instant};

use common::{clocked, tmp_root_with_carts};
use slot::frontend::Frontend;
use slot_gfx::{Compositor, HeadlessSurface, OUT_H, OUT_W};
use slot_input::{Btn, InputSource, Millis, RawEvent};
use slot_power::SimPlatform;
use slot_ui::{quick_row_top, QuickRow};

struct Script(VecDeque<Vec<RawEvent>>);

impl InputSource for Script {
    fn poll(&mut self, _now: Millis) -> Vec<RawEvent> {
        self.0.pop_front().unwrap_or_default()
    }
}

fn tap(f: &mut Frontend, input: &mut Script, btn: Btn) {
    input.0.push_back(vec![RawEvent::Down(btn)]);
    f.advance(input);
    input.0.push_back(vec![RawEvent::Up(btn)]);
    f.advance(input);
}

fn run_for(f: &mut Frontend, input: &mut Script, secs: f32) {
    let until = Instant::now() + Duration::from_secs_f32(secs);
    while Instant::now() < until {
        f.advance(input);
        std::thread::sleep(Duration::from_millis(2));
    }
}

fn at(px: &[u8], x: usize, y: usize) -> [u8; 3] {
    let o = (y * OUT_W as usize + x) * 4;
    [px[o], px[o + 1], px[o + 2]]
}

fn composed(f: &mut Frontend, c: &mut Compositor, name: &str) -> Vec<u8> {
    f.compose(c);
    let px = c.read_frame();
    if let Ok(dir) = std::env::var("SCRATCH_PNG_DIR") {
        let path = format!("{dir}/in-game-menu-{name}.png");
        let file = std::fs::File::create(&path).expect("create png");
        let mut e = png::Encoder::new(std::io::BufWriter::new(file), OUT_W, OUT_H);
        e.set_color(png::ColorType::Rgba);
        e.set_depth(png::BitDepth::Eight);
        e.write_header()
            .expect("png header")
            .write_image_data(&px)
            .expect("png data");
    }
    px
}

fn bright(p: [u8; 3]) -> u32 {
    p.iter().map(|&c| u32::from(c)).sum()
}

#[test]
fn the_in_game_menu_darkens_the_game_and_screen_leaves_it_bare() {
    let Ok(surface) = HeadlessSurface::new() else {
        return;
    };
    let Ok(mut c) = Compositor::new(&surface) else {
        return;
    };
    let d = tmp_root_with_carts(&["Emerald", "Fusion"]);
    clocked(d.path());
    let mut f = Frontend::boot(Box::new(SimPlatform::at(d.path().to_path_buf())));
    f.upload_faces(&mut c);
    let mut input = Script(VecDeque::new());
    tap(&mut f, &mut input, Btn::A);
    run_for(&mut f, &mut input, 4.0);
    tap(&mut f, &mut input, Btn::Menu);
    run_for(&mut f, &mut input, 0.1);
    let held = composed(&mut f, &mut c, "held");
    run_for(&mut f, &mut input, 0.4);
    assert_eq!(f.app().in_game_menu(), Some(QuickRow::Resume));
    let top = composed(&mut f, &mut c, "top");
    for btn in [Btn::Down, Btn::Down, Btn::A, Btn::A] {
        tap(&mut f, &mut input, btn);
    }
    run_for(&mut f, &mut input, 0.3);
    assert_eq!(f.app().in_game_menu(), Some(QuickRow::Shader));
    let screen = composed(&mut f, &mut c, "screen");

    let panel = quick_row_top(QuickRow::Shader, &f.app().in_game_hidden()) as usize - 16;
    let (mut probed, mut lit) = (0, 0);
    for y in (48..panel).step_by(17) {
        for x in (0..150).chain(570..OUT_W as usize).step_by(13) {
            let (bare, dark) = (at(&screen, x, y), at(&top, x, y));
            assert_eq!(
                at(&held, x, y),
                bare,
                "({x}, {y}): the menu drew inside the double tap window"
            );
            for ch in 0..3 {
                let want = f32::from(bare[ch]) * 0.35;
                assert!(
                    (f32::from(dark[ch]) - want).abs() <= 3.0,
                    "({x}, {y}) channel {ch}: bare {} drew {} under the scrim, want about {want}",
                    bare[ch],
                    dark[ch]
                );
            }
            probed += 1;
            lit += usize::from(bright(bare) > 60);
        }
    }
    assert!(
        lit * 4 > probed,
        "the game is too dark to prove anything: {lit} of {probed} probes lit"
    );

    for (side, xs) in [("battery", 8..120), ("clock", 600..712)] {
        let ink = (6..34)
            .flat_map(|y| xs.clone().map(move |x| (x, y)))
            .filter(|&(x, y)| bright(at(&top, x, y)) > 600)
            .count();
        assert!(
            ink > 20,
            "no {side} in the menu's corner: {ink} bright pixels"
        );
    }

    let row = quick_row_top(QuickRow::Shader, &f.app().in_game_hidden()) as usize;
    let ink = (row..row + 40)
        .flat_map(|y| (64..400).map(move |x| (x, y)))
        .filter(|&(x, y)| bright(at(&screen, x, y)) > 600)
        .count();
    assert!(
        ink > 100,
        "no Shader label in the panel: {ink} bright pixels"
    );
    let (x, y) = (700, OUT_H as usize - 4);
    let (panel_px, bare) = (at(&screen, x, y), at(&held, x, y));
    assert!(
        bright(panel_px) < bright(bare),
        "the panel is not darker than the game under it: {panel_px:?} over {bare:?}"
    );
}

#[test]
fn the_controls_page_renders() {
    let Ok(surface) = HeadlessSurface::new() else {
        return;
    };
    let Ok(mut c) = Compositor::new(&surface) else {
        return;
    };
    let d = tmp_root_with_carts(&["Emerald", "Fusion"]);
    clocked(d.path());
    let mut f = Frontend::boot(Box::new(SimPlatform::at(d.path().to_path_buf())));
    f.upload_faces(&mut c);
    let mut input = Script(VecDeque::new());
    tap(&mut f, &mut input, Btn::A);
    run_for(&mut f, &mut input, 4.0);
    tap(&mut f, &mut input, Btn::Menu);
    run_for(&mut f, &mut input, 0.5);
    for btn in [Btn::Down, Btn::Down, Btn::A, Btn::Down, Btn::Down, Btn::A] {
        tap(&mut f, &mut input, btn);
    }
    run_for(&mut f, &mut input, 0.3);
    assert_eq!(f.app().in_game_menu(), Some(QuickRow::GameUp));
    composed(&mut f, &mut c, "controls");
    for _ in 0..7 {
        tap(&mut f, &mut input, Btn::Down);
    }
    run_for(&mut f, &mut input, 0.2);
    assert_eq!(f.app().in_game_menu(), Some(QuickRow::GameStart));
    composed(&mut f, &mut c, "controls-scrolled");
    tap(&mut f, &mut input, Btn::A);
    run_for(&mut f, &mut input, 0.2);
    assert!(f.app().capturing());
    composed(&mut f, &mut c, "controls-waiting");
    input.0.push_back(vec![RawEvent::Down(Btn::Menu)]);
    f.advance(&mut input);
    tap(&mut f, &mut input, Btn::X);
    input.0.push_back(vec![RawEvent::Up(Btn::Menu)]);
    f.advance(&mut input);
    run_for(&mut f, &mut input, 0.2);
    assert!(!f.app().capturing());
    tap(&mut f, &mut input, Btn::Up);
    run_for(&mut f, &mut input, 0.2);
    composed(&mut f, &mut c, "controls-bound");
}

#[test]
fn the_battery_badge_draws_in_the_bottom_right_of_a_game() {
    let Ok(surface) = HeadlessSurface::new() else {
        return;
    };
    let Ok(mut c) = Compositor::new(&surface) else {
        return;
    };
    let d = tmp_root_with_carts(&["Emerald", "Fusion"]);
    clocked(d.path());
    let mut e = slot_store::read_extras(d.path());
    e.set_flag("battery_badge", true);
    slot_store::write_extras(d.path(), &e).unwrap();
    let mut f = Frontend::boot(Box::new(SimPlatform::at(d.path().to_path_buf())));
    f.upload_faces(&mut c);
    let mut input = Script(VecDeque::new());
    tap(&mut f, &mut input, Btn::A);
    run_for(&mut f, &mut input, 4.0);
    assert!(f.app().battery().is_some(), "the sim gave no battery");
    let px = composed(&mut f, &mut c, "battery-badge");
    // The gauge's white outline, in the bottom right corner.
    let (w, h) = (OUT_W as usize, OUT_H as usize);
    let white = (h - 40..h - 8)
        .flat_map(|y| (w - 140..w - 8).map(move |x| (x, y)))
        .filter(|&(x, y)| at(&px, x, y).iter().all(|c| *c > 0xe8))
        .count();
    assert!(white > 60, "no badge in the corner: {white} white pixels");
}

#[test]
fn the_this_game_page_renders_over_the_game() {
    let Ok(surface) = HeadlessSurface::new() else {
        return;
    };
    let Ok(mut c) = Compositor::new(&surface) else {
        return;
    };
    let d = tmp_root_with_carts(&["Emerald", "Fusion"]);
    clocked(d.path());
    let mut f = Frontend::boot(Box::new(SimPlatform::at(d.path().to_path_buf())));
    f.upload_faces(&mut c);
    let mut input = Script(VecDeque::new());
    tap(&mut f, &mut input, Btn::A);
    run_for(&mut f, &mut input, 4.0);
    tap(&mut f, &mut input, Btn::Menu);
    run_for(&mut f, &mut input, 0.5);
    for btn in [Btn::Down, Btn::Down, Btn::A] {
        tap(&mut f, &mut input, btn);
    }
    run_for(&mut f, &mut input, 0.2);
    composed(&mut f, &mut c, "settings");
    for btn in [Btn::Down, Btn::Down, Btn::Down, Btn::A] {
        tap(&mut f, &mut input, btn);
    }
    run_for(&mut f, &mut input, 0.3);
    assert_eq!(f.app().in_game_menu(), Some(QuickRow::ThisShader));
    let px = composed(&mut f, &mut c, "this-game");
    let row = quick_row_top(QuickRow::ThisShader, &f.app().in_game_hidden()) as usize;
    let ink = (row..row + 40)
        .flat_map(|y| (64..400).map(move |x| (x, y)))
        .filter(|&(x, y)| at(&px, x, y).iter().map(|c| u32::from(*c)).sum::<u32>() > 600)
        .count();
    assert!(
        ink > 100,
        "no Shader label on This Game: {ink} bright pixels"
    );
}

#[test]
fn a_game_boy_game_at_3x_sits_in_the_classic_3x_bezel_on_a_640_by_480_panel() {
    let Ok(surface) = HeadlessSurface::new() else {
        return;
    };
    let Ok(mut c) = Compositor::new(&surface) else {
        return;
    };
    let d = common::tmp_root_with_gb_carts(&["Tetris", "Kirby"]);
    clocked(d.path());
    let card = common::repo_root().join("card/Bezels/GB");
    for name in ["Classic.png", "Classic 3x.png"] {
        std::fs::copy(card.join(name), d.path().join("Bezels/GB").join(name)).unwrap();
    }
    let mut s = slot_store::read_slot_state(d.path());
    s.bezel_gb = slot_store::BezelPick::File("Classic.png".into());
    slot_store::write_slot_state(d.path(), &s).unwrap();
    let mut e = slot_store::read_extras(d.path());
    e.set_flag("integer_gb", true);
    slot_store::write_extras(d.path(), &e).unwrap();

    let panel = (640, 480);
    let mut f = Frontend::boot(Box::new(SimPlatform::at(d.path().to_path_buf())));
    f.upload_faces(&mut c);
    f.upload_bezel(&mut c, panel);
    let mut input = Script(VecDeque::new());
    tap(&mut f, &mut input, Btn::A);
    let until = Instant::now() + Duration::from_secs(4);
    while Instant::now() < until {
        f.advance(&mut input);
        f.render(&mut c, panel);
        std::thread::sleep(Duration::from_millis(2));
    }
    assert_eq!(f.app().integer_picture(), Some((160, 144)));
    f.render(&mut c, panel);
    let px = c.read_frame();
    assert_eq!(c.frame_size(), panel);
    if let Ok(dir) = std::env::var("SCRATCH_PNG_DIR") {
        let path = format!("{dir}/integer-classic-gb.png");
        let file = std::fs::File::create(&path).unwrap();
        let mut enc = png::Encoder::new(std::io::BufWriter::new(file), panel.0, panel.1);
        enc.set_color(png::ColorType::Rgba);
        enc.set_depth(png::BitDepth::Eight);
        enc.write_header().unwrap().write_image_data(&px).unwrap();
    }
    let at = |x: usize, y: usize| {
        let o = (y * 640 + x) * 4;
        [px[o], px[o + 1], px[o + 2]]
    };
    // The mock game's gradient lights the 480x432 window; the bezel's black frame is round it.
    let lit = |x: usize, y: usize| at(x, y).iter().map(|c| u32::from(*c)).sum::<u32>() > 60;
    assert!(lit(320, 240), "no game in the middle");
    assert!(
        lit(84, 240) && lit(555, 240),
        "the game does not reach the 3x window's sides"
    );
    assert!(!lit(40, 20), "the bezel's corner is not drawn");
}

const TINT: &str = "#if defined(VERTEX)
#if __VERSION__ >= 130
#define COMPAT_VARYING out
#define COMPAT_ATTRIBUTE in
#else
#define COMPAT_VARYING varying
#define COMPAT_ATTRIBUTE attribute
#endif
COMPAT_ATTRIBUTE vec4 VertexCoord;
COMPAT_ATTRIBUTE vec4 TexCoord;
COMPAT_VARYING vec4 TEX0;
uniform mat4 MVPMatrix;
void main() { gl_Position = MVPMatrix * VertexCoord; TEX0 = TexCoord; }
#elif defined(FRAGMENT)
#if __VERSION__ >= 130
#define COMPAT_VARYING in
out vec4 FragColor;
#else
#define COMPAT_VARYING varying
#define FragColor gl_FragColor
#endif
#ifdef GL_ES
precision mediump float;
#endif
COMPAT_VARYING vec4 TEX0;
void main() { FragColor = vec4(1.0, 0.0, 1.0, 1.0); }
#endif
";

fn game_with_card_shader(c: &mut Compositor, file: &str) -> (tempfile::TempDir, Frontend) {
    let d = tmp_root_with_carts(&["Emerald", "Fusion"]);
    clocked(d.path());
    std::fs::write(d.path().join("Shaders/Magenta.glsl"), file).unwrap();
    let mut e = slot_store::read_extras(d.path());
    e.set("card_shader_gba", Some("Magenta"));
    slot_store::write_extras(d.path(), &e).unwrap();
    let mut f = Frontend::boot(Box::new(SimPlatform::at(d.path().to_path_buf())));
    f.upload_faces(c);
    let mut input = Script(VecDeque::new());
    tap(&mut f, &mut input, Btn::A);
    run_for(&mut f, &mut input, 4.0);
    (d, f)
}

#[test]
fn a_card_shader_draws_the_game_and_a_broken_one_says_so() {
    let Ok(surface) = HeadlessSurface::new() else {
        return;
    };
    let Ok(mut c) = Compositor::new(&surface) else {
        return;
    };
    let (_d, mut f) = game_with_card_shader(&mut c, TINT);
    let px = composed(&mut f, &mut c, "card-shader");
    assert_eq!(
        at(&px, 360, 200),
        [255, 0, 255],
        "the card shader did not draw the game"
    );
    assert_eq!(f.app().toast(), None);
    drop(f);

    let (_d, mut f) = game_with_card_shader(&mut c, "not a shader");
    let px = composed(&mut f, &mut c, "card-shader-broken");
    assert_ne!(at(&px, 360, 200), [255, 0, 255]);
    assert_eq!(f.app().toast(), Some(slot_ui::Toast::ShaderFailed));
    assert!(
        at(&px, 360, 200).iter().any(|ch| *ch > 20),
        "the game went black behind a broken shader"
    );
}
