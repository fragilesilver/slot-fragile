//! Pages of rows made at run time, drawn over the screen they open from: About's Details, and
//! the Button Guide from the Controls page or the in-game Settings, Add to List from the shelf,
//! and Cheats from the in-game menu. Networks open the same way in a later phase.

use slot_gfx::OUT_H;
use slot_input::{Action, Btn};
use slot_power::Charge;
use slot_ui::{Draw, ListPage, ListPageView, ListRow, ListRowFaces, QuickRow, TexId, MOUTH_H};

use super::{App, Phase, HINT_EDGE, LINK_LEGEND_EDGE, LINK_LEGEND_Y};

pub enum Overlay {
    Details(ListPage),
    Guide(ListPage, From),
    /// The lists, and whether the cart under the shelf's cursor is on each.
    Lists(ListPage, slot_store::collections::CartKey),
    /// The playing game's cheats, over the game like the in-game Button Guide.
    Cheats(ListPage),
    /// The Wi-Fi networks in range, from Main menu → Network.
    Networks(ListPage),
    /// The playing game's achievements, over the game.
    Achievements(ListPage),
}

/// Which menu a page was opened from, so it closes when that menu does and B goes back to it.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum From {
    Menu,
    InGame,
}

/// Everything the open page needs, in the one field `App` keeps for it.
#[derive(Default)]
pub struct Overlays {
    pub(super) open: Option<Overlay>,
    /// One per row in view, uploaded by the frontend from the page's own text.
    pub(super) faces: Vec<ListRowFaces>,
    /// The Search shelf, the list shelves and their keyboard.
    pub(super) shelves: super::shelves::Collections,
    /// The playing game's cheats and what the core has of them.
    pub(super) cheats: super::cheats::Cheats,
    /// The playing game's discs, for one on more than one.
    pub(super) discs: super::discs::Discs,
    /// The Wi-Fi radio, the clock's network time and a password being typed.
    pub(super) network: super::network::NetworkState,
    /// "A Details" on the About sticker, dark and light.
    details_legend: Option<[(TexId, u32); 2]>,
    /// The About sticker's own size, when it is not upstream's: the RG35XX Pro's is taller.
    sticker_size: Option<(u32, u32)>,
    /// Menu values made of run-time text, one per row in `QuickRow::ALL`, dim and lit.
    named: Vec<Option<slot_ui::FacePair>>,
    /// Motion Blur for the running core to take up, as colour correction is.
    motion_blur: Option<bool>,
}

impl Overlays {
    pub(super) fn set_motion_blur(&mut self, on: bool) {
        self.motion_blur = Some(on);
    }

    pub(super) fn take_motion_blur(&mut self) -> Option<bool> {
        self.motion_blur.take()
    }
}

impl Overlay {
    fn page(&self) -> &ListPage {
        match self {
            Overlay::Details(page)
            | Overlay::Guide(page, _)
            | Overlay::Lists(page, _)
            | Overlay::Cheats(page)
            | Overlay::Networks(page)
            | Overlay::Achievements(page) => page,
        }
    }

    fn page_mut(&mut self) -> &mut ListPage {
        match self {
            Overlay::Details(page)
            | Overlay::Guide(page, _)
            | Overlay::Lists(page, _)
            | Overlay::Cheats(page)
            | Overlay::Networks(page)
            | Overlay::Achievements(page) => page,
        }
    }
}

impl App {
    pub fn overlay_page(&self) -> Option<&ListPage> {
        self.overlay.open.as_ref().map(Overlay::page)
    }

    pub fn set_overlay_faces(&mut self, faces: Vec<ListRowFaces>) {
        self.overlay.faces = faces;
    }

    pub fn set_named_faces(&mut self, faces: Vec<Option<slot_ui::FacePair>>) {
        self.overlay.named = faces;
    }

    pub(super) fn named_faces(&self) -> &[Option<slot_ui::FacePair>] {
        &self.overlay.named
    }

    pub fn set_sticker_size(&mut self, size: (u32, u32)) {
        self.overlay.sticker_size = Some(size);
    }

    pub(super) fn sticker_size(&self) -> (u32, u32) {
        self.overlay
            .sticker_size
            .unwrap_or((slot_ui::STICKER_W, slot_ui::STICKER_H))
    }

    pub fn set_details_legend(&mut self, dark: (TexId, u32), light: (TexId, u32)) {
        self.overlay.details_legend = Some([dark, light]);
    }

    pub(super) fn open_details(&mut self) {
        let rows = self.details_rows();
        self.overlay.open = Some(Overlay::Details(ListPage::new(rows)));
        self.overlay.faces.clear();
    }

