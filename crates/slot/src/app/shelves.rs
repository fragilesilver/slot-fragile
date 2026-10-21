//! The Search shelf and the list shelves: X types a search and its games become a shelf, and
//! each text file in Config/Lists is a shelf of the games it names. Ported from
//! nickclyde/slot-plus@02c5890, crates/slot/src/app/shelves.rs. Add to List, the page that
//! puts the cart under the cursor on a list or makes a new one, is Fragile's own.

use std::collections::HashMap;
use std::path::Path;
use std::sync::mpsc::{self, Receiver};

use slot_gfx::Compositor;
use slot_input::{Action, Btn};
use slot_store::collections::{
    add_to_list, hits, in_list, list_name, list_path, read_entries, read_lists, remove_from_list,
    CartKey, Index, LISTS_DIR,
};
use slot_store::{sort_key, CarouselAction, Cart, Platform};
use slot_ui::{
    draw_empty_slot, Draw, Keyboard, KeyboardFaces, ListPage, ListRow, Shelf, TexId, UndoFace,
};

use super::overlay::Overlay;
use super::{App, Phase, Rack, FAVORITES_AT, LINK_LEGEND_Y};

mod search;

pub use search::Search;

const SEARCH_AT: usize = FAVORITES_AT + 1;
const NEW_LIST: &str = "New List";
const LIST_NAME_MAX: usize = 24;

/// The keyboard's keys, its sets of legends (search, a list's name, a network's password, a
/// RetroAchievements username and password) and the Networks page's footer legends, drawn off
/// the main thread.
type Built = (Vec<UndoFace>, [Vec<UndoFace>; 5], Vec<UndoFace>);

/// What the keyboard is typing now and how its page reads: the field, the line under it, the
/// matches, and which legends.
pub(super) struct Typing {
    pub shown: String,
    pub line: String,
    pub matches: Vec<String>,
    pub legend: usize,
}

#[derive(Default)]
pub struct Collections {
    stocked: bool,
    search: Option<Search>,
    search_from: usize,
    last_query: String,
    /// The name being typed for a new list, and the cart that goes on it.
    naming: Option<(Keyboard, CartKey)>,
    /// List names live as long as slot does, so a rack can hold one as it holds a console.
    names: Vec<&'static str>,
    faces: KeyboardFaces,
    /// The keyboard's legends for a search and for a list's name.
    legends: [Vec<(TexId, u32)>; 5],
    /// A Join and X Forget, dark then light.
    pub(super) network_legends: Vec<(TexId, u32)>,
    building: Option<Receiver<Built>>,
    faces_for: Option<(String, String, usize)>,
    match_slots: Vec<Option<TexId>>,
    /// A plain shelf key also sends its button: the press that opened a page is not the
    /// page's.
    skip: Option<Btn>,
}

impl Collections {
    fn intern(&mut self, name: String) -> &'static str {
        if let Some(have) = self.names.iter().find(|n| **n == name) {
            return have;
        }
        let name: &'static str = Box::leak(name.into_boxed_str());
        self.names.push(name);
        name
    }
}

pub(super) fn stock(app: &mut App, root: &Path) {
    let _ = std::fs::create_dir_all(root.join(LISTS_DIR));
    let index = Index::new(app.carts());
    let mut racks = vec![(Rack::Search, Shelf::new(Vec::new()))];
    for list in read_lists(root, &index) {
        let name = app.overlay.shelves.intern(list.name);
        racks.push((Rack::List(name), Shelf::new(carts_for(app, &list.carts))));
    }
    let added = racks.len();
    app.shelves.splice(SEARCH_AT..SEARCH_AT, racks);
    if app.shelf_at >= SEARCH_AT {
        app.shelf_at += added;
    }
    app.overlay.shelves.stocked = true;
}

