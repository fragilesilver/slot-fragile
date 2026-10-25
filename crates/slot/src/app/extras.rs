//! Fragile's own shelf settings: the shelf slot opens on, whether Favorites shows, and where
//! each shelf was left. They live in `Config/extras.state`, never `slot.state`, so an upstream
//! merge cannot fight over them. Ideas from marghost/slot_modded (startup screen, shelf marks).

use slot_input::Millis;
use slot_store::{read_extras, write_extras, FfMode, Platform, Recent, LID_OFF_MIN};
use slot_ui::{QuickRow, QuickValue};

use super::{App, Rack};

const START_SHELF: &str = "start_shelf";
const SHOW_FAVORITES: &str = "show_favorites";
const CLOCK_12H: &str = "clock_12h";
const SLEEP_OFF: &str = "sleep_off_min";
const FF_PITCH: &str = "ff_pitch";
const SCREENSAVER_TIMEOUT: &str = "screensaver_timeout";
const SCREENSAVER_STYLE: &str = "screensaver_style";

pub const SCREENSAVER_TIMEOUTS: [u32; 5] = [0, 1, 2, 5, 10];

#[derive(Copy, Clone, PartialEq, Eq, Debug, Default)]
pub enum ScreensaverStyle {
    #[default]
    Black,
    Dim,
    Clock,
    CartArt,
    AutoDoze,
}

impl ScreensaverStyle {
    pub const ALL: [ScreensaverStyle; 5] = [
        ScreensaverStyle::Black,
        ScreensaverStyle::Dim,
        ScreensaverStyle::Clock,
        ScreensaverStyle::CartArt,
        ScreensaverStyle::AutoDoze,
    ];

    pub fn text(self) -> &'static str {
        match self {
            Self::Black => "Black",
            Self::Dim => "Dim",
            Self::Clock => "Clock",
            Self::CartArt => "Cart Art",
            Self::AutoDoze => "Auto-Doze",
        }
    }

    pub fn key(self) -> &'static str {
        match self {
            Self::Black => "black",
            Self::Dim => "dim",
            Self::Clock => "clock",
            Self::CartArt => "cart_art",
            Self::AutoDoze => "auto_doze",
        }
    }

    pub fn parse(text: &str) -> Option<ScreensaverStyle> {
        Self::ALL
            .into_iter()
            .find(|s| s.key().eq_ignore_ascii_case(text))
    }

    pub fn step(self, right: bool) -> ScreensaverStyle {
        let n = Self::ALL.len();
        let at = Self::ALL.iter().position(|s| *s == self).unwrap_or(0);
        Self::ALL[if right {
            (at + 1) % n
        } else {
            (at + n - 1) % n
        }]
    }
}

pub(super) fn mark_key(rack: Rack) -> String {
    format!("shelf.{}", rack.key())
}

#[derive(Copy, Clone, PartialEq, Eq, Debug, Default)]
pub enum StartShelf {
    /// Wherever the shelf was when slot last went off: upstream's own behaviour.
    #[default]
    Last,
    Recents,
    Favorites,
    Of(Platform),
}

impl StartShelf {
    pub const ALL: [StartShelf; 10] = [
        StartShelf::Last,
        StartShelf::Recents,
        StartShelf::Favorites,
        StartShelf::Of(Platform::Gba),
        StartShelf::Of(Platform::Gb),
        StartShelf::Of(Platform::Gbc),
        StartShelf::Of(Platform::Nes),
        StartShelf::Of(Platform::Snes),
        StartShelf::Of(Platform::PlayStation),
        StartShelf::Of(Platform::Nds),
    ];

    fn rack(self) -> Option<Rack> {
        match self {
            StartShelf::Last => None,
            StartShelf::Recents => Some(Rack::Recents),
            StartShelf::Favorites => Some(Rack::Favorites),
            StartShelf::Of(p) => Some(Rack::Of(p)),
        }
    }

    fn key(self) -> String {
        self.rack().map_or("last".into(), Rack::key)
    }

    fn parse(text: &str) -> Option<StartShelf> {
        StartShelf::ALL
            .into_iter()
            .find(|s| s.key().eq_ignore_ascii_case(text))
    }

