//! Fragile's own menu pages, kept out of quick_menu.rs so an upstream merge there stays small.

use crate::bitmaps::ButtonLegend;
use crate::plate::UndoFace;
use crate::quick_menu::{button_face, with_hint, QuickRow};

/// The Add to List page's legends for a list row: X Rename, Y Delete.
pub fn list_legend_faces(ink: [u8; 3]) -> [UndoFace; 2] {
    [
        with_hint(button_face(ButtonLegend::X), "Rename", Some(ink), false),
        with_hint(button_face(ButtonLegend::Y), "Delete", Some(ink), false),
    ]
}

/// The Networks page's own legends in the menu footer: A Join, X Forget.
pub fn network_legend_faces(ink: [u8; 3]) -> [UndoFace; 2] {
    [
        with_hint(button_face(ButtonLegend::A), "Join", Some(ink), false),
        with_hint(button_face(ButtonLegend::X), "Forget", Some(ink), false),
    ]
}

/// The Bluetooth page's own legends in the menu footer: Y Search, X Forget, A Connect, A Disconnect, A Pair.
pub fn bluetooth_legend_faces(ink: [u8; 3]) -> [UndoFace; 5] {
    let hint = Some(ink);
    [
        with_hint(button_face(ButtonLegend::Y), "Search", hint, false),
        with_hint(button_face(ButtonLegend::X), "Forget", hint, false),
        with_hint(button_face(ButtonLegend::A), "Connect", hint, false),
        with_hint(button_face(ButtonLegend::A), "Disconnect", hint, false),
        with_hint(button_face(ButtonLegend::A), "Pair", hint, false),
    ]
}

impl QuickRow {
    /// In game, Settings → This Game: the settings this one game keeps for itself.
    pub const THIS_GAME: [QuickRow; 9] = [
        QuickRow::ThisScaling,
        QuickRow::ThisNdsLayout,
        QuickRow::ThisShader,
        QuickRow::ThisCardShader,
        QuickRow::ThisColour,
        QuickRow::ThisFfSpeed,
        QuickRow::ThisFfSound,
        QuickRow::ThisLabel,
        QuickRow::ThisDefaults,
    ];

    /// Main menu → Network: the Wi-Fi, Bluetooth and network time.
    pub const NETWORK: [QuickRow; 14] = [
        QuickRow::WifiPower,
        QuickRow::WifiNetworks,
        QuickRow::WifiStatus,
        QuickRow::WifiIp,
        QuickRow::Bluetooth,
        QuickRow::BtDevices,
        QuickRow::SshPower,
        QuickRow::SshPassword,
        QuickRow::WebDashboard,
        QuickRow::TimeSync,
        QuickRow::Cheevos,
        QuickRow::CheevosAccount,
        QuickRow::GameArt,
        QuickRow::Update,
    ];
}