fn carts_for(app: &App, keys: &[CartKey]) -> Vec<Cart> {
    let by_key: HashMap<(Platform, &str), &Cart> = app
        .carts()
        .map(|c| ((c.platform, c.stem.as_str()), c))
        .collect();
    keys.iter()
        .filter_map(|(p, stem)| by_key.get(&(*p, stem.as_str())))
        .map(|c| (*c).clone())
        .collect()
}

fn go_to(app: &mut App, to: usize) {
    app.shelf_mut().release_hold();
    app.play_held = None;
    app.shelf_at = to;
    app.shelf_platform = slot_ui::Printed::default();
    app.slot_letter = None;
    app.shelf_named = None;
    app.name_pending = true;
}

/// Takes the action when the shelf's search or a list's name is being typed, or when it is
/// one of the shelf keys this module answers. Says whether it took it.
pub(super) fn input(app: &mut App, action: Action) -> bool {
    let c = &mut app.overlay.shelves;
    if !c.stocked || !matches!(app.phase, Phase::Shelf) || app.core_picker.is_some() {
        return false;
    }
    if let Some(b) = c.skip.take() {
        if action == Action::GbaDown(b) {
            return true;
        }
    }
    if lists_open(app) {
        return false;
    }
    let c = &app.overlay.shelves;
    if c.naming.is_some() {
        naming_input(app, action);
        return true;
    }
    if c.search.is_some() {
        search::input(app, action);
        return true;
    }
    let opener = |a: CarouselAction| {
        let b = app.carousel.get(a)?;
        b.modifier
            .is_none()
            .then(|| crate::input::device_btn(b.button))
    };
    match action {
        Action::Search => {
            app.overlay.shelves.skip = opener(CarouselAction::Search);
            search::open(app)
        }
        Action::AddToList => {
            app.overlay.shelves.skip = opener(CarouselAction::AddToList);
            open_lists(app)
        }
        Action::GbaDown(Btn::B) if app.shelf_at == SEARCH_AT => search::clear(app),
        _ => return false,
    }
    true
}

/// While the keyboard or the Add to List page is up, the buttons are the page's, not the
/// shelf's.
pub(super) fn covered(app: &App) -> bool {
    let c = &app.overlay.shelves;
    c.search.is_some() || c.naming.is_some() || lists_open(app)
}

fn lists_open(app: &App) -> bool {
    matches!(app.overlay.open, Some(Overlay::Lists(..)))
}

pub(super) fn search_title(app: &App) -> Option<String> {
    let on_search = app.shelves[app.shelf_at].0 == Rack::Search;
    (on_search && app.slot_letter.is_none()).then(|| quoted(&app.overlay.shelves.last_query))
}

fn quoted(query: &str) -> String {
    format!("\"{query}\"")
}

fn cart_key(app: &App) -> Option<CartKey> {
    let shelf = app.shelf();
    let cart = shelf.carts.get(shelf.index)?;
    Some((cart.platform, cart.stem.clone()))
}

fn list_rows(app: &App, key: &CartKey) -> Vec<ListRow> {
    let mut rows: Vec<ListRow> = app
        .shelves
        .iter()
        .filter_map(|(rack, shelf)| match rack {
            Rack::List(name) => {
                let on = shelf
                    .carts
                    .iter()
                    .any(|c| (c.platform, &c.stem) == (key.0, &key.1));
                Some(ListRow::new(*name).with_value(if on { "Yes" } else { "No" }))
            }
            _ => None,
        })
        .collect();
    rows.push(ListRow::new(NEW_LIST));
    rows
}

fn open_lists(app: &mut App) {
    let Some(key) = cart_key(app) else {
        app.refuse();
        return;
    };
    app.shelf_mut().release_hold();
    app.play_held = None;
    let page = ListPage::new(list_rows(app, &key));
    app.overlay.open = Some(Overlay::Lists(page, key));
    app.overlay.faces.clear();
}

