//! Fragile's screen extras. Integer Scale draws the picture at a whole multiple of its size on
//! a 4:3 panel, centred, so every game pixel is the same size: GB and GBC at 3x (480x432),
//! GBA at 2x (480x320) on a 640x480 screen. Ideas from marghost/slot_modded. The rows sit on
//! each console's Screen page, shown on every panel so upstream's page layouts hold; on a 3:2
//! panel the setting changes nothing.

use slot_store::{platform_settings, BezelPick, Platform, Scaling, Shader};
use slot_ui::{QuickRow, QuickValue, Toast};

use super::{App, VideoMode};

fn card_shader_key(platform: Platform) -> String {
    format!("card_shader_{}", platform.key())
}

fn card_shader_row_platform(row: QuickRow) -> Option<Platform> {
    match row {
        QuickRow::CardShaderGba => Some(Platform::Gba),
        QuickRow::CardShaderGb => Some(Platform::Gb),
        QuickRow::CardShaderGbc => Some(Platform::Gbc),
        QuickRow::CardShaderNes => Some(Platform::Nes),
        QuickRow::CardShaderSnes => Some(Platform::Snes),
        QuickRow::CardShaderPs => Some(Platform::PlayStation),
        QuickRow::CardShaderNds => Some(Platform::Nds),
        _ => None,
    }
}

fn motion_blur_key(platform: Platform) -> String {
    format!("motion_blur_{}", platform.key())
}

fn motion_blur_row_platform(row: QuickRow) -> Option<Platform> {
    match row {
        QuickRow::MotionBlurGba => Some(Platform::Gba),
        QuickRow::MotionBlurGb => Some(Platform::Gb),
        QuickRow::MotionBlurGbc => Some(Platform::Gbc),
        _ => None,
    }
}

/// The Shader rows of the systems Fragile adds, whose shader lives in extras.state.
fn shader_row_platform(row: QuickRow) -> Option<Platform> {
    match row {
        QuickRow::NesShader => Some(Platform::Nes),
        QuickRow::SnesShader => Some(Platform::Snes),
        QuickRow::PsShader => Some(Platform::PlayStation),
        QuickRow::NdsShader => Some(Platform::Nds),
        _ => None,
    }
}

