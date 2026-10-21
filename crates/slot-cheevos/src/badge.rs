// Ported from nickclyde/slot-plus@29b8a81, crates/slot-cheevos/src/badge.rs.

pub const BADGE: usize = 64;

pub fn decode(bytes: &[u8]) -> Option<Vec<u8>> {
    let mut decoder = png::Decoder::new(bytes);
    decoder.set_transformations(png::Transformations::normalize_to_color8());
    let mut reader = decoder.read_info().ok()?;
    let mut buf = vec![0; reader.output_buffer_size()];
    let info = reader.next_frame(&mut buf).ok()?;
    let (w, h) = (info.width as usize, info.height as usize);
    if w == 0 || h == 0 {
        return None;
    }
    let channels = match info.color_type {
        png::ColorType::Grayscale => 1,
        png::ColorType::GrayscaleAlpha => 2,
        png::ColorType::Rgb => 3,
        png::ColorType::Rgba => 4,
        png::ColorType::Indexed => return None,
    };
    let mut out = vec![0u8; BADGE * BADGE * 4];
    for y in 0..BADGE {
        for x in 0..BADGE {
            let src = &buf[((y * h / BADGE) * w + x * w / BADGE) * channels..][..channels];
            let rgba = match channels {
                1 => [src[0], src[0], src[0], 0xff],
                2 => [src[0], src[0], src[0], src[1]],
                3 => [src[0], src[1], src[2], 0xff],
                _ => [src[0], src[1], src[2], src[3]],
            };
            out[(y * BADGE + x) * 4..][..4].copy_from_slice(&rgba);
        }
    }
    Some(out)
}
