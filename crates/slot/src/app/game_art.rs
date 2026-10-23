//! Main menu → Network → Game Art: labels for the card's games found online, by `crate::scrape`
//! (unslot's). The page lists every system, then each one, with how many of its games have a
//! label; A finds the missing ones. Label Style draws every label as box art (a disc's art on a
//! PlayStation disc), the cart's own label, or the logo, and a game can keep its own on the
//! in-game This Game page. Automatic finds what is missing whenever the Wi-Fi is up. The
//! ScreenScraper row signs in on the keyboard. Each label goes on its cart as soon as it is
//! found, on every shelf the game is on. The page and its rules are Fragile's (Phase 13), from
//! cloudwithax/unslot@6339c18's Game Art.

use std::collections::HashSet;
use std::path::Path;

use slot_input::{Action, Btn};
use slot_store::{Cart, LabelStyle, Platform, ProfileField, SCRAPED_LABELS};
use slot_ui::{ListPage, ListRow, QuickRow};

use super::overlay::Overlay;
use super::{App, Rack};
use crate::scrape::{Job, ScrapeView, Scraper, SsState};

/// extras.state: the style every game's label follows, and whether missing art is found alone.
const LABEL_STYLE: &str = "label_style";
const AUTOMATIC: &str = "game_art_auto";

#[derive(Default)]
pub struct GameArt {
    scraper: Option<Scraper>,
    /// Whether the scraper reaches the real servers, so Automatic has something to ask.
    live: bool,
    view: ScrapeView,
    /// How many of the scraper's labels are on the shelves already.
    labelled: usize,
    /// Carts given a label since their face was built, for the face builder.
    relabelled: Vec<(usize, usize)>,
    ss: Option<SsState>,
    /// What A last said, until the cursor moves.
    note: Option<String>,
    /// The games Automatic has asked for since boot, so one with no art is asked for once.
    asked: HashSet<(Platform, String)>,
    /// Whether the labels drawn before boot have been checked against their styles.
    checked: bool,
}

/// The page's rows after the systems: Label Style, Automatic, then ScreenScraper.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
enum Row {
    Status,
    All,
    System(Platform),
    Style,
    Automatic,
    ScreenScraper,
}

impl App {
    /// At boot: the scraper's worker, idle until asked.
    pub(super) fn start_game_art(&mut self, root: &Path) {
        let live = crate::scrape::live();
        self.use_scraper(Scraper::for_card(root), live);
    }

    /// The scraper Game Art asks, and whether it reaches the real servers. Tests hand it the
    /// pretend one as if it were live, to see Automatic run.
    pub fn use_scraper(&mut self, scraper: Scraper, live: bool) {
        let ss = self.root.as_deref().map(crate::scrape::ss_state);
        let art = &mut self.overlay.game_art;
        art.scraper = Some(scraper);
        art.live = live;
        art.ss = ss;
    }

    pub fn scrape_view(&self) -> &ScrapeView {
        &self.overlay.game_art.view
    }

    /// The carts whose faces want building again, now they have a label.
    pub fn take_relabelled(&mut self) -> Vec<(usize, usize)> {
        std::mem::take(&mut self.overlay.game_art.relabelled)
    }

    fn label_style(&self) -> LabelStyle {
        self.extras
            .get(LABEL_STYLE)
            .and_then(LabelStyle::parse)
            .unwrap_or_default()
    }

    fn automatic(&self) -> bool {
        self.extras.flag(AUTOMATIC, true)
    }

    /// How a game's label is drawn: its own style, else every game's, as far as its system
    /// can take it.
    fn style_for(&self, cart: &Cart) -> LabelStyle {
        let style = self
            .profiles
            .get(cart.platform, &cart.stem)
            .label
            .unwrap_or_else(|| self.label_style());
        match LabelStyle::of(cart.platform).contains(&style) {
            true => style,
            false => LabelStyle::Art,
        }
    }

    /// Whether a cart's label is one Game Art drew, which a new style may draw again; the
    /// player's own in `Labels/` are never touched.
    fn scraped(&self, cart: &Cart) -> bool {
        let Some(root) = self.root.as_deref() else {
            return false;
        };
        cart.label
            .as_deref()
            .is_some_and(|l| l.starts_with(root.join(SCRAPED_LABELS)))
    }

    /// The systems with a shelf of games, in shelf order.
    fn art_platforms(&self) -> Vec<Platform> {
        self.shelves
            .iter()
            .filter(|(_, shelf)| !shelf.carts.is_empty())
            .filter_map(|(rack, _)| rack.platform())
            .collect()
    }

