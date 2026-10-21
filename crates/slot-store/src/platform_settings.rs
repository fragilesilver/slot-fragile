//! Each system's shader, colour correction and bezel. The Game Boy family keeps them where
//! upstream's slot.state has them (`shader_gba` and the rest), so that file never changes shape;
//! a system Fragile adds keeps them in extras.state as `shader.<dir>`, `colour.<dir>` and
//! `bezel.<dir>`, and starts from its row in `spec.rs`.

use crate::{BezelPick, ExtrasState, Platform, Shader, SlotState};

fn key(what: &str, platform: Platform) -> String {
    format!("{what}.{}", platform.key())
}

pub fn shader(state: &SlotState, extras: &ExtrasState, platform: Platform) -> Shader {
    match platform {
        Platform::Gba => state.shader_gba,
        Platform::Gb => state.shader_gb,
        Platform::Gbc => state.shader_gbc,
        #[allow(unreachable_patterns)]
        _ => extras
            .get(&key("shader", platform))
            .and_then(Shader::parse)
            .filter(|s| platform.spec().shaders.contains(s))
            .unwrap_or(platform.spec().shader),
    }
}

pub fn set_shader(state: &mut SlotState, extras: &mut ExtrasState, platform: Platform, to: Shader) {
    match platform {
        Platform::Gba => state.shader_gba = to,
        Platform::Gb => state.shader_gb = to,
        Platform::Gbc => state.shader_gbc = to,
        #[allow(unreachable_patterns)]
        _ => extras.set(&key("shader", platform), Some(to.name())),
    }
}

pub fn colour(state: &SlotState, extras: &ExtrasState, platform: Platform) -> bool {
    match platform {
        Platform::Gba => state.colour_correction_gba,
        Platform::Gb => state.colour_correction_gb,
        Platform::Gbc => state.colour_correction_gbc,
        #[allow(unreachable_patterns)]
        _ => extras.flag(&key("colour", platform), false),
    }
}

pub fn set_colour(state: &mut SlotState, extras: &mut ExtrasState, platform: Platform, on: bool) {
    match platform {
        Platform::Gba => state.colour_correction_gba = on,
        Platform::Gb => state.colour_correction_gb = on,
        Platform::Gbc => state.colour_correction_gbc = on,
        #[allow(unreachable_patterns)]
        _ => extras.set_flag(&key("colour", platform), on),
    }
}

pub fn bezel(state: &SlotState, extras: &ExtrasState, platform: Platform) -> BezelPick {
    match platform {
        Platform::Gba => state.bezel_gba.clone(),
        Platform::Gb => state.bezel_gb.clone(),
        Platform::Gbc => state.bezel_gbc.clone(),
        #[allow(unreachable_patterns)]
        _ => extras
            .get(&key("bezel", platform))
            .map_or(BezelPick::None, BezelPick::parse),
    }
}

pub fn set_bezel(
    state: &mut SlotState,
    extras: &mut ExtrasState,
    platform: Platform,
    to: BezelPick,
) {
    match platform {
        Platform::Gba => state.bezel_gba = to,
        Platform::Gb => state.bezel_gb = to,
        Platform::Gbc => state.bezel_gbc = to,
        #[allow(unreachable_patterns)]
        _ => extras.set(&key("bezel", platform), Some(&to.key())),
    }
}

/// Whether the setting lives in extras.state, so a change to it is saved there.
pub fn in_extras(platform: Platform) -> bool {
    !matches!(platform, Platform::Gba | Platform::Gb | Platform::Gbc)
}
