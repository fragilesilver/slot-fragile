//! Fragile's own menu pages, kept out of quick_menu.rs so an upstream merge there stays small.

use crate::bitmaps::ButtonLegend;
use crate::plate::UndoFace;
use crate::quick_menu::{button_face, with_hint, QuickRow};

/// The Networks page's own legends in the menu footer: A Join, X Forget.
pub fn network_legend_faces(ink: [u8; 3]) -> [UndoFace; 2] {
    [
        with_hint(button_face(ButtonLegend::A), "Join", Some(ink), false),
        with_hint(button_face(ButtonLegend::X), "Forget", Some(ink), false),
    ]
}

impl QuickRow {
    /// In game, Settings → This Game: the settings this one game keeps for itself.
    pub const THIS_GAME: [QuickRow; 5] = [
        QuickRow::ThisShader,
        QuickRow::ThisColour,
        QuickRow::ThisFfSpeed,
        QuickRow::ThisFfSound,
        QuickRow::ThisDefaults,
    ];

    /// Main menu → Network: the Wi-Fi and network time.
    pub const NETWORK: [QuickRow; 7] = [
        QuickRow::WifiPower,
        QuickRow::WifiNetworks,
        QuickRow::WifiStatus,
        QuickRow::WifiIp,
        QuickRow::TimeSync,
        QuickRow::Cheevos,
        QuickRow::CheevosAccount,
    ];
}