/// A on the Add to List page: puts the cart on the lit list or takes it off, or names a new
/// list. Says whether it was A.
pub(super) fn lists_input(app: &mut App, action: Action) -> bool {
    if action != Action::GbaDown(Btn::A) {
        return false;
    }
    let Some(Overlay::Lists(page, key)) = &app.overlay.open else {
        return false;
    };
    let key = key.clone();
    let Some(label) = page.selected().map(|r| r.label.clone()) else {
        return true;
    };
    if label == NEW_LIST {
        app.overlay.open = None;
        let keyboard = Keyboard::default().with_max(LIST_NAME_MAX);
        app.overlay.shelves.naming = Some((keyboard, key));
        return true;
    }
    toggle(app, &label, &key);
    reopen_lists(app, key, &label);
    true
}

fn reopen_lists(app: &mut App, key: CartKey, on: &str) {
    let rows = list_rows(app, &key);
    let at = rows.iter().position(|r| r.label == on).unwrap_or(0);
    let mut page = ListPage::new(rows);
    page.set_cursor(at);
    app.overlay.open = Some(Overlay::Lists(page, key));
}

fn toggle(app: &mut App, name: &str, key: &CartKey) {
    let Some(root) = app.root.clone() else {
        return;
    };
    let index = Index::new(app.carts());
    let written = match in_list(&root, name, key, &index) {
        true => remove_from_list(&root, name, key, &index),
        false => add_to_list(&root, name, key),
    };
    app.card_writes.wrote("lists", &written);
    if let Err(e) = &written {
        eprintln!("slot: list {name}: {e}");
    }
    restock_list(app, name);
}

/// Reads the list back from the card, so its shelf is what the file says.
fn restock_list(app: &mut App, name: &str) {
    let (Some(root), Some(at)) = (app.root.clone(), list_at(app, name)) else {
        return;
    };
    let index = Index::new(app.carts());
    let keys = hits(&read_entries(&list_path(&root, name), &index));
    let carts = carts_for(app, &keys);
    let here = app.shelf_at == at;
    let cursor = app.shelves[at].1.index;
    app.restock(at, carts);
    let shelf = &mut app.shelves[at].1;
    if here && !shelf.carts.is_empty() {
        shelf.select(cursor.min(shelf.carts.len() - 1));
    }
    app.mark_stars();
}

fn list_at(app: &App, name: &str) -> Option<usize> {
    app.shelves
        .iter()
        .position(|(r, _)| matches!(r, Rack::List(n) if n.eq_ignore_ascii_case(name)))
}

fn naming_input(app: &mut App, action: Action) {
    let Some((keyboard, _)) = app.overlay.shelves.naming.as_mut() else {
        return;
    };
    match action {
        Action::GbaDown(Btn::Up) => keyboard.up(),
        Action::GbaDown(Btn::Down) => keyboard.down(),
        Action::GbaDown(Btn::Left) => keyboard.left(),
        Action::GbaDown(Btn::Right) => keyboard.right(),
        Action::GbaDown(Btn::A) => {
            if keyboard.press() {
                make_list(app);
            }
        }
        Action::GbaDown(Btn::Y | Btn::R1) => keyboard.type_char(' '),
        Action::GbaDown(Btn::X) => keyboard.shift(),
        Action::GbaDown(Btn::L1) => keyboard.delete(),
        Action::GbaDown(Btn::B) if !keyboard.text().is_empty() => keyboard.delete(),
        Action::GbaDown(Btn::B) => {
            if let Some((_, key)) = app.overlay.shelves.naming.take() {
                reopen_lists(app, key, NEW_LIST);
            }
        }
        Action::GbaDown(Btn::Start) => make_list(app),
        Action::QuickMenu => app.overlay.shelves.naming = None,
        _ => {}
    }
}

