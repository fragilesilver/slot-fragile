use crate::surface::{OUT_H, OUT_W};

pub fn framed(panel: (u32, u32)) -> bool {
    panel.0 > 0 && panel.0 * 3 == panel.1 * 4
}

pub fn screen_rect(panel: (u32, u32)) -> (i32, i32, i32, i32) {
    let w = panel.0;
    let h = (w * OUT_H + OUT_W / 2) / OUT_W;
    ((panel.0 - w) as i32 / 2, 0, w as i32, h as i32)
}

pub fn game_rect(panel: (u32, u32), sp_frame: bool) -> (i32, i32, i32, i32) {
    let (x, _, w, h) = screen_rect(panel);
    let gap = panel.1 as i32 - h;
    let y = match sp_frame {
        true => 0,
        false => gap - gap / 2,
    };
    (x, y, w, h)
}

/// The largest whole multiple of `pic` that fits the panel, centred: x, y from the top, w, h.
/// Ported from marghost/slot_modded@a78747c, crates/slot-gfx/src/bezel.rs.
pub fn integer_rect(panel: (u32, u32), pic: (u32, u32)) -> (i32, i32, i32, i32) {
    let k = (panel.0 / pic.0.max(1)).min(panel.1 / pic.1.max(1)).max(1);
    let (w, h) = (pic.0 * k, pic.1 * k);
    (
        (panel.0 as i32 - w as i32) / 2,
        (panel.1 as i32 - h as i32) / 2,
        w as i32,
        h as i32,
    )
}

pub fn lifted_rect(panel: (u32, u32), lift: f32) -> (i32, i32, i32, i32) {
    let (x, _, w, h) = screen_rect(panel);
    let t = lift.clamp(0.0, 1.0);
    let eased = t * t * (3.0 - 2.0 * t);
    let drop = ((panel.1 as i32 - h) as f32 * (1.0 - eased)).round() as i32;
    (x, drop, w, h)
}
