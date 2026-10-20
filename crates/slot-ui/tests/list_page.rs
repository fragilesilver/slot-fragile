use slot_ui::{
    list_label_face, list_label_width, list_row_top, list_value_face, quick_row_top,
    quick_scroll_face, Canvas, Draw, ListPage, ListPageView, ListRow, ListRowFaces, QuickRow,
    TexId, LIST_VISIBLE, OUT_H,
};

fn rows(n: usize) -> Vec<ListRow> {
    (0..n).map(|i| ListRow::new(format!("Row {i}"))).collect()
}

const LABEL: usize = 100;
const VALUE: usize = 200;
const POINTER: TexId = TexId::from_raw(1);
const UP: TexId = TexId::from_raw(2);
const DOWN: TexId = TexId::from_raw(3);
const EMPTY: TexId = TexId::from_raw(4);
const HELP: TexId = TexId::from_raw(5);

fn drawn(page: &ListPage, with_values: bool) -> Vec<Draw> {
    let faces: Vec<ListRowFaces> = (0..page.shown().len())
        .map(|i| ListRowFaces {
            label: (TexId::from_raw(LABEL + i), 120, 40),
            value: with_values.then_some((TexId::from_raw(VALUE + i), 60, 40)),
        })
        .collect();
    let view = ListPageView {
        page,
        rows: &faces,
        pointer: (POINTER, 32, 48),
        scroll: [(UP, 20, 40), (DOWN, 20, 40)],
        empty: Some((EMPTY, 200, 40)),
        help: Some((HELP, 300, 22)),
        legends: &[],
        ground: None,
        over_game: false,
        footer_top: OUT_H as f32,
        canvas: Canvas::FRAMED,
    };
    let mut out = Vec::new();
    view.draw(&mut out);
    out
}

fn place(out: &[Draw], tex: TexId) -> Option<(f32, f32, f32)> {
    out.iter().find_map(|d| match *d {
        Draw::Tex {
            x, y, w, tex: t, ..
        } if t == tex => Some((x, y, w)),
        _ => None,
    })
}

#[test]
fn a_short_list_never_scrolls_and_shows_no_carets() {
    let mut page = ListPage::new(rows(5));
    page.set_cursor(4);
    assert_eq!(page.window(), (0, 5));
    let out = drawn(&page, false);
    assert!(place(&out, UP).is_none() && place(&out, DOWN).is_none());
}

#[test]
fn a_long_list_keeps_the_cursor_in_view_and_stops_at_its_ends() {
    let mut page = ListPage::new(rows(20));
    for cursor in 0..20 {
        page.set_cursor(cursor);
        let (start, len) = page.window();
        assert_eq!(len, LIST_VISIBLE);
        assert!(
            (start..start + len).contains(&cursor),
            "{cursor} out of view from {start}"
        );
        assert!(start + len <= 20, "the window ran past the end");
    }
}

#[test]
fn up_and_down_go_round_and_paging_stops_at_the_ends() {
    let mut page = ListPage::new(rows(10));
    page.up();
    assert_eq!(page.cursor(), 9);
    page.down();
    assert_eq!(page.cursor(), 0);
    page.page_down();
    assert_eq!(page.cursor(), LIST_VISIBLE);
    page.page_down();
    assert_eq!(page.cursor(), 9);
    page.page_up();
    page.page_up();
    assert_eq!(page.cursor(), 0);
}

#[test]
fn an_empty_list_moves_nowhere_and_says_so() {
    let mut page = ListPage::new(Vec::new());
    page.up();
    page.down();
    page.page_down();
    assert_eq!(page.cursor(), 0);
    assert!(page.selected().is_none());
    let out = drawn(&page, false);
    assert!(place(&out, EMPTY).is_some(), "no empty line");
    assert!(
        place(&out, POINTER).is_none(),
        "a pointer with nothing to point at"
    );
}

#[test]
fn new_rows_keep_the_cursor_on_the_same_label() {
    let mut page = ListPage::new(rows(5));
    page.set_cursor(3);
    page.set_rows(vec![ListRow::new("New"), ListRow::new("Row 3")]);
    assert_eq!(page.selected().map(|r| r.label.as_str()), Some("Row 3"));
    page.set_rows(rows(2));
    assert_eq!(page.cursor(), 1, "the cursor ran past a shorter list");
    page.set_rows(Vec::new());
    assert_eq!(page.cursor(), 0);
}

#[test]
fn the_pointer_follows_the_cursor_in_the_window() {
    let mut page = ListPage::new(rows(20));
    page.set_cursor(10);
    let (start, _) = page.window();
    let out = drawn(&page, false);
    let (_, pointer_y, _) = place(&out, POINTER).expect("no pointer");
    let (_, label_y, _) = place(&out, TexId::from_raw(LABEL + 10 - start)).expect("no label");
    assert!(
        (label_y - pointer_y - 4.0).abs() < 1.0,
        "the pointer at {pointer_y} is not on the cursor's label at {label_y}"
    );
}

#[test]
fn rows_sit_where_a_menu_page_of_the_same_length_would() {
    let page = ListPage::new(rows(6));
    let general = QuickRow::GENERAL[0];
    assert_eq!(list_row_top(&page, 0), quick_row_top(general, &[]));
}

#[test]
fn values_sit_on_the_right_and_carets_show_more_either_side() {
    let mut page = ListPage::new(rows(20));
    page.set_cursor(10);
    let out = drawn(&page, true);
    let (x, _, w) = place(&out, TexId::from_raw(VALUE)).expect("no value");
    let (label_x, _, _) = place(&out, TexId::from_raw(LABEL)).expect("no label");
    assert!(
        x > label_x + 120.0 && x + w <= Canvas::FRAMED.w,
        "value at {x}"
    );
    let (_, up_y, _) = place(&out, UP).expect("no more-above caret");
    let (_, down_y, _) = place(&out, DOWN).expect("no more-below caret");
    assert!(up_y < list_row_top(&page, 0) && down_y >= list_row_top(&page, LIST_VISIBLE));
    page.set_cursor(0);
    assert!(place(&drawn(&page, true), UP).is_none());
}

#[test]
fn a_long_label_shrinks_to_its_room() {
    let canvas = Canvas::FRAMED;
    let room = list_label_width(canvas, true);
    assert!(room < list_label_width(canvas, false));
    let long = "Infinite health, infinite lives and every item in the game";
    let face = list_label_face(long, true, room);
    if face.w == 0 {
        return;
    }
    assert!(
        face.w as f32 <= room + 2.0 * 18.0,
        "{} wide for {room} of room",
        face.w
    );
    assert_eq!(face.h, list_value_face("On", false).h);
}

#[test]
fn the_scroll_carets_have_ink() {
    for down in [false, true] {
        let face = quick_scroll_face(down);
        assert!(face.w > 0, "no caret face");
        assert!(face.rgba.chunks(4).any(|p| p[3] > 0), "a blank caret");
    }
}
