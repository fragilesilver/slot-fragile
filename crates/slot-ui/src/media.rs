//! The carts and discs of the systems past the Game Boy family. Ported from
//! cloudwithax/unslot@3da2bc3, crates/slot-ui/src/media.rs, cut to the systems Fragile runs.
//! The disc is unslot's render; Fragile draws the NES and SNES shells after the real carts with
//! `tools/fragile/media/draw_carts.py`, which prints their label and sticker boxes below.

use std::sync::OnceLock;

use slot_store::Platform;

use crate::bitmaps;

#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub enum Media {
    Nes,
    Snes,
    Disc,
    Nds,
}

struct Art {
    png: &'static [u8],
    w: u32,
    h: u32,
    label: (u32, u32, u32, u32),
    /// Printed on the shell in its own colours, so the game's colour does not tint it.
    sticker: Option<(u32, u32, u32, u32)>,
    colour: [u8; 3],
}

const ART: [Art; Media::COUNT] = [
    Art {
        png: include_bytes!("../../../assets/nes_neutral.png"),
        w: 304,
        h: 343,
        label: (122, 2, 143, 232),
        sticker: None,
        colour: [0x8a, 0x8a, 0x8e],
    },
    Art {
        png: include_bytes!("../../../assets/snes_neutral.png"),
        w: 360,
        h: 229,
        label: (102, 3, 122, 97),
        sticker: Some((68, 2, 223, 99)),
        colour: [0x9a, 0x99, 0xa0],
    },
    Art {
        png: include_bytes!("../../../assets/disc_neutral.png"),
        w: 337,
        h: 337,
        label: (0, 0, 337, 337),
        sticker: None,
        colour: [0xd8, 0xd8, 0xdc],
    },
    Art {
        png: include_bytes!("../../../assets/nds_neutral.png"),
        w: 300,
        h: 318,
        label: (38, 30, 224, 216),
        sticker: None,
        colour: [0x32, 0x32, 0x36],
    },
];

/// The disc's print runs from the clear hub to just inside the rim, as fractions of its radius:
/// 22.5 mm and 58.5 mm of a 60 mm disc.
const PRINT_INNER: f32 = 22.5 / 60.0;
const PRINT_OUTER: f32 = 58.5 / 60.0;

/// How tall a medium may sit in the slot: the Game Boy pak's seated height.
const SEATED_MAX_H: u32 = 259;

impl Media {
    pub const COUNT: usize = 4;
    pub const ALL: [Media; Media::COUNT] = [Media::Nes, Media::Snes, Media::Disc, Media::Nds];

    /// What a platform's games come on, or None for the Game Boy family's own moulds.
    pub fn of(platform: Platform) -> Option<Media> {
        Some(match platform {
            Platform::Gba | Platform::Gb | Platform::Gbc => return None,
            Platform::Nes => Media::Nes,
            Platform::Snes => Media::Snes,
            Platform::PlayStation => Media::Disc,
            Platform::Nds => Media::Nds,
        })
    }

    pub fn index(self) -> usize {
        self as usize
    }

    fn art(self) -> &'static Art {
        &ART[self.index()]
    }

    pub fn size(self) -> (u32, u32) {
        (self.art().w, self.art().h)
    }

    /// The label box, x, y, w, h in the shell's pixels.
    pub fn label(self) -> (u32, u32, u32, u32) {
        self.art().label
    }

    /// The part of the shell that keeps its own colours, x, y, w, h.
    pub fn sticker(self) -> Option<(u32, u32, u32, u32)> {
        self.art().sticker
    }

    /// The plastic a game gets when nothing names another colour for it.
    pub fn colour(self) -> [u8; 3] {
        self.art().colour
    }

    /// The size it is drawn at once it sits in the slot: as wide as a seated cart, unless that
    /// would stand taller than a seated Game Boy pak.
    pub fn seated(self, seated_w: u32) -> (u32, u32) {
        let (w, h) = self.size();
        let tall = (seated_w * h + w / 2) / w;
        if tall <= SEATED_MAX_H {
            return (seated_w, tall);
        }
        ((SEATED_MAX_H * w + h / 2) / h, SEATED_MAX_H)
    }

    /// The neutral shell, decoded once.
    pub fn neutral(self) -> &'static [u8] {
        static SHELLS: [OnceLock<Vec<u8>>; Media::COUNT] =
            [const { OnceLock::new() }; Media::COUNT];
        let art = self.art();
        bitmaps::load(art.png, art.w, art.h, &SHELLS[self.index()])
    }

    /// The shell's coverage, from the render's alpha.
    pub fn mask(self) -> &'static [u8] {
        static MASKS: [OnceLock<Vec<u8>>; Media::COUNT] = [const { OnceLock::new() }; Media::COUNT];
        MASKS[self.index()]
            .get_or_init(|| self.neutral().iter().skip(3).step_by(4).copied().collect())
    }

    /// Where on the label box the label may print, for a medium whose print is not the whole box:
    /// the disc's ring between its hub and rim, or the Nintendo DS card's chamfered label well.
    /// Coverage a pixel, row by row over the label box.
    pub fn label_mask(self) -> Option<&'static [u8]> {
        match self {
            Media::Disc => {
                static RING: OnceLock<Vec<u8>> = OnceLock::new();
                Some(RING.get_or_init(|| {
                    let (_, _, w, h) = self.label();
                    ring(w, h, PRINT_INNER, PRINT_OUTER)
                }))
            }
            Media::Nds => {
                static NDS_LABEL: OnceLock<Vec<u8>> = OnceLock::new();
                Some(NDS_LABEL.get_or_init(|| {
                    let (_, _, w, h) = self.label();
                    nds_label_mask(w, h)
                }))
            }
            _ => None,
        }
    }
}

