//! Fragile's screen extras. Integer Scale draws the picture at a whole multiple of its size on
//! a 4:3 panel, centred, so every game pixel is the same size: GB and GBC at 3x (480x432),
//! GBA at 2x (480x320) on a 640x480 screen. Ideas from marghost/slot_modded. The rows sit on
//! each console's Screen page, shown on every panel so upstream's page layouts hold; on a 3:2
//! panel the setting changes nothing.

use slot_store::Platform;
use slot_ui::{QuickRow, QuickValue};

use super::{App, VideoMode};

fn integer_key(platform: Platform) -> &'static str {
    match platform {
        Platform::Gba => "integer_gba",
        Platform::Gb => "integer_gb",
        Platform::Gbc => "integer_gbc",
    }
}

impl App {
    pub fn integer_scale(&self, platform: Platform) -> bool {
        self.extras.flag(integer_key(platform), false)
    }

    /// The picture size the compositor draws at a whole multiple, while Integer Scale is on for
    /// the playing console on a 4:3 panel. Stretch wins over it.
    pub fn integer_picture(&self) -> Option<(u32, u32)> {
        let platform = self.playing_platform();
        (self.framed_panel()
            && self.video_mode == VideoMode::Actual
            && self.integer_scale(platform))
        .then(|| platform.picture())
    }

    fn integer_row_platform(row: QuickRow) -> Option<Platform> {
        match row {
            QuickRow::IntegerGba => Some(Platform::Gba),
            QuickRow::IntegerGb => Some(Platform::Gb),
            QuickRow::IntegerGbc => Some(Platform::Gbc),
            _ => None,
        }
    }

    pub(super) fn screen_value(&self, row: QuickRow) -> Option<Option<QuickValue>> {
        let platform = Self::integer_row_platform(row)?;
        Some(Some(QuickValue::flag(self.integer_scale(platform))))
    }

    pub(super) fn change_screen(&mut self, row: QuickRow) -> bool {
        let Some(platform) = Self::integer_row_platform(row) else {
            return false;
        };
        let on = !self.integer_scale(platform);
        self.extras.set_flag(integer_key(platform), on);
        true
    }
}
