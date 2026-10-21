//! The Button Guide: every button and what it does, made from the bindings as they are set
//! now, so a remapped shortcut shows its new buttons. The gestures nothing remaps are worded
//! as upstream's own guide words them (site/index.html). Layout ideas from marghost/slot_modded's
//! cheat sheet and roygravy/slot's Buttons pages.

use slot_store::{Binding, Shortcut};
use slot_ui::{ListRow, QuickRow};

use super::App;

const IN_GAME: &str = "In Game";

fn heading(name: &str) -> ListRow {
    ListRow::new(format!("\u{2014} {name} \u{2014}"))
}

fn fixed(what: &str, buttons: &str) -> ListRow {
    ListRow::new(what).with_value(buttons)
}

impl App {
    fn bound_row(&self, row: QuickRow) -> ListRow {
        let text = match self.binding_of(row) {
            Some(b) => Binding::text(Some(b)),
            None => "Not set".into(),
        };
        ListRow::new(row.label()).with_value(text)
    }

    /// Where the guide opens in game: on its In Game part, the buttons that matter there.
    pub(super) fn guide_in_game_at(&self) -> usize {
        let wanted = heading(IN_GAME).label;
        self.guide_rows()
            .iter()
            .position(|r| r.label == wanted)
            .unwrap_or(0)
    }

    pub(super) fn guide_rows(&self) -> Vec<ListRow> {
        let mut rows = vec![
            heading("Shelf"),
            fixed("Browse", "Left / Right"),
            fixed("Next letter", "Up / Down"),
            fixed("Play from last state", "Tap A"),
            fixed("Start fresh", "Hold A"),
            fixed("Pick emulator", "START"),
            fixed("Settings", "MENU"),
        ];
        rows.extend(QuickRow::SHELF_KEYS.map(|r| self.bound_row(r)));
        rows.extend([
            heading(IN_GAME),
            fixed("In-game menu", "MENU"),
            fixed("Eject and save", "Hold MENU"),
            fixed("Save states", "Double tap MENU"),
            fixed("Link screen", "SELECT+MENU"),
            fixed("Stretch / actual (GB)", "L1 / R1"),
        ]);
        rows.extend(QuickRow::SHORTCUTS.map(|r| self.bound_row(r)));
        if let Some(ff) = self.shortcuts.get(Shortcut::FastForward) {
            let held = Binding::text(Some(ff));
            rows.push(fixed("Lock fast forward", &format!("Double tap {held}")));
        }
        rows.push(heading("Game Buttons"));
        rows.extend(
            QuickRow::CONTROLS
                .into_iter()
                .filter(|r| {
                    self.state.turbo || !matches!(r, QuickRow::GameTurboA | QuickRow::GameTurboB)
                })
                .map(|r| self.bound_row(r)),
        );
        rows.extend([
            heading("Everywhere"),
            fixed("Volume", "VOL+ / VOL-"),
            fixed("Mute", "VOL+ and VOL-"),
            fixed("Sleep", "POWER"),
            fixed("Power off", "Hold POWER"),
        ]);
        rows
    }
}
