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
    add_to_list, delete_list, hits, in_list, list_name, list_path, read_entries, read_lists,
    remove_from_list, rename_list, CartKey, Index, LISTS_DIR,
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

#[derive(Clone, Debug, PartialEq, Eq)]
enum Naming {
    New(CartKey),
    Rename { old: String, key: CartKey },
}

impl Naming {
    fn key(&self) -> &CartKey {
        match self {
            Naming::New(key) | Naming::Rename { key, .. } => key,
        }
    }
}

/// The keyboard's keys, its sets of legends (search, a list's name, a network's password, a
/// RetroAchievements username and password) and the Networks and Add to List pages' footer
/// legends, drawn off the main thread.
type Built = (
    Vec<UndoFace>,
    [Vec<UndoFace>; 5],
    Vec<UndoFace>,
    Vec<UndoFace>,
);

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
    /// A list's name being typed: a new one, with the cart that goes on it, or a new name for
    /// one. The cart is the one the Add to List page was opened for.
    naming: Option<(Keyboard, Naming)>,
    /// The list whose Delete was pressed once; a second Delete removes it.
    deleting: Option<String>,
    /// X Rename and Y Delete for the Add to List page, dark then light.
    list_legends: Vec<(TexId, u32)>,
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
    let deleting = app.overlay.shelves.deleting.as_deref();
    let mut rows: Vec<ListRow> = app
        .shelves
        .iter()
        .filter_map(|(rack, shelf)| match rack {
            Rack::List(name) => {
                let on = shelf
                    .carts
                    .iter()
                    .any(|c| (c.platform, &c.stem) == (key.0, &key.1));
                let value = match (deleting == Some(*name), on) {
                    (true, _) => "Y again to delete",
                    (false, true) => "Yes",
                    (false, false) => "No",
                };
                Some(ListRow::new(*name).with_value(value))
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

/// The Add to List page's own buttons: A puts the cart on the lit list or takes it off, or
/// names a new list; X renames the lit list; Y twice deletes it (anything else between
/// cancels). Says whether it took the action.
pub(super) fn lists_input(app: &mut App, action: Action) -> bool {
    let Some(Overlay::Lists(page, key)) = &app.overlay.open else {
        return false;
    };
    let key = key.clone();
    let label = page.selected().map(|r| r.label.clone());
    let on_list = label.as_deref().is_some_and(|l| l != NEW_LIST);
    let pressed = matches!(action, Action::GbaDown(_) | Action::QuickMenu);
    let cancel = pressed && action != Action::GbaDown(Btn::Y);
    if cancel && app.overlay.shelves.deleting.take().is_some() {
        reopen_lists(app, key.clone(), label.as_deref().unwrap_or(NEW_LIST));
    }
    let Some(label) = label else {
        return false;
    };
    match action {
        Action::GbaDown(Btn::A) if label == NEW_LIST => {
            app.overlay.open = None;
            let keyboard = Keyboard::default().with_max(LIST_NAME_MAX);
            app.overlay.shelves.naming = Some((keyboard, Naming::New(key)));
        }
        Action::GbaDown(Btn::A) => {
            toggle(app, &label, &key);
            reopen_lists(app, key, &label);
        }
        Action::GbaDown(Btn::X) if on_list => {
            app.overlay.open = None;
            let keyboard = Keyboard::default()
                .with_max(LIST_NAME_MAX)
                .with_text(&label);
            let naming = Naming::Rename { old: label, key };
            app.overlay.shelves.naming = Some((keyboard, naming));
        }
        Action::GbaDown(Btn::Y) if on_list => match app.overlay.shelves.deleting.take() {
            Some(armed) if armed == label => delete(app, &label, key),
            _ => {
                app.overlay.shelves.deleting = Some(label.clone());
                reopen_lists(app, key, &label);
            }
        },
        _ => return false,
    }
    true
}

/// Removes the list's file and its shelf, and the page comes back on the row after it.
fn delete(app: &mut App, name: &str, key: CartKey) {
    let (Some(root), Some(at)) = (app.root.clone(), list_at(app, name)) else {
        return;
    };
    let written = delete_list(&root, name);
    app.card_writes.wrote("lists", &written);
    if let Err(e) = written {
        eprintln!("slot: list {name}: {e}");
        return app.refuse();
    }
    let row = app.overlay_page().map_or(0, |p| p.cursor());
    let (rack, _) = take_rack(app, at);
    app.extras.set(&super::extras::mark_key(rack), None);
    app.save_extras();
    let mut page = ListPage::new(list_rows(app, &key));
    page.set_cursor(row.min(page.rows().len().saturating_sub(1)));
    app.overlay.open = Some(Overlay::Lists(page, key));
}

/// Takes a rack out, keeping the shelf being shown and where a search came from pointing at
/// the same shelves; when the one shown goes, the next with games is shown.
fn take_rack(app: &mut App, at: usize) -> (Rack, Shelf) {
    let taken = app.shelves.remove(at);
    let from = &mut app.overlay.shelves.search_from;
    if *from > at || *from >= app.shelves.len() {
        *from = from.saturating_sub(1);
    }
    if app.shelf_at > at {
        app.shelf_at -= 1;
    } else if app.shelf_at == at {
        let here = at.min(app.shelves.len() - 1);
        app.shelf_at = here;
        let to = match app.shelves[here].1.carts.is_empty() {
            true => app.next_shelf(1).unwrap_or(here),
            false => here,
        };
        go_to(app, to);
    }
    taken
}

/// Puts a list's rack among the others in name order, and says where.
fn put_rack(app: &mut App, name: &str, shelf: Shelf) -> usize {
    let rack = Rack::List(app.overlay.shelves.intern(name.to_string()));
    let at = app.shelves[SEARCH_AT + 1..]
        .iter()
        .position(|(r, _)| match r {
            Rack::List(n) => sort_key(n) > sort_key(name),
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
    at
}

/// OK on a list's new name: its file and shelf take the name, the shelf keeps its place in
/// name order and its remembered cart, and the page comes back on it.
fn rename(app: &mut App, typed: &str, old: &str, key: CartKey) {
    let Some(name) = list_name(typed) else {
        return app.refuse();
    };
    let at = list_at(app, old);
    let taken = list_at(app, &name).is_some_and(|other| Some(other) != at);
    let (Some(root), Some(at), false) = (app.root.clone(), at, taken) else {
        return app.refuse();
    };
    let written = rename_list(&root, old, &name);
    app.card_writes.wrote("lists", &written);
    if let Err(e) = written {
        eprintln!("slot: list {old}: {e}");
        return app.refuse();
    }
    app.overlay.shelves.naming = None;
    let shown = app.shelf_at == at;
    let (rack, shelf) = app.shelves.remove(at);
    if app.shelf_at > at {
        app.shelf_at -= 1;
    }
    if app.overlay.shelves.search_from > at {
        app.overlay.shelves.search_from -= 1;
    }
    let mark = app.extras.cart(&super::extras::mark_key(rack));
    app.extras.set(&super::extras::mark_key(rack), None);
    let to = put_rack(app, &name, shelf);
    if shown {
        go_to(app, to);
    }
    let new_mark = super::extras::mark_key(app.shelves[to].0);
    app.extras.set_cart(&new_mark, mark.as_ref());
    app.save_extras();
    reopen_lists(app, key, &name);
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
                name_typed(app);
            }
        }
        Action::GbaDown(Btn::Y | Btn::R1) => keyboard.type_char(' '),
        Action::GbaDown(Btn::X) => keyboard.shift(),
        Action::GbaDown(Btn::L1) => keyboard.delete(),
        Action::GbaDown(Btn::B) if !keyboard.text().is_empty() => keyboard.delete(),
        Action::GbaDown(Btn::B) => match app.overlay.shelves.naming.take() {
            Some((_, Naming::New(key))) => reopen_lists(app, key, NEW_LIST),
            Some((_, Naming::Rename { old, key })) => reopen_lists(app, key, &old),
            None => {}
        },
        Action::GbaDown(Btn::Start) => name_typed(app),
        Action::QuickMenu => app.overlay.shelves.naming = None,
        _ => {}
    }
}

/// OK on the keyboard: a new list is made, or the list is renamed.
fn name_typed(app: &mut App) {
    let Some((keyboard, naming)) = &app.overlay.shelves.naming else {
        return;
    };
    let typed = keyboard.text().to_string();
    match naming.clone() {
        Naming::New(_) => make_list(app),
        Naming::Rename { old, key } => rename(app, &typed, &old, key),
    }
}

/// OK on a new list's name: the list is made with the cart on it, gets its shelf among the
/// others, and the Add to List page comes back on it.
fn make_list(app: &mut App) {
    let Some((keyboard, naming)) = &app.overlay.shelves.naming else {
        return;
    };
    let (typed, key) = (keyboard.text().to_string(), naming.key().clone());
    let Some(name) = list_name(&typed) else {
        app.refuse();
        return;
    };
    let name = match list_at(app, &name) {
        Some(at) => app.shelves[at].0.name().to_string(),
        None => {
            let shelf = Shelf::dressed_like(Vec::new(), &app.shelves[0].1);
            put_rack(app, &name, shelf);
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

/// The Add to List page, laid out as a menu page over the shelf: Back, then Rename and
/// Delete on a list, then Set.
fn draw_lists(app: &App, out: &mut Vec<Draw>) {
    slot_ui::draw_menu_ground(app.canvas(), app.menu_ground(), out);
    app.draw_overlay(out);
    let on_list = app
        .overlay_page()
        .and_then(|p| p.selected())
        .is_some_and(|r| r.label != NEW_LIST);
    // Four legends lie across the middle, so the slot is drawn plugged as under a binding row.
    match on_list {
        true => slot_ui::draw_plugged_slot(app.canvas(), app.plugged_face, out),
        false => draw_empty_slot(app.canvas(), app.slot_faces, out),
    }
    let Some(faces) = &app.quick_menu_faces else {
        return;
    };
    let light = app.light() as usize;
    let legend = [faces.legend, faces.legend_light][light];
    let made = &app.overlay.shelves.list_legends;
    let mut legends = vec![legend[0]];
    if let (true, Some(&rename), Some(&delete)) =
        (on_list, made.get(light * 2), made.get(light * 2 + 1))
    {
        legends.extend([rename, delete]);
    }
    legends.push(legend[5]);
    app.draw_spread_legends(&legends, LINK_LEGEND_Y, out);
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
                        let lists: Vec<UndoFace> = [false, true]
                            .into_iter()
                            .flat_map(|light| {
                                slot_ui::list_legend_faces(slot_ui::footer_ink(light))
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
                            lists,
                        ));
                    });
                if spawned.is_ok() {
                    c.building = Some(rx);
                }
            }
            Some(rx) => {
                if let Ok((keys, legends, network, lists)) = rx.try_recv() {
                    c.list_legends = lists
                        .iter()
                        .map(|f| (compositor.create_texture(f.w, f.h, &f.rgba), f.w))
                        .collect();
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
        (None, Some((keyboard, naming))) => Some(Typing {
            shown: keyboard.shown(),
            line: match naming {
                Naming::New(_) => "Name the new list".to_string(),
                Naming::Rename { old, .. } => format!("New name for {old}"),
            },
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
