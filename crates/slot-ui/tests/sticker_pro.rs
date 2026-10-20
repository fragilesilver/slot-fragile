use slot_ui::{
    device_sticker_face, rg35xxpro_credit_lines, rg35xxpro_ratings, sticker_face, StickerDevice,
    StickerFields, DC, RG35XXPRO_STICKER_H, STICKER_H, STICKER_W,
};

fn fields(battery: Option<u8>) -> StickerFields<'static> {
    StickerFields {
        battery,
        serial: "04738857",
    }
}

fn shot(name: &str, rgba: &[u8], h: u32) {
    let Ok(dir) = std::env::var("SCRATCH_PNG_DIR") else {
        return;
    };
    let f = std::fs::File::create(format!("{dir}/{name}.png")).unwrap();
    let mut e = png::Encoder::new(std::io::BufWriter::new(f), STICKER_W, h);
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
    shot("sticker-rg35xxpro", &face.rgba, face.h);
    assert_eq!((face.w, face.h), (STICKER_W, RG35XXPRO_STICKER_H));
    assert!(
        face.h > STICKER_H,
        "no room under the label for the credits"
    );
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
        let credits = 224..face.h - 6;
        for (what, ys) in [
            ("wordmark and model", 8..84),
            ("body", 96..216),
            ("credits", credits),
        ] {
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

#[test]
fn the_band_under_the_label_carries_upstreams_credits_and_fragiles() {
    let all = rg35xxpro_credit_lines().join(" ");
    for owed in [
        "MGBA",
        "GPSP",
        "BASEOS BY PVAIBHAV",
        "OPEN SANS",
        "NERD",
        "LCD3X",
        "MARKUS",
        "CREDITS.MD",
        "BRANDON T. KOWALSKI'S GAME BOY",
    ] {
        assert!(
            all.contains(owed),
            "the credits do not mention {owed}: {all}"
        );
    }
}

#[test]
fn the_label_outline_is_drawn_as_tall_as_the_face() {
    let svg = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/assets/sticker_rg35xxpro.svg"
    ))
    .unwrap();
    assert!(
        svg.contains(&format!("viewBox=\"0 0 560 {RG35XXPRO_STICKER_H}\"")),
        "regenerate it: python3 tools/fragile/sticker_rg35xxpro.py {RG35XXPRO_STICKER_H}"
    );
}

#[test]
fn the_fork_signs_off_at_the_bottom() {
    assert_eq!(
        slot_ui::RG35XXPRO_FORK,
        "Fork in Brunei Darussalam by a cat in pink headphone"
    );
    let all = rg35xxpro_credit_lines().join(" ");
    assert!(
        !all.contains("MY CHILDHOOD"),
        "the cart sounds still speak as upstream: {all}"
    );
    let face = device_sticker_face(StickerDevice::Rg35xxPro, &fields(Some(50)));
    let low = (face.h - 24..face.h - 6)
        .flat_map(|y| (0..face.w).map(move |x| (x, y)))
        .filter(|&(x, y)| face.rgba[((y * face.w + x) * 4) as usize] > 0xe0)
        .count();
    assert!(low > 150, "nothing printed along the bottom: {low}");
}
