//! Fragile: cartridge photo cropping and label fitting for all cartridge systems (Phase 13).
//! Ported and generalized from pvaibhav/slot `artwork.rs`. Cuts the sticker label out from full
//! cartridge photos (LaunchBox cart-fronts / 3D renders) for GBA, GB, GBC, NES, and SNES, and
//! resamples to the platform's exact label well dimensions with Lanczos3 filtering.

use image::imageops::FilterType;
use image::{DynamicImage, ImageFormat, ImageReader, Limits, RgbImage};
use slot_store::Platform;
use std::io::Cursor;

pub fn label_dimensions(platform: Platform) -> (u32, u32) {
    match platform {
        Platform::Gba => (264, 136),
        Platform::Gb => (220, 195),
        Platform::Gbc => (223, 199),
        Platform::Nes => (143, 232),
        Platform::Snes => (122, 97),
        Platform::PlayStation => (337, 337),
        Platform::Nds => (224, 216),
    }
}

pub fn decode(bytes: &[u8]) -> Option<DynamicImage> {
    let mut reader = ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .ok()?;
    let mut limits = Limits::default();
    limits.max_image_width = Some(4096);
    limits.max_image_height = Some(4096);
    limits.max_alloc = Some(64 * 1024 * 1024);
    reader.limits(limits);
    reader.decode().ok()
}

pub fn encode_png(image: &RgbImage) -> Option<Vec<u8>> {
    let mut out = Cursor::new(Vec::new());
    image.write_to(&mut out, ImageFormat::Png).ok()?;
    Some(out.into_inner())
}

/// Prepares a cartridge label from raw image bytes (JPEG or PNG): if it's a full cartridge photo,
/// the plastic casing is cropped away; if it's already a sticker, it is fitted cleanly.
pub fn prepare_cart_label(bytes: &[u8], platform: Platform, target_scale: u32) -> Option<Vec<u8>> {
    if platform.spec().discs {
        return None;
    }
    let image = decode(bytes)?;
    let cropped = crop_cart_sticker(&image, platform);
    let (lw, lh) = label_dimensions(platform);
    let (tw, th) = (lw * target_scale, lh * target_scale);
    let resized = cropped.resize_exact(tw, th, FilterType::Lanczos3).to_rgb8();
    encode_png(&resized)
}

/// Cuts the printed sticker out from a full cartridge photo based on the cartridge mould's
/// physical aspect ratio and label placement. If the aspect ratio does not match a cart casing,
/// the image is assumed to be an existing label or box art and kept whole.
fn crop_cart_sticker(image: &DynamicImage, platform: Platform) -> DynamicImage {
    let (w, h) = (image.width(), image.height());
    if w < 32 || h < 32 {
        return image.clone();
    }
    let aspect = w as f64 / h as f64;

    let bounds = match platform {
        // GBA cart front (aspect ~1.74): well inset
        Platform::Gba if (1.65..=1.85).contains(&aspect) => Some((
            w * 137 / 1000,
            h * 226 / 1000,
            w * 728 / 1000,
            h * 640 / 1000,
        )),
        // Game Boy / Game Boy Color cart front (aspect ~0.89): central sticker
        Platform::Gb | Platform::Gbc if (0.84..=0.94).contains(&aspect) => Some((
            w * 135 / 1000,
            h * 285 / 1000,
            w * 730 / 1000,
            h * 573 / 1000,
        )),
        // NES cart front (aspect ~0.89): tall vertical label in the right groove
        Platform::Nes if (0.80..=0.96).contains(&aspect) => {
            Some((w * 401 / 1000, h * 6 / 1000, w * 471 / 1000, h * 676 / 1000))
        }
        // SNES cart front (aspect ~1.57): central sticker art window
        Platform::Snes if (1.45..=1.70).contains(&aspect) => Some((
            w * 283 / 1000,
            h * 13 / 1000,
            w * 339 / 1000,
            h * 424 / 1000,
        )),
        // Nintendo DS Game Card front (aspect ~0.94): central square sticker
        Platform::Nds if (0.88..=1.02).contains(&aspect) => Some((
            w * 127 / 1000,
            h * 94 / 1000,
            w * 747 / 1000,
            h * 679 / 1000,
        )),
        _ => None,
    };

    if let Some((left, top, cw, ch)) = bounds {
        if cw > 0 && ch > 0 && left + cw <= w && top + ch <= h {
            return image.crop_imm(left, top, cw, ch);
        }
    }

    image.clone()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gba_cart_photo_is_cropped_to_label_well() {
        // 1000x574 GBA photo mockup (aspect ~1.742)
        let img = DynamicImage::new_rgb8(1000, 574);
        let cropped = crop_cart_sticker(&img, Platform::Gba);
        assert_eq!(cropped.width(), 728);
        assert_eq!(cropped.height(), 367); // 574 * 640 / 1000
    }

    #[test]
    fn nes_cart_photo_is_cropped_to_vertical_groove() {
        // 304x343 NES photo mockup (aspect ~0.886)
        let img = DynamicImage::new_rgb8(304, 343);
        let cropped = crop_cart_sticker(&img, Platform::Nes);
        assert_eq!(cropped.width(), 304 * 471 / 1000);
        assert_eq!(cropped.height(), 343 * 676 / 1000);
    }

    #[test]
    fn snes_cart_photo_is_cropped_to_center_window() {
        // 360x229 SNES photo mockup (aspect ~1.572)
        let img = DynamicImage::new_rgb8(360, 229);
        let cropped = crop_cart_sticker(&img, Platform::Snes);
        assert_eq!(cropped.width(), 360 * 339 / 1000);
        assert_eq!(cropped.height(), 229 * 424 / 1000);
    }

    #[test]
    fn already_cropped_sticker_is_not_re_cropped() {
        // An image that is already 264x136 GBA label (aspect ~1.94)
        let img = DynamicImage::new_rgb8(264, 136);
        let cropped = crop_cart_sticker(&img, Platform::Gba);
        assert_eq!(cropped.width(), 264);
        assert_eq!(cropped.height(), 136);
    }
}
