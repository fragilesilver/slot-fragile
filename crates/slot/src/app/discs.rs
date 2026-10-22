//! Fragile: the in-game Disc row, for a game on more than one disc (Phase 12). The session tells
//! the app which disc is in each frame and hands the core the one asked for.

use slot_store::Platform;
use slot_ui::{QuickRow, Toast};

use super::App;

/// Set in extras.state once the built-in BIOS note has been shown for a system, by its folder.
const BIOS_NOTED: &str = "bios_noted";

#[derive(Default)]
pub struct Discs {
    /// The disc in the drive (from 0) and how many there are, as the core last said.
    now: Option<(u8, u8)>,
    /// A disc asked for that the core has not been handed yet.
    want: Option<u8>,
}

impl App {
    /// A game has started on its core. With no BIOS on the card for a system that boots from
    /// one, the core runs on its own, which most games take but some do not: say so, once.
    pub fn core_opened(&mut self, platform: Platform, named: bool) {
        let Some(root) = self.root() else {
            return;
        };
        if !named || crate::discs::has_bios(root, platform) != Some(false) {
            return;
        }
        let key = format!("{BIOS_NOTED}.{}", platform.key());
        if self.extras.flag(&key, false) {
            return;
        }
        self.extras.set_flag(&key, true);
        self.save_extras();
        self.hud.toast(Toast::BuiltInBios, self.now());
    }

    /// What the running core says of its discs, None between games.
    pub fn set_discs(&mut self, discs: Option<(u8, u8)>) {
        self.overlay.discs.now = discs;
    }

    /// The disc for the core to put in, once.
    pub fn take_disc(&mut self) -> Option<u8> {
        self.overlay.discs.want.take()
    }

    /// The Disc row shows for a game on more than one disc.
    pub(super) fn disc_hidden(&self) -> bool {
        !matches!(self.overlay.discs.now, Some((_, count)) if count > 1)
    }

    /// The disc showing on the row: the one asked for, before the core has it.
    fn disc_shown(&self) -> Option<(u8, u8)> {
        let (at, count) = self.overlay.discs.now?;
        Some((self.overlay.discs.want.unwrap_or(at), count))
    }

    pub(super) fn disc_text(&self, row: QuickRow) -> Option<String> {
        if row != QuickRow::PlayDisc {
            return None;
        }
        let (at, count) = self.disc_shown()?;
        Some(format!("{} of {count}", at + 1))
    }

    /// Left and right step through the discs, round from the last to the first.
    pub(super) fn step_disc(&mut self, right: bool) {
        let Some((at, count)) = self.disc_shown() else {
            return;
        };
        if count < 2 {
            return;
        }
        let to = match right {
            true => (at + 1) % count,
            false => (at + count - 1) % count,
        };
        self.overlay.discs.want = Some(to);
    }
}
