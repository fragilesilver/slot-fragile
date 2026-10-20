//! The label on the back of the Anbernic RG35XX Pro, drawn as `sticker.rs` draws the RG SP's:
//! its text as printed on the device, the battery's charge beside its rating, and the marks
//! in `sticker_rg35xxpro.svg`. Any other device keeps upstream's sticker.

use crate::art::render_svg;
use crate::plate::UndoFace;
use crate::sticker::{sticker_face, Canvas, StickerFields, DC, STICKER_H, STICKER_W};

const LABEL_SVG: &str = include_str!("../assets/sticker_rg35xxpro.svg");
const WORDMARK_SVG: &str = include_str!("../assets/wordmark_anbernic.svg");

/// The label is narrower than the RG SP's, so it sits in the middle of the same face and
/// the About screen places it the same way.
const LABEL_W: u32 = 560;
const LABEL_X: u32 = (STICKER_W - LABEL_W) / 2;

const INK: [u8; 3] = [0xf4, 0xf4, 0xf4];
const WORDMARK_W: u32 = 300;

#[derive(Copy, Clone, PartialEq, Eq, Debug, Default)]
pub enum StickerDevice {
    /// Upstream's sticker, for the RG SP slot was made for and every other device.
    #[default]
    Sp,
    Rg35xxPro,
}

impl StickerDevice {
    pub fn of_target(target: &str) -> StickerDevice {
        match target {
            "rg35xxpro" => StickerDevice::Rg35xxPro,
            _ => StickerDevice::Sp,
        }
    }
}

pub fn device_sticker_face(device: StickerDevice, f: &StickerFields) -> UndoFace {
    match device {
        StickerDevice::Sp => sticker_face(f),
        StickerDevice::Rg35xxPro => rg35xxpro_face(f),
    }
}

/// The ratings on the left, as printed, with the charge after the battery's.
pub fn rg35xxpro_ratings(f: &StickerFields) -> [String; 3] {
    let charge = f.battery.map_or(String::new(), |p| format!(" ({p}%)"));
    [
        format!("RATING: 5V{DC}7.5W"),
        format!("BATTERY: Li-Po 3.8V 3200mAh{charge}"),
        format!("ADAPTER INPUT: 5V{DC}\u{2265}1.5A"),
    ]
}

pub const RG35XXPRO_MODEL: &str = "MODEL No.RG 35XXPro";

pub const RG35XXPRO_WARNINGS: [&str; 3] = [
    "Built in rechargeable li-po battery",
    "Only can use certified charger",
    "The battery may explode in the fire",
];

pub const RG35XXPRO_ORIGIN: &str = "MADE IN CHINA";

/// Where the left column starts, and the right one, before the print is centred.
const LEFT_X: f32 = 26.0;
const RIGHT_X: f32 = 278.0;
const RATING_PX: f32 = 14.0;
const WARNING_PX: f32 = 12.0;

fn rg35xxpro_face(f: &StickerFields) -> UndoFace {
    let mut c = Canvas::blank(STICKER_W, STICKER_H);
    let ratings = rg35xxpro_ratings(f);
    // The two columns and the marks under them move together, so the margins either side
    // of the widest line on each are the same.
    let left_w = ratings
        .iter()
        .map(|l| rating_width(&c, l))
        .fold(0.0, f32::max);
    let right_w = RG35XXPRO_WARNINGS
        .iter()
        .map(|l| c.measure_as(l, WARNING_PX, true))
        .fold(left_w - (RIGHT_X - LEFT_X), f32::max);
    let used = RIGHT_X + right_w - LEFT_X;
    let dx = ((LABEL_W as f32 - used) / 2.0 - LEFT_X).round();
    let label = LABEL_SVG.replacen("translate(0 0)", &format!("translate({dx} 0)"), 1);
    if let Some(label) = render_svg(&label, LABEL_W, STICKER_H) {
        c.blit(LABEL_X, 0, &label, LABEL_W, STICKER_H);
    }
    let centred = |w: f32| LABEL_X as f32 + ((LABEL_W as f32 - w) / 2.0).round();
    if let Some((mark, (mw, mh))) = wordmark() {
        c.blit(centred(mw as f32) as u32, 12, &mark, mw, mh);
    }
    let model_w = c.measure_as(RG35XXPRO_MODEL, 18.0, true);
    c.print_as(centred(model_w), 63.0, RG35XXPRO_MODEL, 18.0, INK, true);
    let at = |x: f32| LABEL_X as f32 + x + dx;
    c.print_as(at(352.0), 95.0, "Li-Po", 17.0, INK, true);
    let mut y = 102.0;
    for line in &ratings {
        print_rating(&mut c, at(LEFT_X), y, line);
        y += 20.0;
    }
    let mut y = 140.0;
    for line in RG35XXPRO_WARNINGS {
        y = c.print_as(at(RIGHT_X), y, line, WARNING_PX, INK, true);
    }
    c.print_as(at(256.0), 184.0, RG35XXPRO_ORIGIN, 16.0, INK, true);
    c.into_face()
}

fn rating_width(c: &Canvas, line: &str) -> f32 {
    let parts: Vec<&str> = line.split(DC).collect();
    let words: f32 = parts
        .iter()
        .map(|p| c.measure_as(p, RATING_PX, true) + 2.0)
        .sum();
    words + (parts.len() - 1) as f32 * (RATING_PX * 0.78 + 3.0)
}

/// A rating line, with its ⎓ drawn the way the RG SP's sticker draws it: the font has none.
fn print_rating(c: &mut Canvas, x: f32, y: f32, line: &str) {
    const PX: f32 = RATING_PX;
    let mut x = x;
    let mut parts = line.split(DC).peekable();
    while let Some(part) = parts.next() {
        c.print_as(x, y, part, PX, INK, true);
        x += c.measure_as(part, PX, true) + 2.0;
        if parts.peek().is_some() {
            x += c.dc(x, y, PX, INK) + 3.0;
        }
    }
}

fn wordmark() -> Option<(Vec<u8>, (u32, u32))> {
    let tree = usvg::Tree::from_str(WORDMARK_SVG, &usvg::Options::default()).ok()?;
    let size = tree.size();
    let h = (WORDMARK_W as f32 * size.height() / size.width()).round() as u32;
    Some((render_svg(WORDMARK_SVG, WORDMARK_W, h)?, (WORDMARK_W, h)))
}