/// OK on a new list's name: the list is made with the cart on it, gets its shelf among the
/// others, and the Add to List page comes back on it.
fn make_list(app: &mut App) {
    let Some((keyboard, key)) = &app.overlay.shelves.naming else {
        return;
    };
    let (typed, key) = (keyboard.text().to_string(), key.clone());
    let Some(name) = list_name(&typed) else {
        app.refuse();
        return;
    };
    let name = match list_at(app, &name) {
        Some(at) => app.shelves[at].0.name().to_string(),
        None => {
            let rack = Rack::List(app.overlay.shelves.intern(name.clone()));
            let shelf = Shelf::dressed_like(Vec::new(), &app.shelves[0].1);
            let at = app.shelves[SEARCH_AT + 1..]
                .iter()
                .position(|(r, _)| match r {
                    Rack::List(n) => sort_key(n) > sort_key(&name),
                    _ => true,
                })
                .map_or(app.shelves.len(), |i| SEARCH_AT + 1 + i);
            app.shelves.insert(at, (rack, shelf));
            if app.shelf_at >= at {
                app.shelf_at += 1;
            }
            if app.overlay.shelves.search_from >= at {
                app.overlay.shelves.search_from += 1;
            }
            name
        }
    };
    app.overlay.shelves.naming = None;
    if let Some(root) = app.root.clone() {
        let index = Index::new(app.carts());
        if !in_list(&root, &name, &key, &index) {
            let written = add_to_list(&root, &name, &key);
            app.card_writes.wrote("lists", &written);
            if let Err(e) = &written {
                eprintln!("slot: list {name}: {e}");
            }
        }
    }
    restock_list(app, &name);
    reopen_lists(app, key, &name);
}

pub(super) fn draw(app: &App, out: &mut Vec<Draw>) {
    if app.draw_network_keyboard(out) {
        return;
    }
    if !matches!(app.phase, Phase::Shelf) {
        return;
    }
    let c = &app.overlay.shelves;
    let keyboard = match (&c.search, &c.naming) {
        (Some(search), _) => Some(&search.keyboard),
        (None, Some((keyboard, _))) => Some(keyboard),
        (None, None) => None,
    };
    if let Some(keyboard) = keyboard {
        let shake = app.shake_when(true, app.now());
        keyboard.draw(app.canvas(), &c.faces, shake, out);
        return;
    }
    if lists_open(app) {
        draw_lists(app, out);
    }
}

/// The Add to List page, laid out as a menu page over the shelf: Back on the left and Set on
/// the right.
fn draw_lists(app: &App, out: &mut Vec<Draw>) {
    slot_ui::draw_menu_ground(app.canvas(), app.menu_ground(), out);
    app.draw_overlay(out);
    draw_empty_slot(app.canvas(), app.slot_faces, out);
    let Some(faces) = &app.quick_menu_faces else {
        return;
    };
    let legend = [faces.legend, faces.legend_light][app.light() as usize];
    app.draw_side_legend(legend[0], false, LINK_LEGEND_Y, out);
    app.draw_side_legend(legend[5], true, LINK_LEGEND_Y, out);
}

