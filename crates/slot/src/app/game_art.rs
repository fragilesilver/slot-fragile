//! Main menu → Network → Game Art: labels for the card's games found online, by `crate::scrape`
//! (unslot's). The page lists every system, then each one, with how many of its games have a
//! label; A finds the missing ones, and the ScreenScraper row signs in on the keyboard. Each
//! label goes on its cart as soon as it is found, on every shelf the game is on. The page and
//! its rules are Fragile's (Phase 13), from cloudwithax/unslot@6339c18's Game Art.

use std::path::Path;

use slot_input::{Action, Btn};
use slot_store::{Cart, Platform};
use slot_ui::{ListPage, ListRow};

use super::overlay::Overlay;
use super::{App, Rack};
use crate::scrape::{ScrapeView, Scraper, SsState};

#[derive(Default)]
pub struct GameArt {
    scraper: Option<Scraper>,
    view: ScrapeView,
    /// How many of the scraper's labels are on the shelves already.
    labelled: usize,
    /// Carts given a label since their face was built, for the face builder.
    relabelled: Vec<(usize, usize)>,
    ss: Option<SsState>,
    /// What A last said, until the cursor moves.
    note: Option<String>,
}

impl App {
    /// At boot: the scraper's worker, idle until asked.
    pub(super) fn start_game_art(&mut self, root: &Path) {
        let art = &mut self.overlay.game_art;
        art.scraper = Some(Scraper::for_card(root));
        art.ss = Some(crate::scrape::ss_state(root));
    }

    pub fn scrape_view(&self) -> &ScrapeView {
        &self.overlay.game_art.view
    }

    /// The carts whose faces want building again, now they have a label.
    pub fn take_relabelled(&mut self) -> Vec<(usize, usize)> {
        std::mem::take(&mut self.overlay.game_art.relabelled)
    }

    /// The systems with a shelf of games, in shelf order.
    fn art_platforms(&self) -> Vec<Platform> {
        self.shelves
            .iter()
            .filter(|(_, shelf)| !shelf.carts.is_empty())
            .filter_map(|(rack, _)| rack.platform())
            .collect()
    }

    /// A system's games, or every system's for None, and whether each has a label.
    fn art_carts(&self, platform: Option<Platform>) -> Vec<(Cart, bool)> {
        self.shelves
            .iter()
            .filter(|(rack, _)| match (rack, platform) {
                (Rack::Of(p), Some(want)) => *p == want,
                (Rack::Of(_), None) => true,
                _ => false,
            })
            .flat_map(|(_, shelf)| shelf.carts.iter())
            .map(|c| (c.clone(), c.label.is_some()))
            .collect()
    }

    fn art_count(&self, platform: Option<Platform>) -> String {
        let carts = self.art_carts(platform);
        let has = carts.iter().filter(|(_, has)| *has).count();
        format!("{has} of {}", carts.len())
    }

    /// The Network page's Game Art row: how many games have a label, or that it is busy.
    pub(super) fn game_art_text(&self) -> String {
        match self.overlay.game_art.view.running {
            true => "Finding".into(),
            false => self.art_count(None),
        }
    }

    fn art_status(&self) -> String {
        let art = &self.overlay.game_art;
        let v = &art.view;
        let made = v.made.len();
        let s = |n: usize| if n == 1 { "" } else { "s" };
        match () {
            _ if v.running => format!(
                "Finding art for {} ({} of {})",
                v.current.as_deref().unwrap_or("..."),
                (v.done + 1).min(v.total),
                v.total
            ),
            _ if art.note.is_some() => art.note.clone().unwrap_or_default(),
            _ if v.error.is_some() => v.error.clone().unwrap_or_default(),
            _ if made > 0 => format!("Added {made} label{} to the shelves", s(made)),
            _ if v.missed > 0 => format!("No art found for {} game{}", v.missed, s(v.missed)),
            _ if !crate::wifi::online() => "Join a Wi-Fi network first".into(),
            _ => match art.ss {
                Some(SsState::On(_)) => "Art from ScreenScraper and libretro".into(),
                Some(SsState::NoDevId(_)) => "ScreenScraper needs a developer ID".into(),
                _ => "Art from libretro-thumbnails".into(),
            },
        }
    }

    /// The status, All systems, each system, then ScreenScraper.
    fn game_art_rows(&self) -> Vec<ListRow> {
        let user = match &self.overlay.game_art.ss {
            Some(SsState::NoDevId(user) | SsState::On(Some(user))) => user.clone(),
            _ => "Sign in".into(),
        };
        std::iter::once(ListRow::new(self.art_status()))
            .chain(std::iter::once(
                ListRow::new("All systems").with_value(self.art_count(None)),
            ))
            .chain(
                self.art_platforms()
                    .into_iter()
                    .map(|p| ListRow::new(p.name()).with_value(self.art_count(Some(p)))),
            )
            .chain(std::iter::once(
                ListRow::new("ScreenScraper").with_value(user),
            ))
            .collect()
    }

