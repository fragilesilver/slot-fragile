// Ported from nickclyde/slot-plus@29b8a81, crates/slot-cheevos/tests/badge.rs.

use slot_cheevos::badge::{decode, BADGE};

fn encode(w: u32, h: u32, color: png::ColorType, pixels: &[u8], palette: Option<&[u8]>) -> Vec<u8> {
    let mut out = Vec::new();
    let mut enc = png::Encoder::new(&mut out, w, h);
    enc.set_color(color);
    enc.set_depth(png::BitDepth::Eight);
    if let Some(p) = palette {
        enc.set_palette(p.to_vec());
    }
    enc.write_header()
        .unwrap()
        .write_image_data(pixels)
        .unwrap();
    out
}

fn pixel(rgba: &[u8], x: usize, y: usize) -> [u8; 4] {
    rgba[(y * BADGE + x) * 4..][..4].try_into().unwrap()
}

#[test]
fn decodes_an_rgb_badge_to_opaque_rgba() {
    let px: Vec<u8> = (0..64 * 64).flat_map(|_| [10, 20, 30]).collect();
    let rgba = decode(&encode(64, 64, png::ColorType::Rgb, &px, None)).unwrap();
    assert_eq!(rgba.len(), BADGE * BADGE * 4);
    assert_eq!(pixel(&rgba, 5, 5), [10, 20, 30, 0xff]);
}

#[test]
fn decodes_grey_and_palette_badges() {
    let grey = vec![77u8; 64 * 64];
    let rgba = decode(&encode(64, 64, png::ColorType::Grayscale, &grey, None)).unwrap();
    assert_eq!(pixel(&rgba, 0, 0), [77, 77, 77, 0xff]);

    let indices = vec![1u8; 64 * 64];
    let palette = [0, 0, 0, 200, 100, 50];
    let rgba = decode(&encode(
        64,
        64,
        png::ColorType::Indexed,
        &indices,
        Some(&palette),
    ))
    .unwrap();
    assert_eq!(pixel(&rgba, 63, 63), [200, 100, 50, 0xff]);
}

#[test]
fn scales_other_sizes_to_the_badge_size() {
    let px: Vec<u8> = (0..32 * 32)
        .flat_map(|i| {
            if i % 32 < 16 {
                [255, 0, 0, 255]
            } else {
                [0, 0, 255, 128]
            }
        })
        .collect();
    let rgba = decode(&encode(32, 32, png::ColorType::Rgba, &px, None)).unwrap();
    assert_eq!(rgba.len(), BADGE * BADGE * 4);
    assert_eq!(pixel(&rgba, 0, 10), [255, 0, 0, 255]);
    assert_eq!(pixel(&rgba, 63, 10), [0, 0, 255, 128]);
}

#[test]
fn garbage_is_not_a_badge() {
    assert_eq!(decode(b"not a png"), None);
    assert_eq!(decode(&[]), None);
}
