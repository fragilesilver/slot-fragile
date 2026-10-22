//! Fragile: what each system's pad changes in slot's menus and shortcuts (Phase 12). Upstream
//! knew one pad, the GBA's; a SNES pad has X and Y of its own, a PlayStation one L2 and R2 too.

use slot_input::Entry;
use slot_store::GameButton;
use slot_ui::QuickRow;

use super::{App, Phase};

impl App {
    /// The rows the playing system has no use for, after upstream's own: Color Correction on a
    /// system with none, Bezel on one drawn at its own size with none, and each button its pad
    /// does not have.
    pub(super) fn system_hidden(&self, hidden: &mut Vec<QuickRow>) {
        let spec = self.playing_platform().spec();
        let mut hide = |row: QuickRow| {
            if !hidden.contains(&row) {
                hidden.push(row);
            }
        };
        if !spec.colour_correction {
            hide(QuickRow::ColourCorrection);
        }
        if spec.native && !spec.bezels {
            hide(QuickRow::Bezel);
        }
        let pad = self.game_pad();
        for row in QuickRow::CONTROLS {
            if row.game_button().is_some_and(|g| !pad.contains(&g)) {
                hide(row);
            }
        }
    }

    /// While a game plays, a single-button shortcut on one of its pad's own extra buttons gives
    /// the button to the game: a PlayStation game's L2 and R2 are its own, not rewind and fast
    /// forward. The Game Boy family's pads have none, so theirs are upstream's.
    pub fn yield_to_pad(&self, entries: &mut Vec<Entry>) {
        if !matches!(self.phase, Phase::Playing { .. } | Phase::Inserting { .. }) {
            return;
        }
        let pad = self.game_pad();
        let taken: Vec<_> = GameButton::EXTRA
            .into_iter()
            .filter(|g| pad.contains(g))
            .filter_map(|g| self.controls.game(g))
            .filter(|b| b.modifier.is_none())
            .map(|b| crate::input::device_btn(b.button))
            .collect();
        entries.retain(|e| e.chord.modifier.is_some() || !taken.contains(&e.chord.button));
    }
}
