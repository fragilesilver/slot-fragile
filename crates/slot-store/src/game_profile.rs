// Ported from suprsomething/slot.advance@bdf47c3, crates/slot-store/src/game_profile.rs, with
// shader and colour correction falling back to the defaults of the game's own platform.

//! Per-game preferences: the screen shader, colour correction, fast-forward speed and
//! fast-forward sound one game was last played with.
//!
//! `Config/game_profiles.txt`, in the same `key = value` form as `Config/selected_core.txt`,
//! one line per setting a game has actually changed:
//!
//! ```text
//! GBA/Mother 3 (Japan).shader = lcd3x
//! GBA/Mother 3 (Japan).colour = on
//! GBA/Mother 3 (Japan).ff = 3
//! GBA/Mother 3 (Japan).ffsound = off
//! GBA/Mother 3 (Japan).label = logo
//! ```
//!
//! A setting with no line falls back to the global default in `Config/slot.state`, which the
//! carousel's settings menu edits, so a game that has never been changed simply follows those.
//! It lives in `Config/` so replacing `System/` on an update leaves it alone. Delete a game's
//! lines on a computer to put it back on the defaults.

use std::collections::BTreeMap;
use std::path::Path;

use crate::label_style::LabelStyle;
use crate::platform::Platform;
use crate::scaling::Scaling;
use crate::slot_state::{Shader, SlotState, FF_SPEEDS};

pub const GAME_PROFILES_FILE: &str = "Config/game_profiles.txt";

const HEADER: &str =
    "# Per-game settings written by Fragile's This Game page. A missing line means\n\
# the game uses the default from the settings menu. Delete a game's lines to reset it.\n";

/// What one game has chosen. `None` is "use the default".
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct GameProfile {
    pub shader: Option<Shader>,
    pub colour: Option<bool>,
    pub ff_speed: Option<u8>,
    pub ff_sound: Option<bool>,
    /// Fragile: how Game Art draws its label (Phase 13).
    pub label: Option<LabelStyle>,
    pub scaling: Option<Scaling>,
    pub card_shader: Option<String>,
    pub nds_layout: Option<String>,
}

/// One change to a game's profile.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum ProfileField {
    Shader(Shader),
    Colour(bool),
    FfSpeed(u8),
    FfSound(bool),
    Label(LabelStyle),
    Scaling(Scaling),
    CardShader(String),
    NdsLayout(String),
}

/// The settings a game actually runs with: its own where it has them, the defaults elsewhere.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Resolved {
    pub shader: Shader,
    pub colour: bool,
    pub ff_speed: u8,
    pub ff_sound: bool,
}

impl GameProfile {
    pub fn is_empty(&self) -> bool {
        *self == GameProfile::default()
    }

    pub fn apply(&mut self, field: ProfileField) {
        match field {
            ProfileField::Shader(s) => self.shader = Some(s),
            ProfileField::Colour(on) => self.colour = Some(on),
            ProfileField::FfSpeed(f) => self.ff_speed = Some(f),
            ProfileField::FfSound(on) => self.ff_sound = Some(on),
            ProfileField::Label(style) => self.label = Some(style),
            ProfileField::Scaling(scaling) => self.scaling = Some(scaling),
            ProfileField::CardShader(shader) => self.card_shader = Some(shader),
            ProfileField::NdsLayout(layout) => self.nds_layout = Some(layout),
        }
    }

    pub fn resolve(&self, platform: Platform, defaults: &SlotState) -> Resolved {
        let extras = crate::ExtrasState::default();
        let shader_default = crate::platform_settings::shader(defaults, &extras, platform);
        let colour_default = crate::platform_settings::colour(defaults, &extras, platform);
        Resolved {
            shader: self.shader.unwrap_or(shader_default),
            colour: self.colour.unwrap_or(colour_default),
            ff_speed: self.ff_speed.unwrap_or(defaults.ff_speed),
            ff_sound: self.ff_sound.unwrap_or(defaults.ff_sound),
        }
    }
}

/// Every game's profile, as read from and written back to `GAME_PROFILES_FILE`.
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct GameProfiles {
    games: BTreeMap<String, GameProfile>,
}

pub fn profile_key(platform: Platform, stem: &str) -> String {
    format!("{}/{stem}", platform.dir_name())
}

/// A stem the `key = value` form can hold on one line and read back unchanged.
fn storable(stem: &str) -> bool {
    !stem.is_empty()
        && !stem.contains('=')
        && !stem.contains('\n')
        && !stem.contains('\r')
        && stem.trim() == stem
}

fn flag(value: &str) -> Option<bool> {
    match value {
        "on" => Some(true),
        "off" => Some(false),
        _ => None,
    }
}

fn flag_str(on: bool) -> &'static str {
    if on {
        "on"
    } else {
        "off"
    }
}

