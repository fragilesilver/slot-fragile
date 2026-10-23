//! The Gambatte core's options, for Game Boy and Game Boy Color games. Ported from
//! roygravy/slot@e1bdb53, crates/slot/src/core.rs, which runs every GB and GBC game on it;
//! Fragile keeps mGBA beside it (the core picker chooses, and link play needs mGBA's link
//! mode). Every key and value was checked against gambatte-libretro@d9d6cd0's
//! libgambatte/libretro/libretro_core_options.h.

use slot_retro::LibretroCore;
use slot_store::{Core, GbPalette};

/// Read during `retro_init`, so it is given before the core starts rather than after: the boot
/// logo plays when the card has the boot ROM.
pub const BEFORE_INIT: &[(&str, &str)] = &[("gambatte_gb_bootloader", "enabled")];

pub fn colour_option(on: bool) -> (&'static str, &'static str) {
    (
        "gambatte_gbc_color_correction",
        if on { "GBC only" } else { "disabled" },
    )
}

pub fn motion_blur_option(on: bool) -> (&'static str, &'static str) {
    (
        "gambatte_mix_frames",
        if on { "lcd_ghosting" } else { "disabled" },
    )
}

/// mGBA's name for a palette, as slot keeps it, in Gambatte's spelling.
pub fn palette_name(palette: GbPalette) -> String {
    let name = palette.core_name();
    let base = palette.label();
    if let Some(sgb) = name.strip_prefix("SGB ") {
        let code: String = sgb.chars().filter(char::is_ascii_alphanumeric).collect();
        return format!("SGB - {code}");
    }
    match base {
        "DMG Green" => "GB - DMG".into(),
        "GB Pocket" => "GB - Pocket".into(),
        "GB Light" => "GB - Light".into(),
        "Grayscale" | "GBC Gray" => "GBC - Grayscale".into(),
        "GBC Reverse" => "GBC - Inverted".into(),
        "GBC Pale Yellow" => "GBC - Yellow".into(),
        other => match other.strip_prefix("GBC ") {
            Some(colour) => format!("GBC - {colour}"),
            None => "GB - DMG".into(),
        },
    }
}

/// The options that put `palette` on the running core: mGBA's one, as upstream sends it, or
/// Gambatte's two. A core with no Game Boy palettes gets none.
pub fn palette_options(which: Core, palette: GbPalette) -> Vec<(&'static str, String)> {
    match which {
        // Fragile: off its hardware preset too, for a game opened with palettes off.
        Core::Mgba => vec![
            ("mgba_gb_colors_preset", "0".to_string()),
            ("mgba_gb_colors", palette.core_name().to_string()),
        ],
        Core::Gambatte => vec![
            ("gambatte_gb_colorization", "internal".to_string()),
            ("gambatte_gb_internal_palette", palette_name(palette)),
        ],
        Core::Gpsp | Core::Fceumm | Core::Snes9x | Core::Supafaust | Core::PcsxRearmed => {
            Vec::new()
        }
    }
}

/// Everything Gambatte is told once it is open. A chosen palette colours a monochrome game;
/// without one Gambatte colours it as a Game Boy Color would.
pub fn apply_options(
    core: &mut LibretroCore,
    which: Core,
    colour: bool,
    palette: Option<GbPalette>,
) {
    if which != Core::Gambatte {
        return;
    }
    for (key, value) in BEFORE_INIT {
        core.set_option(key, value);
    }
    core.set_option("gambatte_gb_hwmode", "Auto");
    core.set_option("gambatte_gbc_color_correction_mode", "accurate");
    core.set_option("gambatte_gbc_frontlight_position", "central");
    match palette {
        Some(p) => {
            for (key, value) in palette_options(which, p) {
                core.set_option(key, &value);
            }
        }
        None => core.set_option("gambatte_gb_colorization", "auto"),
    }
    let (key, value) = colour_option(colour);
    core.set_option(key, value);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// gambatte-libretro@d9d6cd0's `gambatte_gb_internal_palette` values that slot can name.
    const GAMBATTE: &[&str] = &[
        "GB - DMG",
        "GB - Pocket",
        "GB - Light",
        "GBC - Blue",
        "GBC - Brown",
        "GBC - Dark Blue",
        "GBC - Dark Brown",
        "GBC - Dark Green",
        "GBC - Grayscale",
        "GBC - Green",
        "GBC - Inverted",
        "GBC - Orange",
        "GBC - Pastel Mix",
        "GBC - Red",
        "GBC - Yellow",
    ];

    #[test]
    fn every_palette_slot_offers_is_one_gambatte_has() {
        for p in GbPalette::all() {
            let name = palette_name(p);
            let sgb = name.strip_prefix("SGB - ").is_some_and(|c| {
                c.len() == 2
                    && ('1'..='4').contains(&c.chars().next().unwrap())
                    && ('A'..='H').contains(&c.chars().nth(1).unwrap())
            });
            assert!(
                sgb || GAMBATTE.contains(&name.as_str()),
                "{} became {name}, which Gambatte does not have",
                p.core_name()
            );
        }
    }

    #[test]
    fn the_named_ones_land_where_they_should() {
        let by = |core_name: &str| {
            GbPalette::all()
                .find(|p| p.core_name() == core_name)
                .map(palette_name)
        };
        assert_eq!(by("DMG Green").as_deref(), Some("GB - DMG"));
        assert_eq!(by("GBC Reverse →B").as_deref(), Some("GBC - Inverted"));
        assert_eq!(by("SGB 3-F").as_deref(), Some("SGB - 3F"));
        assert_eq!(by("GBC Dark Green →A").as_deref(), Some("GBC - Dark Green"));
    }
}
