//! Pages of rows made at run time, drawn over the screen they open from. The first is About's
//! Details; cheats, networks and the button sheet open the same way in later phases.

use slot_gfx::OUT_H;
use slot_input::{Action, Btn};
use slot_power::Charge;
use slot_ui::{Draw, ListPage, ListPageView, ListRow, ListRowFaces, QuickRow, TexId, MOUTH_H};

use super::{App, Phase, HINT_EDGE, LINK_LEGEND_EDGE, LINK_LEGEND_Y};

/// Upstream's sticker credits, kept here too since a device's own label, like the RG35XX Pro's,
/// has no room for them.
const CREDITS: [(&str, &str); 7] = [
    ("slot", "Brandon T. Kowalski"),
    ("Artwork", "Markus"),
    ("Emulation", "mGBA, gpSP"),
    ("System", "BaseOS by pvaibhav"),
    ("Type", "Open Sans, Nerd Fonts"),
    ("Panel Mask", "Gigaherz's LCD3x"),
    ("Fragile Ports", "See CREDITS.md"),
];

pub enum Overlay {
    Details(ListPage),
}

/// Everything the open page needs, in the one field `App` keeps for it.
#[derive(Default)]
pub struct Overlays {
    open: Option<Overlay>,
    /// One per row in view, uploaded by the frontend from the page's own text.
    faces: Vec<ListRowFaces>,
    /// "A Details" on the About sticker, dark and light.
    details_legend: Option<[(TexId, u32); 2]>,
}

impl Overlay {
    fn page(&self) -> &ListPage {
        match self {
            Overlay::Details(page) => page,
        }
    }

    fn page_mut(&mut self) -> &mut ListPage {
        match self {
            Overlay::Details(page) => page,
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

    pub fn set_details_legend(&mut self, dark: (TexId, u32), light: (TexId, u32)) {
        self.overlay.details_legend = Some([dark, light]);
    }

    pub(super) fn open_details(&mut self) {
        let rows = self.details_rows();
        self.overlay.open = Some(Overlay::Details(ListPage::new(rows)));
        self.overlay.faces.clear();
    }

    fn close_overlay(&mut self) {
        self.overlay.open = None;
        self.overlay.faces.clear();
    }

    /// Takes the action when a page is open, and says so. A page left behind by its screen,
    /// eg the power going off under it, is closed and takes nothing.
    pub(super) fn overlay_input(&mut self, action: Action) -> bool {
        if self.overlay.open.is_none() {
            return false;
        }
        if !matches!(self.phase, Phase::About) {
            self.close_overlay();
            return false;
        }
        let Some(page) = self.overlay.open.as_mut().map(Overlay::page_mut) else {
            return false;
        };
        match action {
            Action::GbaDown(Btn::Up) => page.up(),
            Action::GbaDown(Btn::Down) => page.down(),
            Action::GbaDown(Btn::L1) => page.page_up(),
            Action::GbaDown(Btn::R1) => page.page_down(),
            Action::GbaDown(Btn::B) => self.close_overlay(),
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
        rows.extend(CREDITS.map(|(who, what)| ListRow::new(who).with_value(what)));
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