    pub fn value(self) -> QuickValue {
        match self {
            StartShelf::Last => QuickValue::Last,
            StartShelf::Recents => QuickValue::RecentsShelf,
            StartShelf::Favorites => QuickValue::FavoritesShelf,
            StartShelf::Of(Platform::Gba) => QuickValue::GbaShelf,
            StartShelf::Of(Platform::Gb) => QuickValue::GbShelf,
            StartShelf::Of(Platform::Gbc) => QuickValue::GbcShelf,
            StartShelf::Of(Platform::Nes) => QuickValue::NesShelf,
            StartShelf::Of(Platform::Snes) => QuickValue::SnesShelf,
            StartShelf::Of(Platform::PlayStation) => QuickValue::Ps1Shelf,
            StartShelf::Of(Platform::Nds) => QuickValue::NdsShelf,
        }
    }

    fn step(self, right: bool) -> StartShelf {
        let n = StartShelf::ALL.len();
        let at = StartShelf::ALL.iter().position(|s| *s == self).unwrap_or(0);
        StartShelf::ALL[if right {
            (at + 1) % n
        } else {
            (at + n - 1) % n
        }]
    }
}

impl App {
    /// The shadow each NES or SNES cart or disc casts, on every shelf.
    pub fn set_media_shadow(&mut self, m: slot_ui::Media, face: slot_gfx::TexId) {
        for (_, shelf) in &mut self.shelves {
            shelf.set_media_shadow(m, face);
        }
    }

    pub(super) fn load_extras(&mut self, root: &std::path::Path) {
        self.extras = read_extras(root);
    }

    pub(super) fn save_extras(&self) {
        let Some(root) = &self.root else {
            return;
        };
        let written = write_extras(root, &self.extras);
        if let Err(e) = &written {
            eprintln!("slot: extras.state: {e}");
        }
        self.card_writes.wrote("extras.state", &written);
    }

    pub fn start_shelf(&self) -> StartShelf {
        self.extras
            .get(START_SHELF)
            .and_then(StartShelf::parse)
            .unwrap_or_default()
    }

    pub fn show_favorites(&self) -> bool {
        self.extras.flag(SHOW_FAVORITES, true)
    }

    pub fn twelve_hour(&self) -> bool {
        self.extras.flag(CLOCK_12H, false)
    }

    /// Minutes asleep before a device with no lid powers off, or `None` for never. Until it is
    /// set it follows the lid's minutes, so a card moved from a clamshell keeps its timing.
    pub fn sleep_off_min(&self) -> Option<u8> {
        match self.extras.get(SLEEP_OFF) {
            Some("never") => None,
            Some(m) => m
                .parse()
                .ok()
                .filter(|m| LID_OFF_MIN.contains(m))
                .or(Some(self.lid_off_min())),
            None => Some(self.lid_off_min()),
        }
    }

    pub fn ff_pitch(&self) -> bool {
        self.extras.flag(FF_PITCH, false)
    }

    pub fn ff_mode(&self) -> FfMode {
        self.extras.ff_mode()
    }

    pub fn set_ff_mode(&mut self, mode: FfMode) {
        self.extras.set_ff_mode(mode);
        self.save_extras();
    }

    pub fn hdmi_mode(&self) -> crate::hdmi::HdmiMode {
        self.extras
            .get(crate::hdmi::EXTRAS_KEY)
            .and_then(crate::hdmi::HdmiMode::parse)
            .unwrap_or_default()
    }

    pub fn set_hdmi_mode(&mut self, mode: crate::hdmi::HdmiMode) {
        self.extras.set(crate::hdmi::EXTRAS_KEY, Some(mode.key()));
        self.save_extras();
    }

    pub fn step_hdmi_mode(&mut self, right: bool) {
        let current = self.hdmi_mode();
        let next = current.step(right);
        self.set_hdmi_mode(next);
    }

    pub fn screensaver_timeout_min(&self) -> u32 {
        self.extras
            .get(SCREENSAVER_TIMEOUT)
            .and_then(|s| s.parse().ok())
            .unwrap_or(0)
    }

