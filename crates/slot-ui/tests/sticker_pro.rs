use slot_ui::{
    device_sticker_face, rg35xxpro_ratings, sticker_face, StickerDevice, StickerFields, DC,
    STICKER_H, STICKER_W,
};

fn fields(battery: Option<u8>) -> StickerFields<'static> {
    StickerFields {
        battery,
        serial: "04738857",
    }
}

fn shot(name: &str, rgba: &[u8]) {
    let Ok(dir) = std::env::var("SCRATCH_PNG_DIR") else {
        return;
    };
    let f = std::fs::File::create(format!("{dir}/{name}.png")).unwrap();
    let mut e = png::Encoder::new(std::io::BufWriter::new(f), STICKER_W, STICKER_H);
    e.set_color(png::ColorType::Rgba);
    e.set_depth(png::BitDepth::Eight);
    e.write_header().unwrap().write_image_data(rgba).unwrap();
}

#[test]
fn only_the_rg35xx_pro_gets_its_own_label() {
    assert_eq!(
        StickerDevice::of_target("rg35xxpro"),
        StickerDevice::Rg35xxPro
    );
    for t in ["", "rg35xxsp", "rg34xxsp", "rg35xxplus"] {
        assert_eq!(StickerDevice::of_target(t), StickerDevice::Sp, "{t:?}");
    }
    let f = fields(Some(87));
    assert_eq!(
        device_sticker_face(StickerDevice::Sp, &f).rgba,
        sticker_face(&f).rgba,
        "the RG SP lost upstream's sticker"
    );
}

#[test]
fn the_ratings_read_as_printed_with_the_charge_after_the_battery() {
    let [rating, battery, input] = rg35xxpro_ratings(&fields(Some(87)));
    assert_eq!(rating, format!("RATING: 5V{DC}7.5W"));
    assert_eq!(battery, "BATTERY: Li-Po 3.8V 3200mAh (87%)");
    assert_eq!(input, format!("ADAPTER INPUT: 5V{DC}\u{2265}1.5A"));
    let [_, battery, _] = rg35xxpro_ratings(&fields(None));
    assert_eq!(
        battery, "BATTERY: Li-Po 3.8V 3200mAh",
        "no gauge read as a number"
    );
}

#[test]
fn the_label_is_grey_and_stands_clear_of_the_face_edges() {
    let face = device_sticker_face(StickerDevice::Rg35xxPro, &fields(Some(87)));
    shot("sticker-rg35xxpro", &face.rgba);
    assert_eq!((face.w, face.h), (STICKER_W, STICKER_H));
    let at = |x: u32, y: u32| &face.rgba[((y * face.w + x) * 4) as usize..][..4];
    assert_eq!(
        at(10, STICKER_H / 2)[3],
        0,
        "the label runs to the face's edge"
    );
    let ground = at(STICKER_W / 2, 6);
    assert!(
        ground[3] == 0xff && (0x80..0xa8).contains(&ground[0]),
        "the label is not the Pro's grey: {ground:?}"
    );
    let inked = face
        .rgba
        .chunks(4)
        .filter(|p| p[3] == 0xff && p[0] > 0xe0)
        .count();
    assert!(inked > 4000, "too little printed on the label: {inked}");
}

#[test]
fn the_print_is_centred_on_the_label() {
    // The label is 560 wide in the middle of the face.
    let (left, right) = ((STICKER_W - 560) / 2, (STICKER_W + 560) / 2);
    for battery in [Some(100), Some(7), None] {
        let face = device_sticker_face(StickerDevice::Rg35xxPro, &fields(battery));
        let inked = |x: u32, ys: std::ops::Range<u32>| {
            ys.clone().any(|y| {
                let p = &face.rgba[((y * face.w + x) * 4) as usize..][..4];
                p[0] > 0xe0 && p[1] > 0xe0
            })
        };
        for (what, ys) in [("wordmark and model", 8..84), ("body", 96..216)] {
            let xs: Vec<u32> = (left + 6..right - 6)
                .filter(|&x| inked(x, ys.clone()))
                .collect();
            let (first, last) = (xs[0], xs[xs.len() - 1]);
            let (l, r) = (first - left, right - 1 - last);
            assert!(
                l.abs_diff(r) <= 6,
                "{what} with {battery:?}: {l} px in on the left, {r} on the right"
            );
        }
    }
}
