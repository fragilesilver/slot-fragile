//! Fragile's screen extras. Integer Scale draws the picture at a whole multiple of its size on
//! a 4:3 panel, centred, so every game pixel is the same size: GB and GBC at 3x (480x432),
//! GBA at 2x (480x320) on a 640x480 screen. Ideas from marghost/slot_modded. The rows sit on
//! each console's Screen page, shown on every panel so upstream's page layouts hold; on a 3:2
//! panel the setting changes nothing.

use slot_store::Platform;
use slot_ui::{QuickRow, QuickValue, Toast};

use super::{App, VideoMode};

fn card_shader_key(platform: Platform) -> &'static str {
    match platform {
        Platform::Gba => "card_shader_gba",
        Platform::Gb => "card_shader_gb",
        Platform::Gbc => "card_shader_gbc",
    }
}

fn card_shader_row_platform(row: QuickRow) -> Option<Platform> {
    match row {
        QuickRow::CardShaderGba => Some(Platform::Gba),
        QuickRow::CardShaderGb => Some(Platform::Gb),
        QuickRow::CardShaderGbc => Some(Platform::Gbc),
        _ => None,
    }
}

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
        if card_shader_row_platform(row).is_some() {
            // Drawn from `named_value`, since a file name has no ready-made face.
            return Some(None);
        }
        let platform = Self::integer_row_platform(row)?;
        Some(Some(QuickValue::flag(self.integer_scale(platform))))
    }

    pub(super) fn change_screen(&mut self, row: QuickRow, right: bool) -> bool {
        if let Some(platform) = card_shader_row_platform(row) {
            self.step_card_shader(platform, right);
            return true;
        }
        let Some(platform) = Self::integer_row_platform(row) else {
            return false;
        };
        let on = !self.integer_scale(platform);
        self.extras.set_flag(integer_key(platform), on);
        true
    }

    /// The shader from `Shaders/` a console draws with, by name; `None` for the built-in.
    pub fn card_shader_for(&self, platform: Platform) -> Option<String> {
        self.extras
            .get(card_shader_key(platform))
            .map(str::to_string)
    }

    /// The card shader for the game being played.
    pub fn card_shader(&self) -> Option<String> {
        self.card_shader_for(self.playing_platform())
    }

    /// The frontend could not read or compile it, and the built-in shader is on instead.
    pub fn card_shader_failed(&mut self) {
        self.hud.toast(Toast::ShaderFailed, self.now());
    }

    fn step_card_shader(&mut self, platform: Platform, right: bool) {
        let mut choices = vec![None];
        if let Some(root) = &self.root {
            choices.extend(slot_store::list_shaders(root).into_iter().map(Some));
        }
        let now = self.card_shader_for(platform);
        let at = choices.iter().position(|c| *c == now).unwrap_or(0);
        let n = choices.len();
        let to = &choices[if right {
            (at + 1) % n
        } else {
            (at + n - 1) % n
        }];
        self.extras.set(card_shader_key(platform), to.as_deref());
    }

    /// A menu value that is text slot only knows at run time, for the frontend to draw.
    pub fn named_value(&self, row: QuickRow) -> Option<String> {
        let platform = card_shader_row_platform(row)?;
        Some(
            self.card_shader_for(platform)
                .unwrap_or_else(|| "Off".into()),
        )
    }
}