/// An authentic Nintendo DS Game Card label mask (224x216): rounded top-left, top-right,
/// and bottom-right corners (radius 4px), with a distinctive 45-degree chamfer cutout at
/// the bottom-left corner (20px cut) matching real NTR-005 game cards.
fn nds_label_mask(w: u32, h: u32) -> Vec<u8> {
    let r = 4.0f32;
    let chamfer = 20.0f32;
    let mut out = Vec::with_capacity((w * h) as usize);
    let sqrt2 = std::f32::consts::SQRT_2;
    for y in 0..h {
        let py = y as f32 + 0.5;
        for x in 0..w {
            let px = x as f32 + 0.5;
            let mut cov = 1.0f32;
            if px < r && py < r {
                cov = cov.min((r - (r - px).hypot(r - py) + 0.5).clamp(0.0, 1.0));
            } else if px > w as f32 - r && py < r {
                cov = cov.min((r - (px - (w as f32 - r)).hypot(r - py) + 0.5).clamp(0.0, 1.0));
            } else if px > w as f32 - r && py > h as f32 - r {
                cov = cov.min(
                    (r - (px - (w as f32 - r)).hypot(py - (h as f32 - r)) + 0.5).clamp(0.0, 1.0),
                );
            } else if px < chamfer && py > h as f32 - chamfer {
                let d = (px - (py - (h as f32 - chamfer))) / sqrt2;
                cov = cov.min((d + 0.5).clamp(0.0, 1.0));
            }
            out.push((cov * 255.0 + 0.5).clamp(0.0, 255.0) as u8);
        }
    }
    out
}

/// A ring's coverage over a w by h box centred on it, the radii as fractions of the half width,
/// with its edges antialiased across a pixel.
fn ring(w: u32, h: u32, inner: f32, outer: f32) -> Vec<u8> {
    let half = w as f32 / 2.0;
    let (cx, cy) = (w as f32 / 2.0, h as f32 / 2.0);
    let mut out = Vec::with_capacity((w * h) as usize);
    for y in 0..h {
        for x in 0..w {
            let r = ((x as f32 + 0.5 - cx).hypot(y as f32 + 0.5 - cy)) / half;
            let into_outer = ((outer - r) * half + 0.5).clamp(0.0, 1.0);
            let past_inner = ((r - inner) * half + 0.5).clamp(0.0, 1.0);
            out.push((into_outer.min(past_inner) * 255.0).round() as u8);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_platform_past_the_game_boy_has_media() {
        for p in Platform::ALL {
            let game_boy = matches!(p, Platform::Gba | Platform::Gb | Platform::Gbc);
            assert_eq!(Media::of(p).is_none(), game_boy, "{p:?}");
        }
    }

    #[test]
    fn every_shell_decodes_at_the_size_it_claims_and_its_label_fits() {
        for m in Media::ALL {
            let (w, h) = m.size();
            assert_eq!(m.neutral().len(), (w * h * 4) as usize, "{m:?}");
            let (lx, ly, lw, lh) = m.label();
            assert!(lx + lw <= w && ly + lh <= h, "{m:?} label leaves the shell");
        }
    }

    #[test]
    fn a_shell_is_solid_at_its_label_and_clear_past_its_corners() {
        for m in Media::ALL {
            let (w, _) = m.size();
            let (lx, ly, lw, lh) = m.label();
            let at = |x: u32, y: u32| m.mask()[(y * w + x) as usize];
            let (cx, cy) = (lx + lw / 2, ly + lh / 4);
            if m != Media::Disc {
                assert_eq!(at(cx, cy), 255, "{m:?} has a hole in its label");
            }
            assert_eq!(at(0, 0), 0, "{m:?} fills its top left corner");
        }
    }

    #[test]
    fn the_disc_prints_on_its_ring_but_not_its_hub_or_past_its_rim() {
        let ring = Media::Disc.label_mask().expect("the disc has a ring");
        let (_, _, w, h) = Media::Disc.label();
        let at = |x: u32, y: u32| ring[(y * w + x) as usize];
        assert_eq!(at(w / 2, h / 2), 0, "the hub was printed on");
        assert_eq!(at(w / 2, h / 10), 255, "the print is missing above the hub");
        assert_eq!(at(0, 0), 0, "the corner past the rim was printed on");
    }

    #[test]
    fn the_nds_label_mask_cuts_the_bottom_left_corner() {
        let mask = Media::Nds.label_mask().expect("nds has a label mask");
        let (_, _, w, h) = Media::Nds.label();
        let at = |x: u32, y: u32| mask[(y * w + x) as usize];
        // Center is solid
        assert_eq!(at(w / 2, h / 2), 255, "label center should be solid");
        // Rounded corners
        assert_eq!(at(0, 0), 0, "TL corner should be rounded");
        assert_eq!(at(w - 1, 0), 0, "TR corner should be rounded");
        assert_eq!(at(w - 1, h - 1), 0, "BR corner should be rounded");
        // BL chamfer cut: corner (0, h-1) is cut out
        assert_eq!(at(0, h - 1), 0, "BL corner should be chamfered");
        assert_eq!(at(2, h - 3), 0, "inside BL chamfer cut should be 0");
        // Right along the bottom edge past the chamfer (x > 25) is solid
        assert_eq!(at(30, h - 1), 255, "bottom edge past chamfer should be solid");
    }

    #[test]
    fn nothing_sits_taller_in_the_slot_than_a_game_boy_pak() {
        for m in Media::ALL {
            let (w, h) = m.seated(240);
            assert!(h <= SEATED_MAX_H && w <= 240, "{m:?} sits at {w}x{h}");
        }
    }
}