    /// A on a row that opens one of these pages. Says whether it was one.
    pub(super) fn open_overlay_row(&mut self, row: QuickRow) -> bool {
        let from = match row {
            QuickRow::ControlsGuide => From::Menu,
            QuickRow::PlayGuide => From::InGame,
            QuickRow::PlayCheats => {
                self.open_cheats();
                return true;
            }
            QuickRow::WifiNetworks => {
                self.open_networks();
                return true;
            }
            QuickRow::CheevosAccount => {
                self.start_cheevos_sign_in();
                return true;
            }
            QuickRow::PlayAchievements => {
                self.open_achievements();
                return true;
            }
            QuickRow::Network => {
                let row = QuickRow::NETWORK[0];
                self.phase = Phase::QuickMenu { row };
                return true;
            }
            _ => return false,
        };
        let rows = self.guide_rows();
        let mut page = ListPage::new(rows);
        if from == From::InGame {
            page.set_cursor(self.guide_in_game_at());
        }
        self.overlay.open = Some(Overlay::Guide(page, from));
        self.overlay.faces.clear();
        true
    }

    fn guide_from(&self) -> Option<From> {
        match &self.overlay.open {
            Some(Overlay::Guide(_, from)) => Some(*from),
            Some(Overlay::Cheats(_)) => Some(From::InGame),
            Some(Overlay::Networks(_)) => Some(From::Menu),
            Some(Overlay::Achievements(_)) => Some(From::InGame),
            _ => None,
        }
    }

    pub(super) fn guide_open(&self) -> bool {
        self.guide_from().is_some()
    }

    fn cheats_open(&self) -> bool {
        matches!(self.overlay.open, Some(Overlay::Cheats(_)))
    }

    fn close_overlay(&mut self) {
        self.overlay.open = None;
        self.overlay.faces.clear();
    }

    /// Takes the action when a page is open, and says so. A page left behind by its screen,
    /// eg the power going off under it, is closed and takes nothing.
    pub(super) fn overlay_input(&mut self, action: Action) -> bool {
        if self.typing_input(action) {
            return true;
        }
        let alive = match &self.overlay.open {
            None => return false,
            Some(Overlay::Guide(_, From::Menu)) => matches!(self.phase, Phase::QuickMenu { .. }),
            Some(Overlay::Guide(_, From::InGame)) => self.in_game_menu.is_some(),
            Some(Overlay::Lists(..)) => matches!(self.phase, Phase::Shelf),
            Some(Overlay::Cheats(_)) => self.in_game_menu.is_some(),
            Some(Overlay::Networks(_)) => matches!(self.phase, Phase::QuickMenu { .. }),
            Some(Overlay::Achievements(_)) => self.in_game_menu.is_some(),
            Some(Overlay::Details(_)) => matches!(self.phase, Phase::About),
        };
        if !alive {
            self.close_overlay();
            return false;
        }
        if super::shelves::lists_input(self, action)
            || self.cheats_input(action)
            || self.networks_input(action)
        {
            return true;
        }
        let guide = self.guide_open() || matches!(self.overlay.open, Some(Overlay::Lists(..)));
        let Some(page) = self.overlay.open.as_mut().map(Overlay::page_mut) else {
            return false;
        };
        match action {
            Action::GbaDown(Btn::Up) => page.up(),
            Action::GbaDown(Btn::Down) => page.down(),
            Action::GbaDown(Btn::L1) => page.page_up(),
            Action::GbaDown(Btn::R1) => page.page_down(),
            Action::GbaDown(Btn::B) => self.close_overlay(),
            Action::QuickMenu if guide => self.close_overlay(),
            Action::QuickMenu => {
                self.close_overlay();
                self.phase = Phase::QuickMenu {
                    row: QuickRow::About,
                };
            }
            _ => {}
        }
        true
    }

    /// Draws the open page over the menu ground, and says whether there was one.
    pub(super) fn draw_overlay(&self, out: &mut Vec<Draw>) -> bool {
        let (Some(page), Some(faces)) = (self.overlay_page(), &self.quick_menu_faces) else {
            return false;
        };
        if self.overlay.faces.len() != page.shown().len() {
            return true;
        }
        ListPageView {
            page,
            rows: &self.overlay.faces,
            pointer: faces.pointer,
            scroll: faces.scroll,
            empty: None,
            help: None,
            legends: &[],
            ground: None,
            over_game: false,
            footer_top: OUT_H as f32,
            canvas: self.canvas(),
        }
        .draw_rows(out);
        true
    }

    /// The sticker's footer carries three legends, so it draws the plugged slot under them the
    /// way a binding row does, rather than the mouth the middle one would sit on.
    pub(super) fn about_footer_plugged(&self) -> bool {
        self.overlay.open.is_none() && self.overlay.details_legend.is_some()
    }

    /// The menu footer under the Button Guide: Back alone. Says whether it drew.
    pub(super) fn draw_guide_legend(&self, y: f32, out: &mut Vec<Draw>) -> bool {
        let Some(faces) = self.quick_menu_faces.as_ref().filter(|_| self.guide_open()) else {
            return false;
        };
        let legend = [faces.legend, faces.legend_light][self.light() as usize];
        if self.draw_networks_legend(legend[0], y, out) {
            return true;
        }
        self.draw_side_legend(legend[0], false, y, out);
        if self.cheats_open() {
            self.draw_side_legend(legend[5], true, y, out);
        }
        true
    }

