//! Fragile's own menu pages, kept out of quick_menu.rs so an upstream merge there stays small.

use crate::quick_menu::QuickRow;

impl QuickRow {
    /// In game, Settings → This Game: the settings this one game keeps for itself.
    pub const THIS_GAME: [QuickRow; 5] = [
        QuickRow::ThisShader,
        QuickRow::ThisColour,
        QuickRow::ThisFfSpeed,
        QuickRow::ThisFfSound,
        QuickRow::ThisDefaults,
    ];
}