fn platform_of(key: &str) -> Option<Platform> {
    let dir = key.split_once('/')?.0;
    Platform::ALL.into_iter().find(|p| p.dir_name() == dir)
}

fn allowed_shaders(platform: Platform) -> &'static [Shader] {
    platform.spec().shaders
}

impl GameProfiles {
    pub fn read(root: &Path) -> Self {
        std::fs::read_to_string(root.join(GAME_PROFILES_FILE))
            .map(|text| Self::parse(&text))
            .unwrap_or_default()
    }

    /// Lines that name no known platform, no known setting, or a value that setting cannot
    /// take are skipped, so a hand edit gone wrong costs that one line and nothing else.
    pub fn parse(text: &str) -> Self {
        let mut games: BTreeMap<String, GameProfile> = BTreeMap::new();
        // Line by line rather than through `ini::parse`, whose map keeps only the last of a
        // repeated key even when that one is unreadable: here a later good line wins and a
        // later bad one is ignored.
        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
                continue;
            }
            let Some((key, value)) = line.split_once('=') else {
                continue;
            };
            let (key, value) = (key.trim(), value.trim().to_string());
            let Some((game, field)) = key.rsplit_once('.') else {
                continue;
            };
            let Some(platform) = platform_of(game) else {
                continue;
            };
            if game.split_once('/').is_none_or(|(_, stem)| stem.is_empty()) {
                continue;
            }
            let field = match field {
                "shader" => Shader::parse(&value)
                    .filter(|s| allowed_shaders(platform).contains(s))
                    .map(ProfileField::Shader),
                "colour" => flag(&value).map(ProfileField::Colour),
                "ff" => value
                    .parse::<u8>()
                    .ok()
                    .filter(|f| FF_SPEEDS.contains(f))
                    .map(ProfileField::FfSpeed),
                "ffsound" => flag(&value).map(ProfileField::FfSound),
                "label" => LabelStyle::parse(&value)
                    .filter(|s| LabelStyle::of(platform).contains(s))
                    .map(ProfileField::Label),
                "scaling" => Scaling::parse(&value).map(ProfileField::Scaling),
                "card_shader" => Some(ProfileField::CardShader(value)),
                "nds_layout" => Some(ProfileField::NdsLayout(value)),
                _ => None,
            };
            if let Some(field) = field {
                games.entry(game.to_string()).or_default().apply(field);
            }
        }
        GameProfiles { games }
    }

    pub fn get(&self, platform: Platform, stem: &str) -> GameProfile {
        self.games
            .get(&profile_key(platform, stem))
            .cloned()
            .unwrap_or_default()
    }

    /// Records one change. Answers false, and changes nothing, for a stem the file could not
    /// hold, which is then simply not remembered rather than written wrong.
    pub fn set(&mut self, platform: Platform, stem: &str, field: ProfileField) -> bool {
        if !storable(stem) {
            return false;
        }
        self.games
            .entry(profile_key(platform, stem))
            .or_default()
            .apply(field);
        true
    }

    /// Puts one game back on the defaults. Answers whether it had settings of its own.
    pub fn clear(&mut self, platform: Platform, stem: &str) -> bool {
        self.games.remove(&profile_key(platform, stem)).is_some()
    }

    pub fn len(&self) -> usize {
        self.games.len()
    }

    pub fn is_empty(&self) -> bool {
        self.games.is_empty()
    }

    /// The whole file, sorted by game then setting, so the same profiles always write the
    /// same bytes.
    pub fn to_text(&self) -> String {
        let mut out = String::from(HEADER);
        for (game, p) in &self.games {
            if let Some(s) = p.shader {
                out.push_str(&format!("{game}.shader = {}\n", s.name()));
            }
            if let Some(on) = p.colour {
                out.push_str(&format!("{game}.colour = {}\n", flag_str(on)));
            }
            if let Some(f) = p.ff_speed {
                out.push_str(&format!("{game}.ff = {f}\n"));
            }
            if let Some(on) = p.ff_sound {
                out.push_str(&format!("{game}.ffsound = {}\n", flag_str(on)));
            }
            if let Some(style) = p.label {
                out.push_str(&format!("{game}.label = {}\n", style.key()));
            }
            if let Some(scaling) = p.scaling {
                out.push_str(&format!("{game}.scaling = {}\n", scaling.key()));
            }
            if let Some(card_shader) = &p.card_shader {
                out.push_str(&format!("{game}.card_shader = {card_shader}\n"));
            }
            if let Some(nds_layout) = &p.nds_layout {
                out.push_str(&format!("{game}.nds_layout = {nds_layout}\n"));
            }
        }
        out
    }

    pub fn write(&self, root: &Path) -> std::io::Result<()> {
        let path = root.join(GAME_PROFILES_FILE);
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        crate::atomic::atomic_write(&path, self.to_text().as_bytes())
    }
}
