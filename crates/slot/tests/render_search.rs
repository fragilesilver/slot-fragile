#![cfg(any(target_os = "macos", target_os = "linux"))]

// The search keyboard and the Add to List page, drawn whole. Set SCRATCH_PNG_DIR to see them.

mod common;

use std::collections::VecDeque;

use common::{clocked, tmp_root_with_carts};
use slot::frontend::Frontend;
use slot_gfx::{Compositor, HeadlessSurface, OUT_H, OUT_W};
use slot_input::{Btn, InputSource, Millis, RawEvent};
use slot_power::SimPlatform;
use slot_store::collections::LISTS_DIR;
use slot_ui::{edge, Keyboard};

struct Script(VecDeque<Vec<RawEvent>>);

impl InputSource for Script {
    fn poll(&mut self, _now: Millis) -> Vec<RawEvent> {
        self.0.pop_front().unwrap_or_default()
    }
}

fn step(f: &mut Frontend, input: &mut Script, events: Vec<RawEvent>) {
    input.0.push_back(events);
    f.advance(input);
}

fn tap(f: &mut Frontend, input: &mut Script, btn: Btn) {
    step(f, input, vec![RawEvent::Down(btn)]);
    step(f, input, vec![RawEvent::Up(btn)]);
}

fn at(px: &[u8], x: usize, y: usize) -> [u8; 3] {
    let o = (y * OUT_W as usize + x) * 4;
    [px[o], px[o + 1], px[o + 2]]
}

fn inked(px: &[u8], (x, y, w, h): (f32, f32, f32, f32)) -> usize {
    let (x, y) = (x as usize, y as usize);
    (y..y + h as usize)
        .flat_map(|py| (x..x + w as usize).map(move |px| (px, py)))
        .filter(|&(qx, qy)| at(px, qx, qy).iter().all(|c| *c > 0xc0))
        .count()
}

fn composed(f: &mut Frontend, c: &mut Compositor, name: &str) -> Vec<u8> {
    f.compose(c);
    let px = c.read_frame();
    if let Ok(dir) = std::env::var("SCRATCH_PNG_DIR") {
        let path = format!("{dir}/shelves-{name}.png");
        let file = std::fs::File::create(&path).expect("create png");
        let mut e = png::Encoder::new(std::io::BufWriter::new(file), OUT_W, OUT_H);
        e.set_color(png::ColorType::Rgba);
        e.set_depth(png::BitDepth::Eight);
        e.write_header()
            .expect("png header")
            .write_image_data(&px)
            .expect("png data");
        println!("wrote {path}");
    }
    px
}

/// The keyboard's faces are drawn on a thread: compose until they are up.
fn settle(f: &mut Frontend, c: &mut Compositor, input: &mut Script) {
    for _ in 0..200 {
        f.compose(c);
        step(f, input, Vec::new());
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
}

#[test]
fn the_search_keyboard_and_add_to_list_render_over_the_shelf() {
    let Ok(surface) = HeadlessSurface::new() else {
        return;
    };
    let Ok(mut c) = Compositor::new(&surface) else {
        return;
    };
    let d = tmp_root_with_carts(&["Emerald", "Fusion", "Golden Sun"]);
    std::fs::create_dir_all(d.path().join(LISTS_DIR)).unwrap();
    std::fs::write(d.path().join(LISTS_DIR).join("Couch.txt"), "Fusion\n").unwrap();
    clocked(d.path());
    let mut f = Frontend::boot(Box::new(SimPlatform::at(d.path().to_path_buf())));
    f.upload_faces(&mut c);
    let mut input = Script(VecDeque::new());
    settle(&mut f, &mut c, &mut input);

    tap(&mut f, &mut input, Btn::X);
    assert!(f.app().search().is_some(), "X did not open the search");
    for _ in 0..2 {
        tap(&mut f, &mut input, Btn::Right);
    }
    tap(&mut f, &mut input, Btn::A);
    settle(&mut f, &mut c, &mut input);
    assert_eq!(f.app().search().unwrap().query(), "E");
    let px = composed(&mut f, &mut c, "search");
    let canvas = f.app().canvas();
    let (row, col) = f.app().search().unwrap().keyboard.at();
    let (x, y, w, h) = Keyboard::key_rect(canvas, row, col);
    let lit = edge().map(|v| (v * 255.0).round() as i32);
    let corner = at(&px, x as usize + 2, y as usize + 2).map(i32::from);
    assert!(
        (0..3).all(|k| (corner[k] - lit[k]).abs() <= 2),
        "the lit key is {corner:?}, not the edge colour {lit:?}"
    );
    assert!(inked(&px, (x, y, w, h)) > 0, "the lit key has no letter");
    assert!(
        inked(&px, (80.0, 26.0, 480.0, 48.0)) > 0,
        "the field shows nothing typed"
    );

    a_menu_y(&mut f, &mut input);
    assert!(
        f.app().search().is_some(),
        "MENU+Y reached the shelf under the search"
    );
    tap(&mut f, &mut input, Btn::Menu);
    settle(&mut f, &mut c, &mut input);
    assert!(f.app().search().is_none(), "MENU did not close the search");

    a_menu_y(&mut f, &mut input);
    settle(&mut f, &mut c, &mut input);
    let page = f
        .app()
        .overlay_page()
        .expect("MENU+Y did not open Add to List")
        .clone();
    assert_eq!(f.app().quick_menu(), None, "the chord opened the settings");
    let labels: Vec<&str> = page.rows().iter().map(|r| r.label.as_str()).collect();
    assert_eq!(labels, ["Couch", "New List"]);
    let px = composed(&mut f, &mut c, "add-to-list");
    let top = slot_ui::list_row_top(&page, 0);
    assert!(
        inked(&px, (100.0, top, 400.0, 44.0)) > 0,
        "the Couch row has no label"
    );
}

fn a_menu_y(f: &mut Frontend, input: &mut Script) {
    step(f, input, vec![RawEvent::Down(Btn::Menu)]);
    tap(f, input, Btn::Y);
    step(f, input, vec![RawEvent::Up(Btn::Menu)]);
    for _ in 0..25 {
        step(f, input, Vec::new());
    }
}

#[test]
fn a_game_boy_carts_picker_shows_gambatte_and_mgba() {
    let Ok(surface) = HeadlessSurface::new() else {
        return;
    };
    let Ok(mut c) = Compositor::new(&surface) else {
        return;
    };
    let d = common::tmp_root_with_gb_carts(&["Tetris", "Zelda"]);
    clocked(d.path());
    let mut f = Frontend::boot(Box::new(SimPlatform::at(d.path().to_path_buf())));
    f.upload_faces(&mut c);
    let mut input = Script(VecDeque::new());
    settle(&mut f, &mut c, &mut input);
    tap(&mut f, &mut input, Btn::Start);
    settle(&mut f, &mut c, &mut input);
    assert_eq!(
        f.app().core_picker_choices(),
        Some([slot_store::Core::Gambatte, slot_store::Core::Mgba])
    );
    composed(&mut f, &mut c, "picker-gb");
    tap(&mut f, &mut input, Btn::Right);
    settle(&mut f, &mut c, &mut input);
    assert_eq!(f.app().core_picker(), Some(slot_store::Core::Mgba));
    composed(&mut f, &mut c, "picker-gb-mgba");
}