    pub(super) fn open_game_art(&mut self) {
        let mut page = ListPage::new(self.game_art_rows());
        page.set_cursor(1);
        self.overlay.game_art.note = None;
        self.overlay.open = Some(Overlay::GameArt(page));
        self.overlay.faces.clear();
    }

    /// Each frame: what the scraper has done, each new label on its carts, and the open page.
    pub(super) fn game_art_tick(&mut self) {
        let Some(view) = self.overlay.game_art.scraper.as_ref().map(Scraper::view) else {
            return;
        };
        if view != self.overlay.game_art.view {
            self.overlay.game_art.view = view;
            self.show_scraped();
        }
        self.refresh_game_art();
    }

    /// The open page's rows, as they are now.
    fn refresh_game_art(&mut self) {
        if !matches!(self.overlay.open, Some(Overlay::GameArt(_))) {
            return;
        }
        let rows = self.game_art_rows();
        if let Some(Overlay::GameArt(page)) = &mut self.overlay.open {
            if page.rows() != rows {
                page.set_rows(rows);
            }
        }
    }

    /// Puts each label the scraper has made since last time on its cart, on every shelf the
    /// game is on, so it shows without slot starting again.
    fn show_scraped(&mut self) {
        let Some(root) = self.root.clone() else {
            return;
        };
        let art = &mut self.overlay.game_art;
        let made: Vec<_> = art
            .view
            .made
            .get(art.labelled..)
            .unwrap_or_default()
            .to_vec();
        art.labelled = art.view.made.len();
        for (platform, stem) in made {
            let label = root
                .join("Labels")
                .join(platform.dir_name())
                .join(format!("{stem}.png"));
            for (at, (_, shelf)) in self.shelves.iter_mut().enumerate() {
                for i in 0..shelf.carts.len() {
                    let cart = &mut shelf.carts[i];
                    if cart.platform != platform || cart.stem != stem {
                        continue;
                    }
                    cart.label = Some(label.clone());
                    // A cart with no face yet gets one with the label when it comes into view.
                    if shelf.face(i).is_some() {
                        self.overlay.game_art.relabelled.push((at, i));
                    }
                }
            }
        }
    }

    /// A on a system finds its games' missing art, on ScreenScraper signs in. Says whether it
    /// took the action.
    pub(super) fn game_art_input(&mut self, action: Action) -> bool {
        let Some(Overlay::GameArt(page)) = &self.overlay.open else {
            return false;
        };
        let (cursor, count) = (page.cursor(), page.rows().len());
        match action {
            Action::GbaDown(Btn::Up | Btn::Down) => {
                if self.overlay.game_art.note.take().is_some() {
                    self.refresh_game_art();
                }
                false
            }
            Action::GbaDown(Btn::A) if cursor + 1 == count => {
                self.start_ss_sign_in();
                true
            }
            Action::GbaDown(Btn::A) if cursor >= 1 => {
                self.find_missing_art(cursor);
                self.refresh_game_art();
                true
            }
            Action::GbaDown(Btn::A) => true,
            _ => false,
        }
    }

    fn find_missing_art(&mut self, cursor: usize) {
        if self.overlay.game_art.view.running {
            return;
        }
        let which = cursor
            .checked_sub(2)
            .and_then(|i| self.art_platforms().get(i).copied());
        let missing: Vec<Cart> = self
            .art_carts(which)
            .into_iter()
            .filter(|(_, has)| !has)
            .map(|(c, _)| c)
            .collect();
        let art = &mut self.overlay.game_art;
        art.note = match () {
            _ if missing.is_empty() => Some("Every game here has a label".into()),
            _ if !crate::wifi::online() => Some("Join a Wi-Fi network first".into()),
            _ => {
                if let Some(scraper) = &art.scraper {
                    scraper.fetch(missing);
                }
                None
            }
        };
    }

    fn start_ss_sign_in(&mut self) {
        let user = match &self.overlay.game_art.ss {
            Some(SsState::NoDevId(user) | SsState::On(Some(user))) => user.clone(),
            _ => String::new(),
        };
        self.type_ss_user(&user);
    }

    /// Saves a ScreenScraper login typed in on the device, or signs out for an empty user.
    pub(super) fn save_ss_login(&mut self, user: &str, password: &str) {
        let Some(root) = self.root.clone() else {
            return;
        };
        let saved = crate::scrape::save_ss_login(&root, user, password);
        let art = &mut self.overlay.game_art;
        art.ss = Some(crate::scrape::ss_state(&root));
        art.note = saved
            .err()
            .map(|e| format!("Could not save the login: {e}"));
    }
}