    /// The Button Guide over the game, under the same scrim as the in-game menu. Says whether
    /// it drew, so the menu stays when it did not.
    pub(super) fn draw_guide_over_game(&self, footer_top: f32, out: &mut Vec<Draw>) -> bool {
        if self.guide_from() != Some(From::InGame) {
            return false;
        }
        out.push(Draw::Rect {
            x: 0.0,
            y: 0.0,
            w: self.canvas().w,
            h: footer_top.max(0.0),
            colour: [0.0, 0.0, 0.0, 0.65],
        });
        self.draw_overlay(out);
        true
    }

    /// The About footer: Back alone over an open page, or Back, Details and Restart on the
    /// sticker. Says whether it drew, so the sticker's own footer stands in until the faces come.
    pub(super) fn draw_overlay_legend(&self, out: &mut Vec<Draw>) -> bool {
        let (Some(faces), Some(details)) = (&self.quick_menu_faces, self.overlay.details_legend)
        else {
            return false;
        };
        let light = self.light() as usize;
        let legend = [faces.legend, faces.legend_light][light];
        let (back, restart) = (legend[0], legend[4]);
        if self.overlay.open.is_some() {
            self.draw_side_legend(back, false, LINK_LEGEND_Y, out);
            return true;
        }
        let legends = [back, details[light], restart];
        let seen = |w: u32| w.saturating_sub(HINT_EDGE) as f32;
        let room = self.canvas().w - 2.0 * LINK_LEGEND_EDGE;
        let gap = (room - legends.iter().map(|l| seen(l.1)).sum::<f32>()) / 2.0;
        let mut x = LINK_LEGEND_EDGE;
        for (tex, w) in legends {
            out.push(Draw::Tex {
                x: x.round(),
                y: LINK_LEGEND_Y,
                w: w as f32,
                h: MOUTH_H,
                tex,
                alpha: 1.0,
            });
            x += seen(w) + gap;
        }
        true
    }

    fn details_rows(&self) -> Vec<ListRow> {
        let build = crate::build_info::Build::current();
        let mut rows = vec![
            ListRow::new("Version").with_value(format!(
                "Fragile (slot {})",
                crate::build_info::SLOT_RELEASE
            )),
            ListRow::new("Build").with_value(build.serial()),
        ];
        if let Some(v) = crate::baseos::var("BASEOS_VERSION") {
            rows.push(ListRow::new("BaseOS").with_value(v));
        }
        if let Some(model) = crate::baseos::var("BASEOS_MODEL") {
            rows.push(ListRow::new("Device").with_value(model));
        }
        rows.push(ListRow::new("Games").with_value(self.carts().count().to_string()));
        rows.extend(self.card_rows());
        if let Some(b) = self.battery {
            let how = match b.charge {
                Charge::Charging => ", charging",
                Charge::Full => ", full",
                Charge::Discharging | Charge::Unknown => "",
            };
            rows.push(ListRow::new("Battery").with_value(format!("{}%{how}", b.percent)));
        }
        rows.push(ListRow::new("\u{2014} Cores \u{2014}"));
        rows.extend(slot_store::Core::ALL.map(|core| {
            let c = slot_store::core_credits::credit(core);
            ListRow::new(c.name).with_value(c.license)
        }));
        rows
    }

    /// The card the games are on, then the other card when BaseOS has it mounted.
    #[cfg(unix)]
    fn card_rows(&self) -> Vec<ListRow> {
        use crate::card_space::{card_slot, card_space, mount_of};
        let Some(root) = &self.root else {
            return Vec::new();
        };
        let mounts = std::fs::read_to_string("/proc/mounts").unwrap_or_default();
        let games = mount_of(&mounts, root);
        let other = std::path::Path::new("/mnt/system");
        let mut cards = vec![(games.as_ref().map(|(dev, _)| dev.clone()), root.clone())];
        if let Some((dev, at)) = mount_of(&mounts, other) {
            let elsewhere = games.as_ref().is_none_or(|(g, _)| *g != dev);
            if at == "/mnt/system" && elsewhere {
                cards.push((Some(dev), other.to_path_buf()));
            }
        }
        cards
            .into_iter()
            .filter_map(|(dev, path)| {
                let space = card_space(&path)?;
                let label = match dev.as_deref().and_then(card_slot) {
                    Some(slot) => format!("{slot} Card"),
                    None => "Card".into(),
                };
                Some(ListRow::new(label).with_value(space.text()))
            })
            .collect()
    }

    #[cfg(not(unix))]
    fn card_rows(&self) -> Vec<ListRow> {
        Vec::new()
    }
}
