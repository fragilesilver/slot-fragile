#![cfg(any(target_os = "macos", target_os = "linux"))]

use std::sync::{Mutex, MutexGuard, PoisonError};

use slot_gfx::{Canvas, Compositor, HeadlessSurface};
use slot_ui::{
    list_label_face, list_label_width, list_row_top, list_value_face, quick_description_face,
    quick_pointer_face, quick_scroll_face, Draw, ListPage, ListPageView, ListRow, ListRowFaces,
    UndoFace, OUT_H, OUT_W,
};

static GL: Mutex<()> = Mutex::new(());

fn compositor() -> Option<(MutexGuard<'static, ()>, HeadlessSurface, Compositor)> {
    let guard = GL.lock().unwrap_or_else(PoisonError::into_inner);
    let surface = HeadlessSurface::new().ok()?;
    let compositor = Compositor::new(&surface).ok()?;
    Some((guard, surface, compositor))
}

fn shot(px: &[u8], name: &str) {
    let Ok(dir) = std::env::var("SCRATCH_PNG_DIR") else {
        return;
    };
    let path = format!("{dir}/{name}.png");
    let file = std::fs::File::create(&path).expect("create png");
    let mut e = png::Encoder::new(std::io::BufWriter::new(file), OUT_W, OUT_H);
    e.set_color(png::ColorType::Rgba);
    e.set_depth(png::BitDepth::Eight);
    e.write_header()
        .expect("png header")
        .write_image_data(px)
        .expect("png data");
    println!("wrote {path}");
}

fn cheats() -> ListPage {
    let names = [
        "Infinite Health",
        "Max Money",
        "Walk Through Walls",
        "Every Pokemon In The Pokedex And Every Item In The Bag",
        "Rare Candy x99",
        "Shiny Encounters",
        "Fast Text",
        "No Random Battles",
        "Master Ball In Shop",
    ];
    let rows = names
        .iter()
        .enumerate()
        .map(|(i, n)| {
            ListRow::new(*n)
                .with_value(if i % 3 == 0 { "On" } else { "Off" })
                .with_help(format!("Cheat {} of {}.", i + 1, names.len()))
        })
        .collect();
    let mut page = ListPage::new(rows);
    page.set_cursor(4);
    page
}

fn upload(c: &mut Compositor, f: &UndoFace) -> (slot_ui::TexId, u32, u32) {
    (c.create_texture(f.w, f.h, &f.rgba), f.w, f.h)
}

fn rendered(c: &mut Compositor, page: &ListPage) -> Vec<u8> {
    let canvas = Canvas::FRAMED;
    let (start, _) = page.window();
    let rows: Vec<ListRowFaces> = page
        .shown()
        .iter()
        .enumerate()
        .map(|(i, row)| {
            let lit = start + i == page.cursor();
            let room = list_label_width(canvas, row.value.is_some());
            ListRowFaces {
                label: upload(c, &list_label_face(&row.label, lit, room)),
                value: row
                    .value
                    .as_deref()
                    .map(|v| upload(c, &list_value_face(v, lit, slot_ui::LIST_VALUE_W))),
            }
        })
        .collect();
    let help = page
        .selected()
        .and_then(|r| r.help.as_deref())
        .map(|h| upload(c, &quick_description_face(&[h])));
    let view = ListPageView {
        page,
        rows: &rows,
        pointer: upload(c, &quick_pointer_face()),
        scroll: [
            upload(c, &quick_scroll_face(false)),
            upload(c, &quick_scroll_face(true)),
        ],
        empty: None,
        help,
        legends: &[],
        ground: None,
        over_game: false,
        footer_top: OUT_H as f32,
        canvas,
    };
    let mut list: Vec<Draw> = Vec::new();
    view.draw(&mut list);
    c.begin_frame();
    c.draw_list(&list);
    c.read_frame()
}

fn ink_in(px: &[u8], ys: std::ops::Range<usize>, xs: std::ops::Range<usize>) -> usize {
    ys.flat_map(|y| xs.clone().map(move |x| (y * OUT_W as usize + x) * 4))
        .filter(|&o| u32::from(px[o]) + u32::from(px[o + 1]) + u32::from(px[o + 2]) > 600)
        .count()
}

#[test]
fn the_cursors_row_is_drawn_bright_and_the_rest_dim() {
    let Some((_g, _s, mut c)) = compositor() else {
        return;
    };
    let page = cheats();
    let px = rendered(&mut c, &page);
    shot(&px, "list_page");
    let (start, len) = page.window();
    for slot in 0..len {
        let top = list_row_top(&page, slot) as usize;
        let ink = ink_in(&px, top..top + 40, 64..400);
        match start + slot == page.cursor() {
            true => assert!(ink > 100, "the cursor's row has {ink} bright pixels"),
            false => assert!(ink < 20, "row {slot} is bright: {ink} pixels"),
        }
    }
}
