//! The in-game Cheats page: the playing game's cheats from `Cheats/`, A turns one on or off,
//! and the codes that are on go to the core. Hidden when the game has none, and the core gets
//! none while two consoles are linked, so the two never drift apart. The page and its rules
//! are Fragile's; the files are read by `crate::cheats` (roygravy's).

use slot_input::{Action, Btn};
use slot_store::Platform;
use slot_ui::{ListPage, ListRow};

use super::overlay::Overlay;
use super::App;
use crate::cheats::Cheat;

#[derive(Default)]
pub struct Cheats {
    list: Vec<Cheat>,
    /// The game the list was read for.
    game: Option<(Platform, String)>,
    /// The codes the core was last given, so a change, or a link starting or ending, sends
    /// them again; `None` until the running core has had any.
    sent: Option<Vec<String>>,
}

fn rows(list: &[Cheat]) -> Vec<ListRow> {
    list.iter()
        .map(|c| ListRow::new(c.name.as_str()).with_value(if c.on { "On" } else { "Off" }))
        .collect()
}

impl App {
    fn cheats_game(&self) -> Option<(Platform, String)> {
        let cart = self.seated_cart()?;
        Some((cart.platform, cart.stem.clone()))
    }

    /// A new core is up: read its game's cheats and hand it the ones left on.
    pub(super) fn cheats_core_ready(&mut self) {
        let game = self.cheats_game();
        let c = &mut self.overlay.cheats;
        c.sent = None;
        if c.game == game {
            return;
        }
        c.list = match (self.root.as_deref(), &game) {
            (Some(root), Some((platform, stem))) => crate::cheats::load(root, *platform, stem),
            _ => Vec::new(),
        };
        c.game = game;
    }

    /// The Cheats row shows for a game that has cheats, while it is not linked.
    pub(super) fn cheats_hidden(&self) -> bool {
        let c = &self.overlay.cheats;
        c.list.is_empty() || c.game != self.cheats_game() || self.link_active()
    }

    /// The codes for the core when they differ from what it has: the ones on, or none while
    /// linked. The session hands them over.
    pub fn take_cheats(&mut self) -> Option<Vec<String>> {
        let linked = self.link_active();
        let c = &mut self.overlay.cheats;
        let want = match linked {
            true => Vec::new(),
            false => crate::cheats::codes(&c.list),
        };
        if c.sent.as_ref() == Some(&want) {
            return None;
        }
        c.sent = Some(want.clone());
        Some(want)
    }

    pub(super) fn open_cheats(&mut self) {
        let page = ListPage::new(rows(&self.overlay.cheats.list));
        self.overlay.open = Some(Overlay::Cheats(page));
        self.overlay.faces.clear();
    }

    /// A on the Cheats page turns the lit cheat on or off and remembers it. Says whether it
    /// took the action.
    pub(super) fn cheats_input(&mut self, action: Action) -> bool {
        if action != Action::GbaDown(Btn::A) {
            return false;
        }
        let Some(Overlay::Cheats(page)) = &mut self.overlay.open else {
            return false;
        };
        let at = page.cursor();
        let c = &mut self.overlay.cheats;
        let Some(cheat) = c.list.get_mut(at) else {
            return true;
        };
        cheat.on = !cheat.on;
        page.set_rows(rows(&c.list));
        if let (Some(root), Some((platform, stem))) = (self.root.as_deref(), &c.game) {
            crate::cheats::save(root, *platform, stem, &c.list);
        }
        true
    }

    /// The names of the playing game's cheats that are on, for tests and the row's value.
    pub fn cheats_on(&self) -> Vec<String> {
        let c = &self.overlay.cheats;
        c.list
            .iter()
            .filter(|c| c.on)
            .map(|c| c.name.clone())
            .collect()
    }
}
