//! The label on the back of the Anbernic RG35XX Pro, drawn as `sticker.rs` draws the RG SP's:
//! its text as printed on the device, the battery's charge beside its rating, and the marks
//! in `sticker_rg35xxpro.svg`. Any other device keeps upstream's sticker.

use crate::art::render_svg;
use crate::plate::UndoFace;
use crate::sticker::{sticker_face, Canvas, StickerFields, COPYRIGHT, DC, HOME, STICKER_W};
use crate::text;

const LABEL_SVG: &str = include_str!("../assets/sticker_rg35xxpro.svg");
const WORDMARK_SVG: &str = include_str!("../assets/wordmark_anbernic.svg");

/// The label is narrower than the RG SP's, so it sits in the middle of the same face and
/// the About screen places it the same way.
const LABEL_W: u32 = 560;
const LABEL_X: u32 = (STICKER_W - LABEL_W) / 2;

/// Taller than the printed label, which has no room for slot's credits: they go in a band
/// under it, as on the RG SP's sticker.
pub const RG35XXPRO_STICKER_H: u32 = 330;

const INK: [u8; 3] = [0xf4, 0xf4, 0xf4];
const RULE_Y: u32 = 220;
const CREDITS_Y: f32 = 229.0;
const CREDITS_PX: f32 = 10.0;
const CREDITS_W: f32 = 500.0;
const COPYRIGHT_PX: f32 = 9.0;

/// Upstream's sticker credits, said shorter to fit the band, and the cart sounds named as
/// Brandon's, since this label is not his to speak in.
const CREDITS_SHORT: &str = "Artwork by Markus. Emulation by mGBA and gpSP. OS is BaseOS by \
     pvaibhav. Type is Open Sans and Nerd Fonts Symbols by Ryan L McIntyre. Panel mask from \
     Gigaherz's LCD3x. Cart sounds from Brandon T. Kowalski's Game Boy. Fragile's ports are \
     in CREDITS.md.";

/// Where this fork comes from, the way upstream's sticker ends on MADE IN ITHACA.
pub const RG35XXPRO_FORK: &str = "Fork in Brunei Darussalam by a cat in pink headphone";
const FORK_PX: f32 = 8.0;
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

/// Upstream's credits and Fragile's line, wrapped to the band under the label.
pub fn rg35xxpro_credit_lines() -> Vec<String> {
    let Some(font) = text::label_font() else {
        return Vec::new();
    };
    text::fit(font, CREDITS_SHORT, CREDITS_W, 12, CREDITS_PX, CREDITS_PX).lines
}

/// Where the left column starts, and the right one, before the print is centred.
const LEFT_X: f32 = 26.0;
const RIGHT_X: f32 = 278.0;
const RATING_PX: f32 = 14.0;
const WARNING_PX: f32 = 12.0;

fn rg35xxpro_face(f: &StickerFields) -> UndoFace {
    let mut c = Canvas::blank(STICKER_W, RG35XXPRO_STICKER_H);
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
    if let Some(label) = render_svg(&label, LABEL_W, RG35XXPRO_STICKER_H) {
        c.blit(LABEL_X, 0, &label, LABEL_W, RG35XXPRO_STICKER_H);
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

    c.rect(LABEL_X + 26, RULE_Y, LABEL_W - 52, 1, INK);
    let mut y = CREDITS_Y;
    for line in rg35xxpro_credit_lines() {
        let w = c.measure_as(&line, CREDITS_PX, false);
        y = c.print_as(centred(w), y, &line, CREDITS_PX, INK, false);
    }
    let copyright = format!("{COPYRIGHT}. {HOME}");
    let w = c.measure_as(&copyright, COPYRIGHT_PX, false);
    let y = c.print_as(centred(w), y + 4.0, &copyright, COPYRIGHT_PX, INK, false);
    let w = c.measure_as(RG35XXPRO_FORK, FORK_PX, false);
    c.print_as(centred(w), y, RG35XXPRO_FORK, FORK_PX, INK, false);
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
