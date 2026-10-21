// Ported from nickclyde/slot-plus, crates/slot-ui/src/keyboard.rs; adds lower case, symbols,
// a Shift key, an owned entry with a length cap, and masking for passwords.

use crate::draw::{Canvas, Draw, TexId, OUT_H};
use crate::plate::{centred_hints, hint_face, title_face, word_face, UndoFace, HINT_H, LEGEND_GAP};
use crate::power_menu::{menu_face, MENU_H, MENU_INK, MENU_PAD, MENU_PX};
use crate::slot_chrome::{edge, opening, recess};

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Key {
    Char(char),
    Shift,
    Space,
    Delete,
    Done,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug, Default)]
pub enum Layer {
    #[default]
    Upper,
    Lower,
    Symbols,
}

impl Layer {
    pub const ALL: [Layer; 3] = [Layer::Upper, Layer::Lower, Layer::Symbols];

    fn next(self) -> Layer {
        match self {
            Layer::Upper => Layer::Lower,
            Layer::Lower => Layer::Symbols,
            Layer::Symbols => Layer::Upper,
        }
    }

    fn rows(self) -> [&'static str; 4] {
        match self {
            Layer::Upper => ["1234567890", "QWERTYUIOP", "ASDFGHJKL-", "ZXCVBNM._@"],
            Layer::Lower => ["1234567890", "qwertyuiop", "asdfghjkl-", "zxcvbnm._@"],
            Layer::Symbols => ["1234567890", "!?#$%&*+=/", "()[]{}<>\\|", "'\":;,~^`_-"],
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Layer::Upper => "ABC",
            Layer::Lower => "abc",
            Layer::Symbols => "#+=",
        }
    }
}

pub const COLS: usize = 10;
const ROWS: usize = 5;
/// The bottom row is the same on every layer: Shift, Space, Delete, OK.
const BOTTOM: [Key; COLS] = [
    Key::Shift,
    Key::Shift,
    Key::Space,
    Key::Space,
    Key::Space,
    Key::Space,
    Key::Delete,
    Key::Delete,
    Key::Done,
    Key::Done,
];
pub const KEY_W: f32 = 56.0;
pub const KEY_H: f32 = 44.0;
pub const KEY_GAP: f32 = 6.0;
const KEYS_W: f32 = COLS as f32 * (KEY_W + KEY_GAP) - KEY_GAP;
pub const KEYS_Y: f32 = 150.0;
pub const FIELD_Y: f32 = 26.0;
pub const FIELD_H: f32 = 48.0;
const FIELD_W: f32 = 560.0;
const COUNT_Y: f32 = 82.0;
const MATCH_Y: f32 = 112.0;
const MATCH_PITCH: f32 = 28.0;
pub const SHOWN_MATCHES: usize = 1;
pub const KEYBOARD_LEGEND_Y: f32 = 427.0;
const MASK: char = '•';

fn key_at(layer: Layer, row: usize, col: usize) -> Key {
    match row {
        4 => BOTTOM[col],
        _ => Key::Char(layer.rows()[row].chars().nth(col).unwrap_or(' ')),
    }
}

fn span(row: usize, col: usize) -> (usize, usize) {
    if row != 4 {
        return (col, col);
    }
    let key = BOTTOM[col];
    let start = (0..=col).rev().take_while(|c| BOTTOM[*c] == key).last();
    let end = (col..COLS).take_while(|c| BOTTOM[*c] == key).last();
    (start.unwrap_or(col), end.unwrap_or(col))
}

/// Every key on every layer, once each, in a fixed order the faces are built in.
pub fn all_keys() -> Vec<Key> {
    let mut keys = Vec::new();
    for layer in Layer::ALL {
        for row in 0..ROWS {
            for col in 0..COLS {
                let key = key_at(layer, row, col);
                if !keys.contains(&key) {
                    keys.push(key);
                }
            }
        }
    }
    keys
}