impl App {
    /// The buttons the playing system's pad has.
    pub fn game_pad(&self) -> &'static [slot_store::GameButton] {
        slot_store::GameButton::of(self.playing_platform())
    }

    pub(super) fn platform_shader(&self, platform: Platform) -> Shader {
        platform_settings::shader(&self.state, &self.extras, platform)
    }

    pub(super) fn platform_colour(&self, platform: Platform) -> bool {
        platform_settings::colour(&self.state, &self.extras, platform)
    }

    pub(super) fn platform_bezel(&self, platform: Platform) -> BezelPick {
        platform_settings::bezel(&self.state, &self.extras, platform)
    }

    /// The caller saves slot.state; a system Fragile adds keeps its setting in extras.state,
    /// saved here.
    pub(super) fn set_platform_shader(&mut self, platform: Platform, to: Shader) {
        platform_settings::set_shader(&mut self.state, &mut self.extras, platform, to);
        if platform_settings::in_extras(platform) {
            self.save_extras();
        }
    }

    pub(super) fn set_platform_bezel(&mut self, platform: Platform, to: BezelPick) {
        platform_settings::set_bezel(&mut self.state, &mut self.extras, platform, to);
        if platform_settings::in_extras(platform) {
            self.save_extras();
        }
    }

    pub fn scaling(&self, platform: Platform) -> Scaling {
        if let Some((p, stem)) = self.seated_game() {
            if p == platform {
                if let Some(s) = self.profiles.get(p, &stem).scaling {
                    return s;
                }
            }
        }
        slot_store::get_scaling(&self.extras, platform)
    }

    pub fn set_scaling(&mut self, platform: Platform, scaling: Scaling) {
        slot_store::set_scaling(&mut self.extras, platform, scaling);
        self.save_extras();
    }

    pub fn integer_scale(&self, platform: Platform) -> bool {
        self.scaling(platform) == Scaling::Integer
    }

    /// Whether the game picture should fill the display frame completely.
    pub fn fills_frame(&self) -> bool {
        let platform = self.playing_platform();
        self.scaling(platform) == Scaling::Fill
            || (!platform.spec().native && self.video_mode == VideoMode::Stretch)
            || (platform.spec().native
                && self.framed_panel()
                && self.scaling(platform) == Scaling::Fit)
    }

    /// Whether sharp-bilinear integer-snap filtering should be applied to the game picture.
    pub fn sharp_bilinear(&self) -> bool {
        let platform = self.playing_platform();
        self.scaling(platform) == Scaling::SharpFit && self.video_mode == VideoMode::Actual
    }

    /// The picture size the compositor draws at a whole multiple, while Integer Scale is on for
    /// the playing console on a 4:3 panel. Stretch wins over it.
    pub fn integer_picture(&self) -> Option<(u32, u32)> {
        let platform = self.playing_platform();
        (self.framed_panel()
            && self.video_mode == VideoMode::Actual
            && self.integer_scale(platform)
            && !platform.spec().native)
            .then(|| platform.picture())
    }

    /// A system drawn at its own size wants whole, square pixels: Integer Scale on, not
    /// stretched. Any panel; the compositor letterboxes a picture too big for one multiple.
    pub fn integer_native(&self) -> bool {
        let platform = self.playing_platform();
        platform.spec().native
            && self.video_mode == VideoMode::Actual
            && self.integer_scale(platform)
    }

    fn integer_row_platform(row: QuickRow) -> Option<Platform> {
        match row {
            QuickRow::IntegerGba => Some(Platform::Gba),
            QuickRow::IntegerGb => Some(Platform::Gb),
            QuickRow::IntegerGbc => Some(Platform::Gbc),
            QuickRow::IntegerNes => Some(Platform::Nes),
            QuickRow::IntegerSnes => Some(Platform::Snes),
            QuickRow::IntegerPs => Some(Platform::PlayStation),
            QuickRow::IntegerNds => Some(Platform::Nds),
            _ => None,
        }
    }

    pub(super) fn screen_value(&self, row: QuickRow) -> Option<Option<QuickValue>> {
        if card_shader_row_platform(row).is_some()
            || row == QuickRow::CardShader
            || row == QuickRow::ThisCardShader
            || row == QuickRow::ShaderSettings
            || row == QuickRow::Scaling
            || row == QuickRow::ThisScaling
            || row == QuickRow::NdsLayout
            || row == QuickRow::PlayNdsLayout
            || row == QuickRow::ThisNdsLayout
            || Self::integer_row_platform(row).is_some()
        {
            // Drawn from `named_value`, since a file name or scaling has no ready-made face.
            return Some(None);
        }
        if row == QuickRow::MotionBlur {
            return Some(Some(QuickValue::flag(
                self.motion_blur(self.playing_platform()),
            )));
        }
        if let Some(platform) = motion_blur_row_platform(row) {
            return Some(Some(QuickValue::flag(self.motion_blur(platform))));
        }
        if let Some(platform) = shader_row_platform(row) {
            return Some(Some(super::shader_value(self.platform_shader(platform))));
        }
        let platform = Self::integer_row_platform(row)?;
        Some(Some(QuickValue::flag(self.integer_scale(platform))))
    }

    /// The core's own frame blending, per console. Off unless asked for.
    pub fn motion_blur(&self, platform: Platform) -> bool {
        self.extras.flag(&motion_blur_key(platform), false)
    }

    /// The blending the running core should have, once after it loads and after each change.
    pub fn take_motion_blur(&mut self) -> Option<bool> {
        self.overlay.take_motion_blur()
    }

    pub(super) fn motion_blur_changed(&mut self) {
        let on = self.motion_blur(self.playing_platform());
        self.overlay.set_motion_blur(on);
    }

    pub(super) fn change_screen(&mut self, row: QuickRow, right: bool) -> bool {
        if row == QuickRow::Scaling {
            let platform = self.playing_platform();
            let to = self.scaling(platform).step(right);
            if self.seated_cart().is_some() && self.game_profile().scaling.is_some() {
                self.set_own(slot_store::ProfileField::Scaling(to));
            } else {
                self.set_scaling(platform, to);
            }
            return true;
        }
        if row == QuickRow::MotionBlur {
            let platform = self.playing_platform();
            let on = !self.motion_blur(platform);
            self.extras.set_flag(&motion_blur_key(platform), on);
            self.motion_blur_changed();
            return true;
        }
        if row == QuickRow::CardShader {
            self.step_card_shader(self.playing_platform(), right);
            return true;
        }
        if row == QuickRow::ThisNdsLayout {
            self.step_nds_layout(true, right);
            return true;
        }
        if row == QuickRow::NdsLayout || row == QuickRow::PlayNdsLayout {
            self.step_nds_layout(false, right);
            return true;
        }
        if let Some(platform) = card_shader_row_platform(row) {
            self.step_card_shader(platform, right);
            return true;
        }
        if let Some(platform) = shader_row_platform(row) {
            let to = self
                .platform_shader(platform)
                .step(platform.spec().shaders, right);
            self.set_platform_shader(platform, to);
            return true;
        }
        if let Some(platform) = motion_blur_row_platform(row) {
            let on = !self.motion_blur(platform);
            self.extras.set_flag(&motion_blur_key(platform), on);
            self.motion_blur_changed();
            return true;
        }
        let Some(platform) = Self::integer_row_platform(row) else {
            return false;
        };
        let to = self.scaling(platform).step(right);
        self.set_scaling(platform, to);
        true
    }

    /// The shader from `Shaders/` a console draws with, by name; `None` for the built-in.
    pub fn card_shader_for(&self, platform: Platform) -> Option<String> {
        if let Some((p, stem)) = self.seated_game() {
            if p == platform {
                if let Some(cs) = self.profiles.get(p, &stem).card_shader {
                    return if cs == "Off" || cs.is_empty() {
                        None
                    } else {
                        Some(cs)
                    };
                }
            }
        }
        self.extras
            .get(&card_shader_key(platform))
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
        self.extras.set(&card_shader_key(platform), to.as_deref());
    }

    /// A menu value that is text slot only knows at run time, for the frontend to draw.
    pub fn named_value(&self, row: QuickRow) -> Option<String> {
        if row == QuickRow::HdmiOutput {
            return Some(self.hdmi_mode().text().to_string());
        }
        if row == QuickRow::Screensaver {
            return Some(self.screensaver_timeout_text());
        }
        if row == QuickRow::ScreensaverStyle {
            return Some(self.screensaver_style().text().to_string());
        }
        if let Some(text) = self.network_text(row) {
            return Some(text);
        }
        if let Some(text) = self.disc_text(row) {
            return Some(text);
        }
        if let Some(text) = self.this_label_text(row) {
            return Some(text);
        }
        if row == QuickRow::CardShader || row == QuickRow::ThisCardShader {
            return Some(self.card_shader().unwrap_or_else(|| "Off".into()));
        }
        if row == QuickRow::Scaling || row == QuickRow::ThisScaling {
            return Some(self.scaling(self.playing_platform()).text().to_string());
        }
        if row == QuickRow::NdsLayout
            || row == QuickRow::PlayNdsLayout
            || row == QuickRow::ThisNdsLayout
        {
            return Some(self.nds_layout());
        }
        if let Some(platform) = Self::integer_row_platform(row) {
            return Some(self.scaling(platform).text().to_string());
        }
        let platform = card_shader_row_platform(row)?;
        Some(
            self.card_shader_for(platform)
                .unwrap_or_else(|| "Off".into()),
        )
    }

    pub fn nds_layout(&self) -> String {
        if let Some((Platform::Nds, stem)) = self.seated_game() {
            if let Some(ref layout) = self.profiles.get(Platform::Nds, &stem).nds_layout {
                return layout.clone();
            }
        }
        self.extras
            .get(NDS_LAYOUT_KEY)
            .map(str::to_string)
            .unwrap_or_else(|| "Top Dominant".into())
    }

    pub fn step_nds_layout(&mut self, is_this_game: bool, right: bool) {
        let current = self.nds_layout();
        let at = NDS_LAYOUTS.iter().position(|&l| l == current).unwrap_or(0);
        let n = NDS_LAYOUTS.len();
        let next_idx = if right {
            (at + 1) % n
        } else {
            (at + n - 1) % n
        };
        let next = NDS_LAYOUTS[next_idx].to_string();

        if is_this_game
            || (self.seated_cart().is_some() && self.game_profile().nds_layout.is_some())
        {
            self.set_own(slot_store::ProfileField::NdsLayout(next.clone()));
        } else {
            self.extras.set(NDS_LAYOUT_KEY, Some(&next));
            self.save_extras();
        }

        if self.playing_platform() == Platform::Nds {
            self.nds_layout_pending = Some(next);
        }
    }
}

pub const NDS_LAYOUTS: [&str; 6] = [
    "Top Dominant",
    "Top Only",
    "Bottom Only",
    "Vertical",
    "Horizontal",
    "Picture in Picture",
];

const NDS_LAYOUT_KEY: &str = "nds_layout";
