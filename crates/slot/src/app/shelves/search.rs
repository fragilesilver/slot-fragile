//! Ported from nickclyde/slot-plus@02c5890, crates/slot/src/app/shelves/search.rs, on
//! Fragile's keyboard, which keeps the typed text and has Shift.

use slot_input::{Action, Btn};
use slot_store::find::{fold, matches, Folded};
use slot_store::{sort_key, Cart};
use slot_ui::{clean_label, Keyboard, SHOWN_MATCHES};

use super::{go_to, App, SEARCH_AT};

const QUERY_MAX: usize = 24;

pub struct Search {
    pub keyboard: Keyboard,
    pool: Vec<Cart>,
    titles: Vec<Folded>,
    hits: Vec<usize>,
}

impl Search {
    fn new(mut pool: Vec<Cart>, query: &str) -> Self {
        pool.sort_by(|a, b| {
            (sort_key(&a.stem), a.platform as u8).cmp(&(sort_key(&b.stem), b.platform as u8))
        });
        let titles = pool.iter().map(|c| fold(&c.stem)).collect();
        let mut search = Search {
            keyboard: Keyboard::default().with_max(QUERY_MAX).with_text(query),
            pool,
            titles,
            hits: Vec::new(),
        };
        search.refresh();
        search
    }

    pub fn query(&self) -> &str {
        self.keyboard.text()
    }

    fn refresh(&mut self) {
        self.hits = matches(&self.titles, &fold(self.query()));
    }

    pub fn stems(&self) -> Vec<&str> {
        self.hits
            .iter()
            .map(|i| self.pool[*i].stem.as_str())
            .collect()
    }

    pub fn count_text(&self) -> String {
        match self.hits.len() {
            _ if fold(self.query()).is_empty() => String::new(),
            0 => "No games".into(),
            1 => "1 game".into(),
            n => format!("{n} games"),
        }
    }

    pub fn shown(&self) -> Vec<String> {
        self.hits
            .iter()
            .take(SHOWN_MATCHES)
            .map(|i| clean_label(&self.pool[*i].stem))
            .collect()
    }

    fn carts(&self) -> Vec<Cart> {
        self.hits.iter().map(|i| self.pool[*i].clone()).collect()
    }
}

pub(super) fn open(app: &mut App) {
    let query = match app.shelf_at == SEARCH_AT {
        true => app.overlay.shelves.last_query.clone(),
        false => String::new(),
    };
    let pool = app.carts().cloned().collect();
    app.shelf_mut().release_hold();
    app.play_held = None;
    app.overlay.shelves.search = Some(Search::new(pool, &query));
}

fn close(app: &mut App) {
    app.overlay.shelves.search = None;
}

pub(super) fn input(app: &mut App, action: Action) {
    let Some(search) = app.overlay.shelves.search.as_mut() else {
        return;
    };
    let keyboard = &mut search.keyboard;
    match action {
        Action::GbaDown(Btn::Up) => keyboard.up(),
        Action::GbaDown(Btn::Down) => keyboard.down(),
        Action::GbaDown(Btn::Left) => keyboard.left(),
        Action::GbaDown(Btn::Right) => keyboard.right(),
        Action::GbaDown(Btn::A) => {
            if keyboard.press() {
                return confirm(app);
            }
        }
        Action::GbaDown(Btn::Y | Btn::R1) => keyboard.type_char(' '),
        Action::GbaDown(Btn::X) => keyboard.shift(),
        Action::GbaDown(Btn::L1) => keyboard.delete(),
        Action::GbaDown(Btn::B) if keyboard.text().is_empty() => return close(app),
        Action::GbaDown(Btn::B) => keyboard.delete(),
        Action::GbaDown(Btn::Start) => return confirm(app),
        Action::QuickMenu => return close(app),
        _ => return,
    }
    search.refresh();
}

fn confirm(app: &mut App) {
    let Some(search) = &app.overlay.shelves.search else {
        return;
    };
    if search.hits.is_empty() {
        app.refuse();
        return;
    }
    let carts = search.carts();
    let query = search.query().trim().to_string();
    if app.shelf_at != SEARCH_AT {
        app.overlay.shelves.search_from = app.shelf_at;
    }
    app.restock(SEARCH_AT, carts);
    app.shelves[SEARCH_AT].1.select(0);
    app.mark_stars();
    app.overlay.shelves.last_query = query;
    close(app);
    go_to(app, SEARCH_AT);
}

pub(super) fn clear(app: &mut App) {
    let back = app.overlay.shelves.search_from;
    let to = Some(back)
        .filter(|at| !app.shelves[*at].1.carts.is_empty())
        .or_else(|| app.next_shelf(1));
    if let Some(to) = to {
        go_to(app, to);
    }
    app.restock(SEARCH_AT, Vec::new());
    app.overlay.shelves.last_query.clear();
}
