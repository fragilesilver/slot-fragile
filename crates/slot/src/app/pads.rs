//! Fragile: what each system's pad changes in slot's menus and shortcuts (Phase 12). Upstream
//! knew one pad, the GBA's; a SNES pad has X and Y of its own, a PlayStation one L2 and R2 too.

use slot_input::{Action, Chord, Entry, Mod};
use slot_store::{Binding, Device, GameButton, Modifier};
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
    /// the button to the game: a PlayStation game's L2 and R2 are its own. Fast forward and
    /// rewind move to SELECT with the same button, in place of what was there (the Game Boy
    /// palettes, by default). The Game Boy family's pads have none, so theirs are upstream's.
    pub fn yield_to_pad(&self, entries: &mut Vec<Entry>) {
        let taken: Vec<_> = self
            .taken_by_pad()
            .into_iter()
            .map(crate::input::device_btn)
            .collect();
        let mut moved = Vec::new();
        entries.retain(|e| {
            let yields = e.chord.modifier.is_none() && taken.contains(&e.chord.button);
            if yields && matches!(e.action, Action::FfStart | Action::RewindStart) {
                moved.push(Entry {
                    chord: Chord {
                        modifier: Some(Mod::Select),
                        button: e.chord.button,
                    },
                    ..*e
                });
            }
            !yields
        });
        for entry in moved {
            entries.retain(|e| e.chord != entry.chord);
            entries.insert(0, entry);
        }
    }

    fn playing(&self) -> bool {
        matches!(self.phase, Phase::Playing { .. } | Phase::Inserting { .. })
    }

    /// The buttons the playing pad's own X, Y, L2 and R2 are on alone, which shortcuts give up.
    fn taken_by_pad(&self) -> Vec<Device> {
        if !self.playing() {
            return Vec::new();
        }
        let pad = self.game_pad();
        GameButton::EXTRA
            .into_iter()
            .filter(|g| pad.contains(g))
            .filter_map(|g| self.controls.game(g))
            .filter(|b| b.modifier.is_none())
            .map(|b| b.button)
            .collect()
    }

    /// The Button Guide's word for a shortcut that moved to SELECT for the playing pad.
    pub(super) fn guide_moved(&self, row: QuickRow) -> Option<String> {
        if !matches!(row, QuickRow::ScFastForward | QuickRow::ScRewind) {
            return None;
        }
        let bound = self.binding_of(row)?;
        if bound.modifier.is_some() || !self.taken_by_pad().contains(&bound.button) {
            return None;
        }
        Some(Binding::text(Binding::with(Modifier::Select, bound.button)))
    }

    /// In game the guide lists only the buttons the playing pad has.
    pub(super) fn guide_has_button(&self, row: QuickRow) -> bool {
        !self.playing()
            || row
                .game_button()
                .is_none_or(|g| self.game_pad().contains(&g))
    }
}