    /// A system's games, or every system's for None.
    fn art_carts(&self, platform: Option<Platform>) -> Vec<Cart> {
        self.shelves
            .iter()
            .filter(|(rack, _)| match (rack, platform) {
                (Rack::Of(p), Some(want)) => *p == want,
                (Rack::Of(_), None) => true,
                _ => false,
            })
            .flat_map(|(_, shelf)| shelf.carts.iter())
            .cloned()
            .collect()
    }

    fn art_count(&self, platform: Option<Platform>) -> String {
        let carts = self.art_carts(platform);
        let has = carts.iter().filter(|c| c.label.is_some()).count();
        format!("{has} of {}", carts.len())
    }

    /// The Network page's Game Art row: how many games have a label, or that it is busy.
    pub(super) fn game_art_text(&self) -> String {
        match self.overlay.game_art.view.running {
            true => "Finding".into(),
            false => self.art_count(None),
        }
    }

    /// The in-game This Game page's Label row: the game's own style, or Default.
    pub(super) fn this_label_text(&self, row: QuickRow) -> Option<String> {
        if row != QuickRow::ThisLabel {
            return None;
        }
        let cart = self.seated_cart()?;
        let own = self.profiles.get(cart.platform, &cart.stem).label;
        Some(
            own.map_or("Default", |s| s.text(Some(cart.platform)))
                .into(),
        )
    }

    /// Left or Right on This Game → Label: the seated game's own style, its label drawn again.
    pub(super) fn step_own_label(&mut self, right: bool) {
        let Some(cart) = self.seated_cart().cloned() else {
            return;
        };
        let to = self.style_for(&cart).step(Some(cart.platform), right);
        self.set_own(ProfileField::Label(to));
        self.redraw(vec![cart]);
    }

    /// The seated game's label after its own settings were cleared.
    pub(super) fn own_label_cleared(&mut self) {
        if let Some(cart) = self.seated_cart().cloned() {
            self.redraw(vec![cart]);
        }
    }

    /// Draws these games' labels again in their styles: from the art kept on the card when it
    /// was found before, or found now for a game with no label. The player's own are left.
    fn redraw(&mut self, carts: Vec<Cart>) {
        let jobs: Vec<Job> = carts
            .into_iter()
            .filter(|c| c.label.is_none() || self.scraped(c))
            .map(|c| {
                let style = self.style_for(&c);
                (c, style)
            })
            .collect();
        if jobs.is_empty() {
            return;
        }
        if let Some(scraper) = &self.overlay.game_art.scraper {
            scraper.fetch(jobs);
        }
    }

