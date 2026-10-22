// Ported from nickclyde/slot-plus@c218577, crates/slot/tests/cheevos_overlay.rs.

#![cfg(feature = "cheevos")]

use std::time::Duration;

use slot::cheevos::overlay::{
    alpha, compose, lines, rect, Face, Queue, FACE_H, FACE_W, FADE, SHOW,
};
use slot_cheevos::Notice;

fn unlock() -> Notice {
    Notice::Unlocked {
        title: "First Step".into(),
        description: "Set the flag".into(),
        points: 5,
        badge_url: "u".into(),
    }
}

fn save(name: &str, rgba: &[u8]) {
    let Ok(dir) = std::env::var("SCRATCH_PNG_DIR") else {
        return;
    };
    let path = format!("{dir}/{name}.png");
    let file = std::fs::File::create(&path).expect("create png");
    let mut e = png::Encoder::new(std::io::BufWriter::new(file), FACE_W, FACE_H);
    e.set_color(png::ColorType::Rgba);
    e.set_depth(png::BitDepth::Eight);
    e.write_header()
        .expect("png header")
        .write_image_data(rgba)
        .expect("png data");
    println!("wrote {path}");
}

fn face(unlock: bool) -> Face {
    Face {
        rgba: vec![unlock as u8],
        unlock,
    }
}

#[test]
fn fades_in_holds_and_fades_out() {
    assert_eq!(alpha(Duration::ZERO), Some(0.0));
    assert_eq!(alpha(FADE / 2), Some(0.5));
    assert_eq!(alpha(FADE), Some(1.0));
    assert_eq!(alpha(SHOW / 2), Some(1.0));
    let late = alpha(SHOW - FADE / 2).unwrap();
    assert!((late - 0.5).abs() < 1e-3, "{late}");
    assert_eq!(alpha(SHOW), None);
}

// Fragile: centred across the top, on either width of screen.
#[test]
fn sits_centred_across_the_top() {
    for canvas_w in [640.0, 720.0] {
        let (x, y, w, h) = rect(canvas_w);
        assert_eq!((w, h), (FACE_W as f32, FACE_H as f32));
        assert_eq!(x + w / 2.0, canvas_w / 2.0, "not centred on {canvas_w}");
        assert!(y > 0.0 && y < 40.0, "{y}");
    }
}

#[test]
fn an_unlock_names_the_achievement_and_its_points() {
    let (head, detail) = lines(&unlock());
    assert_eq!(head, "First Step - 5 pts");
    assert_eq!(detail, "Set the flag");
}

#[test]
fn an_expired_token_asks_for_the_password_again() {
    let (_, detail) = lines(&Notice::LoginFailed {
        reason: "token expired".into(),
        needs_password: true,
    });
    assert!(detail.contains("Settings > Network"));
}

#[test]
fn a_bad_clock_hint_replaces_a_certificate_error() {
    let (_, detail) = lines(&Notice::LoginFailed {
        reason: "invalid peer certificate: Expired".into(),
        needs_password: false,
    });
    assert_eq!(detail, "Check the clock");
}

#[test]
fn a_game_without_achievements_says_so() {
    let (_, detail) = lines(&Notice::GameLoaded {
        title: "G".into(),
        unlocked: 0,
        total: 0,
        badge_url: String::new(),
    });
    assert_eq!(detail, "No achievements yet");
}

#[test]
fn composes_a_panel_with_ink_and_the_badge() {
    let badge = vec![0x40u8; 64 * 64 * 4]
        .chunks(4)
        .flat_map(|_| [200, 10, 10, 255])
        .collect::<Vec<u8>>();
    let f = compose(&unlock(), Some(&badge));
    save("cheevos-unlock", &f.rgba);
    assert_eq!(f.rgba.len(), (FACE_W * FACE_H * 4) as usize);
    assert!(f.unlock);
    let at = |x: u32, y: u32| &f.rgba[((y * FACE_W + x) * 4) as usize..][..4];
    assert_eq!(at(12, 20), [200, 10, 10, 255]);
    assert!(
        f.rgba.chunks(4).any(|p| p[0] > 0xe0 && p[3] > 0xe0),
        "no text ink"
    );
}

#[test]
fn a_missing_badge_still_composes() {
    let f = compose(&Notice::Offline, None);
    assert_eq!(f.rgba.len(), (FACE_W * FACE_H * 4) as usize);
    assert!(!f.unlock);
}

#[test]
fn a_full_queue_drops_news_before_unlocks() {
    let mut q = Queue::default();
    q.push(face(false));
    for _ in 0..7 {
        q.push(face(true));
    }
    q.push(face(true));
    assert_eq!(q.len(), 8);
    assert!(std::iter::from_fn(|| q.pop()).all(|f| f.unlock));

    let mut q = Queue::default();
    for _ in 0..8 {
        q.push(face(true));
    }
    q.push(face(false));
    assert_eq!(q.len(), 8);
    q.push(face(true));
    assert_eq!(q.len(), 9);
}
