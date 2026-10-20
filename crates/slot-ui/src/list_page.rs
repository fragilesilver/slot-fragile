//! A page of rows slot fills at run time: cheats, networks in range, search results, a disc
//! list. Laid out like a QuickMenu page, with the same pitch, pointer, ground and legends, so it
//! reads as one more page of the menu. Ideas from cloudwithax/unslot, crates/slot-ui/src/list_page.rs.

use crate::draw::{Canvas, Draw, TexId, OUT_H};
use crate::plate::{centred_hints, UndoFace, HINT_H, LEGEND_GAP};
use crate::power_menu::{MENU_INK, MENU_PAD};
use crate::quick_menu::{
    draw_menu_ground, fitted_text_face, push, DESCRIPTION_GAP, DIM_INK, GROUND_SCRIM, LEGEND_Y,
    POINTER_BLEED, POINTER_GAP, SCROLL_GAP, TYPE_DROP,
};
use crate::quick_menu::{QUICK_EDGE, QUICK_LABEL_X, QUICK_PITCH, QUICK_VISIBLE};
use crate::slot_chrome::MOUTH_H;

/// How many rows are in view at once. A longer list scrolls to keep the cursor in view.
pub const LIST_VISIBLE: usize = QUICK_VISIBLE;
/// The widest a value on the right is drawn; the label gets the rest of the row.
pub const LIST_VALUE_W: f32 = 200.0;
/// How small a long label may shrink before it is cut.
const LABEL_MIN_PX: f32 = 20.0;
const VALUE_GAP: f32 = 24.0;

#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct ListRow {
    pub label: String,
    /// What is said of it on the right: "On", "Saved", "Disc 2".
    pub value: Option<String>,
    /// The help line shown under the page while the cursor is on this row.
    pub help: Option<String>,
}

impl ListRow {
    pub fn new(label: impl Into<String>) -> Self {
        ListRow {
            label: label.into(),
            ..ListRow::default()
        }
    }

    pub fn with_value(mut self, value: impl Into<String>) -> Self {
        self.value = Some(value.into());
        self
    }

    pub fn with_help(mut self, help: impl Into<String>) -> Self {
        self.help = Some(help.into());
        self
    }
}

#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct ListPage {
    rows: Vec<ListRow>,
    cursor: usize,
}

impl ListPage {
    pub fn new(rows: Vec<ListRow>) -> Self {
        ListPage { rows, cursor: 0 }
    }

    pub fn rows(&self) -> &[ListRow] {
        &self.rows
    }

    pub fn cursor(&self) -> usize {
        self.cursor
    }

    pub fn selected(&self) -> Option<&ListRow> {
        self.rows.get(self.cursor)
    }

    pub fn selected_mut(&mut self) -> Option<&mut ListRow> {
        self.rows.get_mut(self.cursor)
    }

    /// New rows for the page, eg a fresh scan. The cursor stays on the row with the same label
    /// if there still is one, otherwise on the same place as near as the new list allows.
    pub fn set_rows(&mut self, rows: Vec<ListRow>) {
        let kept = self
            .selected()
            .and_then(|old| rows.iter().position(|r| r.label == old.label));
        self.cursor = kept
            .unwrap_or(self.cursor)
            .min(rows.len().saturating_sub(1));
        self.rows = rows;
    }

    pub fn set_cursor(&mut self, at: usize) {
        self.cursor = at.min(self.rows.len().saturating_sub(1));
    }

    /// Up and Down go round, the way the menu pages do.
    pub fn up(&mut self) {
        if !self.rows.is_empty() {
            self.cursor = (self.cursor + self.rows.len() - 1) % self.rows.len();
        }
    }

    pub fn down(&mut self) {
        if !self.rows.is_empty() {
            self.cursor = (self.cursor + 1) % self.rows.len();
        }
    }

    /// L1 and R1 move a screenful and stop at the ends.
    pub fn page_up(&mut self) {
        self.cursor = self.cursor.saturating_sub(LIST_VISIBLE);
    }

    pub fn page_down(&mut self) {
        self.set_cursor(self.cursor + LIST_VISIBLE);
    }

    /// The first row in view and how many are shown: the window keeps the cursor as near its
    /// middle as the ends allow, like `quick_window`.
    pub fn window(&self) -> (usize, usize) {
        let count = self.rows.len();
        if count <= LIST_VISIBLE {
            return (0, count);
        }
        let start = self
            .cursor
            .saturating_sub(LIST_VISIBLE / 2)
            .min(count - LIST_VISIBLE);
        (start, LIST_VISIBLE)
    }

    /// The rows in view, in order, as `ListPageView::rows` wants their faces.
    pub fn shown(&self) -> &[ListRow] {
        let (start, len) = self.window();
        &self.rows[start..start + len]
    }