pub fn key_text(key: Key) -> String {
    match key {
        Key::Char(c) => c.to_string(),
        Key::Shift => "Shift".into(),
        Key::Space => "Space".into(),
        Key::Delete => "Del".into(),
        Key::Done => "OK".into(),
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Keyboard {
    row: usize,
    col: usize,
    home: usize,
    layer: Layer,
    text: String,
    max: usize,
    masked: bool,
}

impl Default for Keyboard {
    fn default() -> Self {
        Keyboard {
            row: 1,
            col: 0,
            home: 0,
            layer: Layer::Upper,
            text: String::new(),
            max: 63,
            masked: false,
        }
    }
}

impl Keyboard {
    /// A keyboard for a secret, typed in lower case first and shown as dots.
    pub fn password(max: usize) -> Self {
        Keyboard {
            layer: Layer::Lower,
            max,
            masked: true,
            ..Keyboard::default()
        }
    }

    pub fn with_max(mut self, max: usize) -> Self {
        self.max = max;
        self.text = self.text.chars().take(max).collect();
        self
    }

    pub fn with_text(mut self, text: &str) -> Self {
        self.text = text.chars().take(self.max).collect();
        self
    }

    pub fn key(&self) -> Key {
        key_at(self.layer, self.row, self.col)
    }

    pub fn layer(&self) -> Layer {
        self.layer
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    /// What the field shows: dots for a password, with the last letter left readable.
    pub fn shown(&self) -> String {
        if !self.masked {
            return self.text.clone();
        }
        let n = self.text.chars().count();
        let last = self.text.chars().last();
        (0..n.saturating_sub(1)).map(|_| MASK).chain(last).collect()
    }

    pub fn at(&self) -> (usize, usize) {
        (self.row, span(self.row, self.col).0)
    }

    pub fn up(&mut self) {
        self.row = (self.row + ROWS - 1) % ROWS;
        self.col = self.home;
    }

    pub fn down(&mut self) {
        self.row = (self.row + 1) % ROWS;
        self.col = self.home;
    }

    pub fn left(&mut self) {
        let (start, _) = span(self.row, self.col);
        let col = (start + COLS - 1) % COLS;
        self.col = span(self.row, col).0;
        self.home = self.col;
    }

    pub fn right(&mut self) {
        let (_, end) = span(self.row, self.col);
        self.col = (end + 1) % COLS;
        self.home = self.col;
    }

    pub fn type_char(&mut self, c: char) {
        if self.text.chars().count() < self.max {
            self.text.push(c);
        }
    }

    pub fn delete(&mut self) {
        self.text.pop();
    }

    pub fn shift(&mut self) {
        self.layer = self.layer.next();
    }

    /// Press the lit key. True once OK is pressed and the entry is finished.
    pub fn press(&mut self) -> bool {
        match self.key() {
            Key::Char(c) => self.type_char(c),
            Key::Space => self.type_char(' '),
            Key::Delete => self.delete(),
            Key::Shift => self.shift(),
            Key::Done => return true,
        }
        false
    }

    pub fn key_rect(canvas: Canvas, row: usize, col: usize) -> (f32, f32, f32, f32) {
        let (start, end) = span(row, col);
        let wide = (end - start + 1) as f32;
        (
            canvas.centred(KEYS_W).round() + start as f32 * (KEY_W + KEY_GAP),
            KEYS_Y + row as f32 * (KEY_H + KEY_GAP),
            wide * KEY_W + (wide - 1.0) * KEY_GAP,
            KEY_H,
        )
    }
}

pub fn keyboard_key_faces() -> Vec<UndoFace> {
    all_keys()
        .into_iter()
        .map(|k| match k {
            Key::Char(_) => cased_face(&key_text(k)),
            _ => word_face(&key_text(k)),
        })
        .collect()
}

/// A menu face that keeps the letters as typed, so a lower-case key and a password read true.
/// The menu's own faces are all capitals. Fragile's.
fn cased_face(text: &str) -> UndoFace {
    let Some(font) = crate::text::label_font() else {
        return menu_face(text);
    };
    let ink = crate::text::line_width(font, text, MENU_PX, 0.0).ceil() as u32;
    let w = ink + 2 * MENU_PAD;
    let mut rgba = vec![0u8; (w * MENU_H * 4) as usize];
    let layout = crate::text::fit_box_cased(
        font,
        text,
        w as f32,
        MENU_H as f32,
        1,
        MENU_PX,
        MENU_PX / 2.0,
    );
    crate::text::draw_centred(&mut rgba, w, MENU_H, &layout, MENU_INK);
    UndoFace { rgba, w, h: MENU_H }
}

/// Type, Delete, Space, and the word for finishing, which the caller names.
pub fn keyboard_legend_faces(done: &str) -> Vec<UndoFace> {
    vec![
        hint_face("A", "Type"),
        hint_face("B", "Delete"),
        hint_face("Y", "Space"),
        hint_face("X", "Shift"),
        hint_face("START", done),
    ]
}

pub fn keyboard_query_face(shown: &str) -> UndoFace {
    cased_face(&format!("{shown}_"))
}

pub fn keyboard_line_face(text: &str) -> UndoFace {
    title_face(text)
}

#[derive(Default)]
pub struct KeyboardFaces {
    pub keys: Vec<(TexId, u32, u32)>,
    pub legend: Vec<(TexId, u32)>,
    pub query: Option<(TexId, u32, u32)>,
    pub count: Option<(TexId, u32, u32)>,
    pub matches: Vec<(TexId, u32, u32)>,
}

impl Keyboard {
    pub fn draw(&self, canvas: Canvas, faces: &KeyboardFaces, shake: f32, out: &mut Vec<Draw>) {
        out.push(Draw::Rect {
            x: 0.0,
            y: 0.0,
            w: canvas.w,
            h: OUT_H as f32,
            colour: opening(),
        });
        out.push(Draw::Rect {
            x: (canvas.centred(FIELD_W) + shake).round(),
            y: FIELD_Y,
            w: FIELD_W,
            h: FIELD_H,
            colour: recess(),
        });
        if let Some((tex, w, h)) = faces.query {
            let y = FIELD_Y + (FIELD_H - h as f32) / 2.0;
            push(out, tex, canvas.centred(w as f32) + shake, y, w as f32, h);
        }
        if let Some((tex, w, h)) = faces.count {
            push(out, tex, canvas.centred(w as f32), COUNT_Y, w as f32, h);
        }
        for (i, &(tex, w, h)) in faces.matches.iter().take(SHOWN_MATCHES).enumerate() {
            let y = MATCH_Y + i as f32 * MATCH_PITCH;
            push(out, tex, canvas.centred(w as f32), y, w as f32, h);
        }
        let lit = self.at();
        let keys = all_keys();
        for row in 0..ROWS {
            let mut col = 0;
            while col < COLS {
                let (x, y, w, h) = Keyboard::key_rect(canvas, row, col);
                let colour = if (row, col) == lit { edge() } else { recess() };
                out.push(Draw::Rect { x, y, w, h, colour });
                let key = key_at(self.layer, row, col);
                let face = keys
                    .iter()
                    .position(|k| *k == key)
                    .and_then(|i| faces.keys.get(i));
                if let Some(&(tex, fw, fh)) = face {
                    let fw = fw as f32;
                    push(
                        out,
                        tex,
                        x + (w - fw) / 2.0,
                        y + (h - fh as f32) / 2.0,
                        fw,
                        fh,
                    );
                }
                col = span(row, col).1 + 1;
            }
        }
        for (tex, w, x) in centred_hints(canvas, &faces.legend, LEGEND_GAP) {
            push(out, tex, x, KEYBOARD_LEGEND_Y, w as f32, HINT_H);
        }
    }
}

fn push(out: &mut Vec<Draw>, tex: TexId, x: f32, y: f32, w: f32, h: u32) {
    out.push(Draw::Tex {
        x: x.round(),
        y: y.round(),
        w,
        h: h as f32,
        tex,
        alpha: 1.0,
    });
}