    pub fn screensaver_timeout_text(&self) -> String {
        let min = self.screensaver_timeout_min();
        if min == 0 {
            "Off".to_string()
        } else {
            format!("{min} min")
        }
    }

    pub fn step_screensaver_timeout(&mut self, right: bool) {
        let cur = self.screensaver_timeout_min();
        let at = SCREENSAVER_TIMEOUTS
            .iter()
            .position(|&t| t == cur)
            .unwrap_or(0);
        let n = SCREENSAVER_TIMEOUTS.len();
        let next = if right {
            (at + 1) % n
        } else {
            (at + n - 1) % n
        };
        self.extras.set(
            SCREENSAVER_TIMEOUT,
            Some(&SCREENSAVER_TIMEOUTS[next].to_string()),
        );
        self.save_extras();
    }

    pub fn screensaver_style(&self) -> ScreensaverStyle {
        self.extras
            .get(SCREENSAVER_STYLE)
            .and_then(ScreensaverStyle::parse)
            .unwrap_or_default()
    }

    pub fn step_screensaver_style(&mut self, right: bool) {
        let cur = self.screensaver_style();
        let next = cur.step(right);
        self.extras.set(SCREENSAVER_STYLE, Some(next.key()));
        self.save_extras();
    }

    /// The extras rows the carousel menu hides: a clamshell shows Lid Power Off and anything
    /// else Sleep Power Off. Neither is on an in-game page.
    pub(super) fn extras_hidden(&self) -> Vec<QuickRow> {
        let mut hidden = vec![match self.lid {
            true => QuickRow::SleepOff,
            false => QuickRow::LidOff,
        }];
        hidden.extend(self.network_hidden());
        if !crate::hdmi::present() {
            hidden.push(QuickRow::HdmiOutput);
        }
        if self.in_game_menu.is_none() {
            hidden.push(QuickRow::CardShader);
            hidden.push(QuickRow::ShaderSettings);
        } else if !self.current_shader_has_params() {
            hidden.push(QuickRow::ShaderSettings);
        }
        hidden
    }

    /// How long a doze lasts before the power goes off, or `None` to sleep until woken. With
    /// a lid it is upstream's Lid Power Off; `fallback` is the platform's own timeout.
    pub(super) fn doze_timeout(&self, fallback: Millis) -> Option<Millis> {
        if !self.lid {
            return self.sleep_off_min().map(|m| m as Millis * 60_000);
        }
        Some(match self.state.lid_off_min {
            Some(min) => min as Millis * 60_000,
            None => fallback,
        })
    }

    /// At boot, before the last shelf is put back: every shelf opens on the cart it was left on.
    pub(super) fn restore_shelf_marks(&mut self) {
        for (rack, shelf) in &mut self.shelves {
            let Some(mark) = self.extras.cart(&mark_key(*rack)) else {
                continue;
            };
            if let Some(i) = shelf
                .carts
                .iter()
                .position(|c| c.platform == mark.platform && c.stem == mark.stem)
            {
                shelf.select(i);
            }
        }
    }

    /// At boot, after the last shelf is put back: the chosen start shelf wins, if it has carts.
    pub(super) fn open_start_shelf(&mut self) {
        let Some(rack) = self.start_shelf().rack() else {
            return;
        };
        if let Some(at) = self
            .shelves
            .iter()
            .position(|(r, s)| *r == rack && !s.carts.is_empty())
        {
            self.shelf_at = at;
        }
    }

    /// Writes where each shelf is, only when one has moved since the last write, so browsing
    /// along a shelf writes nothing.
    pub(super) fn mark_shelves(&mut self) {
        let before = self.extras.clone();
        for (rack, shelf) in &self.shelves {
            let cart = shelf.carts.get(shelf.index).map(|c| Recent {
                platform: c.platform,
                stem: c.stem.clone(),
            });
            if cart.is_some() && *rack != Rack::Search {
                self.extras.set_cart(&mark_key(*rack), cart.as_ref());
            }
        }
        if self.extras != before {
            self.save_extras();
        }
    }