    fn rows_in_order(&self) -> Vec<Row> {
        [Row::Status, Row::All]
            .into_iter()
            .chain(self.art_platforms().into_iter().map(Row::System))
            .chain([Row::Style, Row::Automatic, Row::ScreenScraper])
            .collect()
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

    fn game_art_rows(&self) -> Vec<ListRow> {
        let user = match &self.overlay.game_art.ss {
            Some(SsState::NoDevId(user) | SsState::On(Some(user))) => user.clone(),
            _ => "Sign in".into(),
        };
        self.rows_in_order()
            .into_iter()
            .map(|row| match row {
                Row::Status => ListRow::new(self.art_status()),
                Row::All => ListRow::new("All systems").with_value(self.art_count(None)),
                Row::System(p) => ListRow::new(p.name()).with_value(self.art_count(Some(p))),
                Row::Style => ListRow::new("Label Style").with_value(self.label_style().text(None)),
                Row::Automatic => ListRow::new("Automatic").with_value(match self.automatic() {
                    true => "On",
                    false => "Off",
                }),
                Row::ScreenScraper => ListRow::new("ScreenScraper").with_value(user.clone()),
            })
            .collect()
    }

    pub(super) fn open_game_art(&mut self) {
        let mut page = ListPage::new(self.game_art_rows());
        page.set_cursor(1);
        self.overlay.game_art.note = None;
        self.overlay.open = Some(Overlay::GameArt(page));
        self.overlay.faces.clear();
    }

    /// Each frame: what the scraper has done, each new label on its carts, Automatic, and the
    /// open page.
    pub(super) fn game_art_tick(&mut self) {
        let Some(view) = self.overlay.game_art.scraper.as_ref().map(Scraper::view) else {
            return;
        };
        if view != self.overlay.game_art.view {
            self.overlay.game_art.view = view;
            self.show_scraped();
        }
        self.check_styles();
        self.find_automatically();
        self.refresh_game_art();
    }

    /// Once, after boot: every label Game Art drew, so one drawn in another style than its game
    /// now takes (a default changed, or a setting changed on another device) is drawn again from
    /// the art kept on the card. The worker leaves the rest as they are.
    fn check_styles(&mut self) {
        if std::mem::replace(&mut self.overlay.game_art.checked, true) {
            return;
        }
        let drawn: Vec<Cart> = self
            .art_carts(None)
            .into_iter()
            .filter(|c| self.scraped(c))
            .collect();
        self.redraw(drawn);
    }

    /// With Automatic on and the internet up, the games with no label that have not been asked
    /// for since boot, while the scraper is idle.
    fn find_automatically(&mut self) {
        let art = &self.overlay.game_art;
        if !art.live || art.view.running || !self.automatic() || !crate::wifi::online() {
            return;
        }
        let missing: Vec<Cart> = self
            .art_carts(None)
            .into_iter()
            .filter(|c| c.label.is_none())
            .filter(|c| !art.asked.contains(&(c.platform, c.stem.clone())))
            .collect();
        if missing.is_empty() {
            return;
        }
        let art = &mut self.overlay.game_art;
        art.asked
            .extend(missing.iter().map(|c| (c.platform, c.stem.clone())));
        // Marked running at once, so the next frame does not ask again before the worker
        // has taken the job.
        art.view.running = true;
        self.redraw(missing);
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
                .join(SCRAPED_LABELS)
                .join(platform.dir_name())
                .join(format!("{stem}.png"));
            for (at, (_, shelf)) in self.shelves.iter_mut().enumerate() {
                for i in 0..shelf.carts.len() {
                    let cart = &mut shelf.carts[i];
                    if cart.platform != platform || cart.stem != stem {
                        continue;
                    }
                    // The player's own label stays, though the game was drawn again behind it.
                    if cart
                        .label
                        .as_deref()
                        .is_some_and(|l| !l.starts_with(root.join(SCRAPED_LABELS)))
                    {
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

    /// A on a system finds its games' missing art; A, Left or Right turn Label Style and
    /// Automatic; A on ScreenScraper signs in. Says whether it took the action.
    pub(super) fn game_art_input(&mut self, action: Action) -> bool {
        let Some(Overlay::GameArt(page)) = &self.overlay.open else {
            return false;
        };
        let row = self.rows_in_order().get(page.cursor()).copied();
        let right = !matches!(action, Action::GbaDown(Btn::Left));
        match (action, row) {
            (Action::GbaDown(Btn::Up | Btn::Down), _) => {
                if self.overlay.game_art.note.take().is_some() {
                    self.refresh_game_art();
                }
                return false;
            }
            (Action::GbaDown(Btn::A | Btn::Left | Btn::Right), Some(Row::Style)) => {
                let to = self.label_style().step(None, right);
                self.extras.set(LABEL_STYLE, Some(to.key()));
                self.save_extras();
                let follow: Vec<Cart> = self
                    .art_carts(None)
                    .into_iter()
                    .filter(|c| self.scraped(c))
                    .filter(|c| self.profiles.get(c.platform, &c.stem).label.is_none())
                    .collect();
                self.redraw(follow);
            }
            (Action::GbaDown(Btn::A | Btn::Left | Btn::Right), Some(Row::Automatic)) => {
                let on = !self.automatic();
                self.extras.set_flag(AUTOMATIC, on);
                self.save_extras();
            }
            (Action::GbaDown(Btn::A), Some(Row::ScreenScraper)) => {
                self.start_ss_sign_in();
                return true;
            }
            (Action::GbaDown(Btn::A), Some(Row::All)) => self.find_missing_art(None),
            (Action::GbaDown(Btn::A), Some(Row::System(p))) => self.find_missing_art(Some(p)),
            (Action::GbaDown(Btn::A), _) => {}
            _ => return false,
        }
        self.refresh_game_art();
        true
    }

    fn find_missing_art(&mut self, platform: Option<Platform>) {
        if self.overlay.game_art.view.running {
            return;
        }
        let missing: Vec<Cart> = self
            .art_carts(platform)
            .into_iter()
            .filter(|c| c.label.is_none())
            .collect();
        self.overlay.game_art.note = match () {
            _ if missing.is_empty() => Some("Every game here has a label".into()),
            _ if !crate::wifi::online() => Some("Join a Wi-Fi network first".into()),
            _ => {
                self.redraw(missing);
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
        // Fragile: with ScreenScraper to ask now, its label and disc scans are looked for.
        if matches!(art.ss, Some(SsState::On(_))) {
            art.checked = false;
        }
        art.note = saved
            .err()
            .map(|e| format!("Could not save the login: {e}"));
    }
}