    pub fn more_above(&self) -> bool {
        self.window().0 > 0
    }

    pub fn more_below(&self) -> bool {
        let (start, len) = self.window();
        start + len < self.rows.len()
    }
}

/// Where the row in view at `slot` sits, from the top of the screen.
pub fn list_row_top(page: &ListPage, slot: usize) -> f32 {
    let (_, len) = page.window();
    (LEGEND_Y - QUICK_PITCH * len as f32) / 2.0 + QUICK_PITCH * slot as f32
}

/// How wide a row's label may be drawn on a canvas, with or without a value beside it.
pub fn list_label_width(canvas: Canvas, has_value: bool) -> f32 {
    let room = canvas.w - QUICK_EDGE - QUICK_LABEL_X;
    match has_value {
        true => room - LIST_VALUE_W - VALUE_GAP,
        false => room,
    }
}

pub fn list_label_face(label: &str, lit: bool, max_w: f32) -> UndoFace {
    fitted_text_face(label, ink(lit), max_w, LABEL_MIN_PX)
}

pub fn list_value_face(value: &str, lit: bool) -> UndoFace {
    fitted_text_face(value, ink(lit), LIST_VALUE_W, LABEL_MIN_PX)
}

fn ink(lit: bool) -> [u8; 3] {
    if lit {
        MENU_INK
    } else {
        DIM_INK
    }
}

pub type Face = (TexId, u32, u32);

/// The faces for one row in view; the lit or dim one, as the cursor says.
#[derive(Copy, Clone, Debug)]
pub struct ListRowFaces {
    pub label: Face,
    pub value: Option<Face>,
}

pub struct ListPageView<'a> {
    pub page: &'a ListPage,
    /// One per row in `page.shown()`.
    pub rows: &'a [ListRowFaces],
    pub pointer: Face,
    /// The carets for more above and more below.
    pub scroll: [Face; 2],
    /// Said in the middle of the page when there are no rows, eg "No cheats for this game".
    pub empty: Option<Face>,
    /// The selected row's help line.
    pub help: Option<Face>,
    pub legends: &'a [(TexId, u32)],
    pub ground: Option<(TexId, u32)>,
    pub over_game: bool,
    /// Where the sliding footer starts. The scrim stops there so it does not darken the strip.
    pub footer_top: f32,
    pub canvas: Canvas,
}

impl ListPageView<'_> {
    pub fn draw(&self, out: &mut Vec<Draw>) {
        match self.over_game {
            false => draw_menu_ground(self.canvas, self.ground, out),
            true => out.push(Draw::Rect {
                x: 0.0,
                y: 0.0,
                w: self.canvas.w,
                h: self.footer_top.max(0.0),
                colour: [0.0, 0.0, 0.0, GROUND_SCRIM],
            }),
        }
        let (right, pad) = (self.canvas.w - QUICK_EDGE, MENU_PAD as f32);
        let (start, _) = self.page.window();
        for (slot, faces) in self.rows.iter().enumerate() {
            let y = list_row_top(self.page, slot) + TYPE_DROP;
            let (tex, w, h) = faces.label;
            let x = QUICK_LABEL_X - pad;
            push(out, tex, x, y, w, h);
            if start + slot == self.page.cursor() {
                let (tex, pw, ph) = self.pointer;
                let px = x + pad - POINTER_GAP - pw as f32;
                push(out, tex, px, y - POINTER_BLEED as f32, pw, ph);
            }
            if let Some((tex, w, h)) = faces.value {
                push(out, tex, right + pad - w as f32, y, w, h);
            }
        }
        if self.page.rows().is_empty() {
            if let Some((tex, w, h)) = self.empty {
                let y = (LEGEND_Y - h as f32) / 2.0;
                push(out, tex, self.canvas.centred(w as f32), y, w, h);
            }
        }
        let [(up_tex, uw, uh), (down_tex, dw, dh)] = self.scroll;
        if self.page.more_above() {
            let y = list_row_top(self.page, 0) - uh as f32 - SCROLL_GAP;
            push(out, up_tex, self.canvas.centred(uw as f32), y, uw, uh);
        }
        if self.page.more_below() {
            let y = list_row_top(self.page, LIST_VISIBLE) + SCROLL_GAP;
            push(out, down_tex, self.canvas.centred(dw as f32), y, dw, dh);
        }
        if let Some((tex, w, h)) = self.help {
            let y = OUT_H as f32 - MOUTH_H - h as f32 - DESCRIPTION_GAP;
            push(out, tex, self.canvas.centred(w as f32), y, w, h);
        }
        for (tex, w, x) in centred_hints(self.canvas, self.legends, LEGEND_GAP) {
            push(out, tex, x, LEGEND_Y, w, HINT_H);
        }
    }
}
