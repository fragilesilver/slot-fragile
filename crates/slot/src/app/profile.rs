//! Per-game settings: the shader, colour correction and fast forward one game keeps for
//! itself, in `Config/game_profiles.txt`, set from the in-game Settings → This Game page.
//! A setting a game has not chosen follows the settings menu. Once a game has its own value,
//! the in-game Screen and Gameplay rows and the shortcuts change that value, so what the
//! player turns is always what the game shows. Store ported from suprsomething/slot.advance.

use slot_store::{GameProfile, GameProfiles, Platform, ProfileField, Shader};
use slot_ui::{QuickRow, QuickValue, Toast};

use super::{ff_next, shader_value, App};

impl App {
    pub(super) fn load_profiles(&mut self, root: &std::path::Path) {
        self.profiles = GameProfiles::read(root);
    }

    fn save_profiles(&self) {
        let Some(root) = &self.root else {
            return;
        };
        let written = self.profiles.write(root);
        if let Err(e) = &written {
            eprintln!("slot: game_profiles.txt: {e}");
        }
        self.card_writes.wrote("game_profiles.txt", &written);
    }

    fn seated_game(&self) -> Option<(Platform, String)> {
        self.seated_cart().map(|c| (c.platform, c.stem.clone()))
    }

    /// What the seated game keeps for itself; nothing when no game is in.
    pub fn game_profile(&self) -> GameProfile {
        self.seated_game()
            .map(|(p, stem)| self.profiles.get(p, &stem))
            .unwrap_or_default()
    }

    pub(super) fn own_colour_for(&self, platform: Platform) -> Option<bool> {
        self.seated_cart()
            .filter(|c| c.platform == platform)
            .and(self.game_profile().colour)
    }

    fn set_own(&mut self, field: ProfileField) {
        let Some((platform, stem)) = self.seated_game() else {
            return;
        };
        if self.profiles.set(platform, &stem, field) {
            self.save_profiles();
        }
    }

    /// Which of the rows that edit a playing platform's shader or colour this one is.
    fn edits_shader(&self, row: QuickRow) -> bool {
        matches!(
            (row, self.playing_platform()),
            (QuickRow::ThisShader | QuickRow::Shader, _)
                | (QuickRow::GbaShader, Platform::Gba)
                | (QuickRow::GbShader, Platform::Gb)
                | (QuickRow::GbcShader, Platform::Gbc)
        )
    }

    fn edits_colour(&self, row: QuickRow) -> bool {
        matches!(
            (row, self.playing_platform()),
            (QuickRow::ThisColour | QuickRow::ColourCorrection, _)
                | (QuickRow::GbaColourCorrection, Platform::Gba)
                | (QuickRow::GbColourCorrection, Platform::Gb)
                | (QuickRow::GbcColourCorrection, Platform::Gbc)
        )
    }

    /// The value a This Game row shows, or a Screen or Gameplay row whose setting the seated
    /// game keeps for itself. `None` leaves the row to upstream.
    pub(super) fn profile_value(&self, row: QuickRow) -> Option<Option<QuickValue>> {
        self.seated_cart()?;
        let own = self.game_profile();
        let this = QuickRow::THIS_GAME.contains(&row);
        match row {
            QuickRow::ThisDefaults => Some(Some(QuickValue::flag(own.is_empty()))),
            r if self.edits_shader(r) && (this || own.shader.is_some()) => {
                Some(Some(shader_value(self.screen_shader())))
            }
            r if self.edits_colour(r) && (this || own.colour.is_some()) => {
                Some(Some(QuickValue::flag(self.colour_correction())))
            }
            QuickRow::ThisFfSpeed => Some(QuickValue::speed(self.ff_speed())),
            QuickRow::FastForward if own.ff_speed.is_some() => {
                Some(QuickValue::speed(self.ff_speed()))
            }
            QuickRow::ThisFfSound => Some(Some(QuickValue::flag(self.ff_sound()))),
            QuickRow::FastForwardSound if own.ff_sound.is_some() => {
                Some(Some(QuickValue::flag(self.ff_sound())))
            }
            _ => None,
        }
    }

    /// Changes the seated game's own setting for a This Game row, or for a Screen or Gameplay
    /// row whose setting it already keeps, and says so; anything else is left to upstream.
    pub(super) fn change_profile(&mut self, row: QuickRow, right: bool) -> bool {
        if self.seated_cart().is_none() {
            return false;
        }
        let own = self.game_profile();
        let this = QuickRow::THIS_GAME.contains(&row);
        match row {
            QuickRow::ThisDefaults => {
                let Some((platform, stem)) = self.seated_game() else {
                    return true;
                };
                if self.profiles.clear(platform, &stem) {
                    self.save_profiles();
                    self.colour_pending = Some(self.colour_correction());
                }
            }
            r if self.edits_shader(r) && (this || own.shader.is_some()) => {
                self.step_own_shader(right);
            }
            r if self.edits_colour(r) && (this || own.colour.is_some()) => {
                let on = !self.colour_correction();
                self.set_own(ProfileField::Colour(on));
                self.colour_pending = Some(on);
            }
            QuickRow::ThisFfSpeed => self.step_own_ff(right),
            QuickRow::FastForward if own.ff_speed.is_some() => self.step_own_ff(right),
            QuickRow::ThisFfSound => self.toggle_own_ff_sound(),
            QuickRow::FastForwardSound if own.ff_sound.is_some() => self.toggle_own_ff_sound(),
            _ => return false,
        }
        true
    }

    fn step_own_shader(&mut self, right: bool) {
        let choices: &[Shader] = match self.playing_platform() {
            Platform::Gba => &Shader::GBA,
            Platform::Gb | Platform::Gbc => &Shader::GB,
        };
        let to = self.screen_shader().step(choices, right);
        self.set_own(ProfileField::Shader(to));
    }

    fn step_own_ff(&mut self, right: bool) {
        let to = ff_next(self.ff_speed(), right);
        self.set_own(ProfileField::FfSpeed(to));
    }

    fn toggle_own_ff_sound(&mut self) {
        let on = !self.ff_sound();
        self.set_own(ProfileField::FfSound(on));
    }

    /// The shader shortcut, for a game that keeps its own shader. Says whether it took it.
    pub(super) fn shortcut_own_shader(&mut self, right: bool) -> bool {
        if self.game_profile().shader.is_none() {
            return false;
        }
        self.step_own_shader(right);
        let said = match self.screen_shader() {
            Shader::Off => Toast::ShaderOff,
            Shader::Lcd3x => Toast::ShaderLcd3x,
            Shader::Grid => Toast::ShaderGrid,
            Shader::Dot => Toast::ShaderDot,
            Shader::Simpletex => Toast::ShaderSimpletex,
        };
        self.hud.toast(said, self.now());
        true
    }
}