    /// The value an extras row shows, or `None` for a row that is not one of them.
    pub(super) fn extras_value(&self, row: QuickRow) -> Option<Option<QuickValue>> {
        match row {
            QuickRow::StartShelf => Some(Some(self.start_shelf().value())),
            QuickRow::ShowFavorites => Some(Some(match self.show_favorites() {
                true => QuickValue::Show,
                false => QuickValue::Hide,
            })),
            QuickRow::SleepOff => Some(match self.sleep_off_min() {
                None => Some(QuickValue::Never),
                Some(m) => QuickValue::LID_OFF
                    .into_iter()
                    .find(|v| v.minutes() == Some(m)),
            }),
            QuickRow::BatteryBadge => Some(Some(QuickValue::flag(self.battery_badge()))),
            QuickRow::FastForwardPitch => Some(Some(QuickValue::flag(self.ff_pitch()))),
            QuickRow::FastForwardMode => Some(Some(match self.ff_mode() {
                FfMode::Hold => QuickValue::Hold,
                FfMode::Toggle => QuickValue::Toggle,
            })),
            QuickRow::ClockFormat => Some(Some(match self.twelve_hour() {
                true => QuickValue::Hour12,
                false => QuickValue::Hour24,
            })),
            QuickRow::HdmiOutput => Some(None),
            QuickRow::Screensaver | QuickRow::ScreensaverStyle => Some(None),
            _ if self.network_value(row).is_some() => self.network_value(row),
            _ => self.screen_value(row),
        }
    }

    /// Changes an extras row and says so, or leaves a row that is not one of them to the caller.
    pub(super) fn change_extra(&mut self, row: QuickRow, right: bool) -> bool {
        match row {
            QuickRow::StartShelf => {
                let to = self.start_shelf().step(right);
                self.extras.set(START_SHELF, Some(&to.key()));
            }
            QuickRow::ShowFavorites => {
                let on = !self.show_favorites();
                self.extras.set_flag(SHOW_FAVORITES, on);
                self.restock_favorites();
            }
            QuickRow::BatteryBadge => self.toggle_battery_badge(),
            QuickRow::HdmiOutput => self.step_hdmi_mode(right),
            QuickRow::FastForwardPitch => {
                let on = !self.ff_pitch();
                self.extras.set_flag(FF_PITCH, on);
            }
            QuickRow::FastForwardMode => {
                let to = match self.ff_mode() {
                    FfMode::Hold => FfMode::Toggle,
                    FfMode::Toggle => FfMode::Hold,
                };
                self.set_ff_mode(to);
            }
            QuickRow::ClockFormat => {
                let on = !self.twelve_hour();
                self.extras.set_flag(CLOCK_12H, on);
            }
            QuickRow::Screensaver => {
                self.step_screensaver_timeout(right);
            }
            QuickRow::ScreensaverStyle => {
                self.step_screensaver_style(right);
            }
            QuickRow::SleepOff => {
                // Never, then the lid's own steps.
                let steps: Vec<Option<u8>> = std::iter::once(None)
                    .chain(LID_OFF_MIN.iter().map(|m| Some(*m)))
                    .collect();
                let n = steps.len();
                let at = steps
                    .iter()
                    .position(|s| *s == self.sleep_off_min())
                    .unwrap_or(0);
                let to = steps[if right {
                    (at + 1) % n
                } else {
                    (at + n - 1) % n
                }];
                let text = to.map_or("never".to_string(), |m| m.to_string());
                self.extras.set(SLEEP_OFF, Some(&text));
            }
            QuickRow::PlayDisc => {
                self.step_disc(right);
                return true;
            }
            _ if self.change_screen(row, right) => {}
            _ if self.change_network(row) => return true,
            _ => return false,
        }
        self.save_extras();
        true
    }

    pub fn ota_target(&self) -> String {
        self.extras
            .get("ota_repo")
            .or_else(|| self.extras.get("ota_url"))
            .unwrap_or(crate::ota::DEFAULT_REPO)
            .to_string()
    }

    pub fn set_ota_target(&mut self, target: &str) {
        self.extras.set("ota_repo", Some(target));
        self.save_extras();
        if let Some(ota) = &mut self.ota {
            ota.set_target(target.to_string());
        }
    }
}