pub fn sync_faces(app: &mut App, compositor: &mut Compositor) {
    let c = &mut app.overlay.shelves;
    if !c.stocked {
        return;
    }
    if c.faces.keys.is_empty() {
        match &c.building {
            None => {
                let (tx, rx) = mpsc::channel();
                let spawned = std::thread::Builder::new()
                    .name("slot-keyboard".into())
                    .spawn(move || {
                        let network: Vec<UndoFace> = [false, true]
                            .into_iter()
                            .flat_map(|light| {
                                slot_ui::network_legend_faces(slot_ui::footer_ink(light))
                            })
                            .collect();
                        let _ = tx.send((
                            slot_ui::keyboard_key_faces(),
                            [
                                slot_ui::keyboard_legend_faces("Search"),
                                slot_ui::keyboard_legend_faces("Make List"),
                                slot_ui::keyboard_legend_faces("Join"),
                                slot_ui::keyboard_legend_faces("Next"),
                                slot_ui::keyboard_legend_faces("Sign In"),
                            ],
                            network,
                        ));
                    });
                if spawned.is_ok() {
                    c.building = Some(rx);
                }
            }
            Some(rx) => {
                if let Ok((keys, legends, network)) = rx.try_recv() {
                    c.network_legends = network
                        .iter()
                        .map(|f| (compositor.create_texture(f.w, f.h, &f.rgba), f.w))
                        .collect();
                    c.faces.keys = keys
                        .iter()
                        .map(|f| (compositor.create_texture(f.w, f.h, &f.rgba), f.w, f.h))
                        .collect();
                    c.legends = legends.map(|set| {
                        set.iter()
                            .map(|f| (compositor.create_texture(f.w, f.h, &f.rgba), f.w))
                            .collect()
                    });
                    c.faces_for = None;
                }
            }
        }
    }
    let typing = match (&c.search, &c.naming) {
        (Some(search), _) => Some(Typing {
            shown: search.keyboard.shown(),
            line: search.count_text(),
            matches: search.shown(),
            legend: 0,
        }),
        (None, Some((keyboard, _))) => Some(Typing {
            shown: keyboard.shown(),
            line: "Name the new list".to_string(),
            matches: Vec::new(),
            legend: 1,
        }),
        (None, None) => None,
    };
    let Some(Typing {
        shown,
        line: count,
        matches: lines,
        legend,
    }) = typing.or_else(|| app.network_typing())
    else {
        app.overlay.shelves.faces_for = None;
        return;
    };
    let c = &mut app.overlay.shelves;
    let wanted = (shown.clone(), count.clone(), legend);
    if c.faces_for.as_ref() == Some(&wanted) {
        return;
    }
    c.faces_for = Some(wanted);
    c.faces.legend = c.legends[legend].clone();
    let query = slot_ui::keyboard_query_face(&shown);
    c.faces.query = Some(put(compositor, c.faces.query.map(|q| q.0), query));
    let count = slot_ui::keyboard_line_face(&count);
    c.faces.count = Some(put(compositor, c.faces.count.map(|q| q.0), count));
    c.match_slots.resize(slot_ui::SHOWN_MATCHES, None);
    c.faces.matches.clear();
    for (slot, line) in c.match_slots.iter_mut().zip(lines) {
        let placed = put(compositor, *slot, slot_ui::keyboard_line_face(&line));
        *slot = Some(placed.0);
        c.faces.matches.push(placed);
    }
}

fn put(compositor: &mut Compositor, slot: Option<TexId>, face: UndoFace) -> (TexId, u32, u32) {
    let tex = match slot {
        Some(tex) => {
            compositor.update_texture(tex, face.w, face.h, &face.rgba);
            tex
        }
        None => compositor.create_texture(face.w, face.h, &face.rgba),
    };
    (tex, face.w, face.h)
}

impl App {
    /// The names of the shelves that have carts, in order, as their slots say them.
    pub fn shelf_names(&self) -> Vec<String> {
        self.shelves
            .iter()
            .filter(|(_, s)| !s.carts.is_empty())
            .map(|(rack, _)| match rack {
                Rack::Search => quoted(&self.overlay.shelves.last_query),
                _ => rack.name().to_string(),
            })
            .collect()
    }

    pub fn shelf_stems(&self) -> Vec<String> {
        self.shelf().carts.iter().map(|c| c.stem.clone()).collect()
    }

    /// The keyboard's faces, for every page that types: search, a list's name, a password.
    pub(super) fn keyboard_faces(&self) -> &KeyboardFaces {
        &self.overlay.shelves.faces
    }

    pub fn search(&self) -> Option<&Search> {
        self.overlay.shelves.search.as_ref()
    }

    /// The new list's name while it is being typed.
    pub fn list_naming(&self) -> Option<&str> {
        self.overlay.shelves.naming.as_ref().map(|(k, _)| k.text())
    }

    pub fn list_keyboard_key(&self) -> Option<slot_ui::Key> {
        self.overlay.shelves.naming.as_ref().map(|(k, _)| k.key())
    }
}
