//! Fragile: the NES and SNES carts and the PlayStation disc on their own shelves.
#![cfg(any(target_os = "macos", target_os = "linux"))]

mod common;

use std::collections::VecDeque;

use common::{clocked, tmp_root_with_carts};
use slot::frontend::Frontend;
use slot_gfx::{Compositor, HeadlessSurface, OUT_H, OUT_W};
use slot_input::{Btn, InputSource, Millis, RawEvent};
use slot_power::SimPlatform;
use slot_store::Platform;
use slot_ui::{cart_box, default_background, rest_y};

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

fn patch(px: &[u8], x: usize, y: usize) -> [u32; 3] {
    let mut sum = [0u32; 3];
    let mut n = 0;
    for py in y - 4..y + 4 {
        for qx in x - 4..x + 4 {
            let o = (py * OUT_W as usize + qx) * 4;
            for k in 0..3 {
                sum[k] += px[o + k] as u32;
            }
            n += 1;
        }
    }
    sum.map(|s| s / n)
}

fn apart(a: [u32; 3], b: [u32; 3]) -> u32 {
    (0..3).map(|k| a[k].abs_diff(b[k])).sum()
}

fn settle(f: &mut Frontend, c: &mut Compositor, input: &mut Script) {
    f.compose(c);
    f.advance(input);
    std::thread::sleep(std::time::Duration::from_millis(300));
    f.advance(input);
}

fn composed(f: &mut Frontend, c: &mut Compositor, name: &str) -> Vec<u8> {
    f.compose(c);
    let px = c.read_frame();
    if let Ok(dir) = std::env::var("SCRATCH_PNG_DIR") {
        let path = format!("{dir}/media-{name}.png");
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

#[test]
fn each_new_system_puts_its_game_in_the_middle_of_its_shelf() {
    let Ok(surface) = HeadlessSurface::new() else {
        return;
    };
    let Ok(mut c) = Compositor::new(&surface) else {
        return;
    };
    let d = tmp_root_with_carts(&[]);
    for (dir, file) in [
        ("NES", "Super Mario Bros.nes"),
        ("SNES", "Super Metroid.sfc"),
        ("PS", "Final Fantasy VII.cue"),
    ] {
        let games = d.path().join("Games").join(dir);
        std::fs::create_dir_all(&games).unwrap();
        std::fs::write(games.join(file), vec![0u8; 0x8000]).unwrap();
    }
    clocked(d.path());
    let mut f = Frontend::boot(Box::new(SimPlatform::at(d.path().to_path_buf())));
    f.upload_faces(&mut c);
    let mut input = Script(VecDeque::new());
    f.advance(&mut input);
    settle(&mut f, &mut c, &mut input);
    let ground = default_background().expect("the default wallpaper");

    let mut seen = Vec::new();
    for _ in 0..3 {
        let name = f.app().shelf_platform_name().expect("a system's shelf");
        let platform = Platform::ALL
            .into_iter()
            .find(|p| p.spec().name == name)
            .expect("a known system");
        let px = composed(&mut f, &mut c, platform.spec().dir);
        let (_, h) = cart_box(platform);
        let (x, y) = (
            (OUT_W / 2) as usize,
            (rest_y(h as f32) + h as f32 * 0.75) as usize,
        );
        assert!(
            apart(patch(&px, x, y), patch(ground, x, y)) > 60,
            "no game in the middle of the {platform:?} shelf"
        );
        seen.push(platform);
        tap(&mut f, &mut input, Btn::R1);
        settle(&mut f, &mut c, &mut input);
    }
    seen.sort_by_key(|p| p.index());
    assert_eq!(seen, [Platform::Nes, Platform::Snes, Platform::PlayStation]);
}

fn lit_in(px: &[u8], xs: std::ops::Range<usize>, ys: std::ops::Range<usize>) -> usize {
    ys.flat_map(|y| xs.clone().map(move |x| (x, y)))
        .filter(|(x, y)| {
            let o = (y * OUT_W as usize + x) * 4;
            px[o..o + 3].iter().any(|c| *c > 8)
        })
        .count()
}

/// A SNES game on the mock core (256x224, shown at 4:3), played to the screen with Integer Scale
/// off or on.
fn snes_game_drawn(c: &mut Compositor, whole: bool) -> Vec<u8> {
    let d = tmp_root_with_carts(&[]);
    let games = d.path().join("Games/SNES");
    std::fs::create_dir_all(&games).unwrap();
    std::fs::write(games.join("Super Metroid.sfc"), vec![0u8; 0x8000]).unwrap();
    if whole {
        let mut extras = slot_store::read_extras(d.path());
        extras.set_flag("integer_snes", true);
        slot_store::write_extras(d.path(), &extras).unwrap();
    }
    clocked(d.path());
    let mut f = Frontend::boot(Box::new(SimPlatform::at(d.path().to_path_buf())));
    f.upload_faces(c);
    let mut input = Script(VecDeque::new());
    f.advance(&mut input);
    settle(&mut f, c, &mut input);
    tap(&mut f, &mut input, Btn::A);
    let until = std::time::Instant::now() + std::time::Duration::from_secs(4);
    while std::time::Instant::now() < until {
        f.advance(&mut input);
        std::thread::sleep(std::time::Duration::from_millis(2));
    }
    composed(&mut f, c, if whole { "snes-integer" } else { "snes-fit" })
}

#[test]
fn integer_scale_draws_a_snes_game_at_twice_its_size_in_the_middle() {
    let Ok(surface) = HeadlessSurface::new() else {
        return;
    };
    let Ok(mut c) = Compositor::new(&surface) else {
        return;
    };
    let rows = 200..280;
    // The 4:3 panel sits in the middle of the canvas; fitted, the picture fills it.
    let mid = OUT_W as usize / 2;
    let fit = snes_game_drawn(&mut c, false);
    assert!(
        lit_in(&fit, mid - 316..mid - 260, rows.clone()) > 1000,
        "fitted to 4:3, the picture should reach the panel's sides"
    );
    // At 2x the picture is 512 wide: 64 pixels of border inside the panel either side.
    let whole = snes_game_drawn(&mut c, true);
    assert_eq!(
        lit_in(&whole, mid - 316..mid - 258, rows.clone()),
        0,
        "at 2x the left 64 pixels of the panel are border"
    );
    assert_eq!(
        lit_in(&whole, mid + 258..mid + 316, rows.clone()),
        0,
        "and the right 64"
    );
    assert!(
        lit_in(&whole, mid - 250..mid + 250, rows) > 30_000,
        "the picture is not there at all"
    );
}
