//! What slot needs to know about each system, as one row of data, so a new system is a variant
//! on `Platform` and a row here rather than a branch in every file that cares. Ported from
//! cloudwithax/unslot@6339c18, crates/slot-store/src/platform.rs, cut to the systems Fragile
//! runs.

use crate::core::Core;
use crate::platform::Platform;
use crate::slot_state::Shader;

/// How the scanner finds a game's title, and for the GBA its four-letter code, in the ROM.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Header {
    Gba,
    Gb,
    /// No title worth reading in the file; the shelf shows the file's name.
    None,
}

#[derive(Debug)]
pub struct Spec {
    pub name: &'static str,
    /// A shorter name where the whole one will not fit; empty when the name is short already.
    pub short: &'static str,
    /// The folder under Games, Labels, Saves, States and Bezels, and the key in config files.
    pub dir: &'static str,
    pub extensions: &'static [&'static str],
    /// Whether the core's picture is drawn at its own size and aspect. When it is not, the game
    /// is centred in the Game Boy family's 240x160 canvas and `picture` says where it sits.
    pub native: bool,
    /// The game's picture inside the 240x160 canvas the Game Boy family is drawn into.
    pub picture: (u32, u32),
    pub header: Header,
    /// The cores that run it, the default first.
    pub cores: &'static [Core],
    pub shaders: &'static [Shader],
    pub shader: Shader,
    /// Whether its core can correct the colour of a handheld's LCD.
    pub colour_correction: bool,
    /// Whether it has a default bezel and a Bezels folder of its own.
    pub bezels: bool,
    /// Whether its games read L and R; when they do not, slot keeps the shoulders.
    pub shoulders: bool,
    /// Whether its games read L2 and R2; when they do, rewind and fast forward give them up.
    pub triggers: bool,
    /// Whether X and Y can fire A and B as turbo buttons. Not on a pad that has its own X and Y.
    pub turbo: bool,
    /// Whether its save states are small enough to keep a rewind buffer of.
    pub rewind: bool,
    /// Whether two of it can play over the wireless link.
    pub link: bool,
    /// The files its core boots best from, in the BIOS folder.
    pub bios: &'static [&'static str],
    /// Whether its games come on discs: each may sit in a folder of its own, and an .m3u stands
    /// for the discs it lists.
    pub discs: bool,
}

/// What a system drawn at its own size shares unless its row says otherwise.
const CONSOLE: Spec = Spec {
    name: "",
    short: "",
    dir: "",
    extensions: &[],
    native: true,
    picture: (240, 160),
    header: Header::None,
    cores: &[],
    shaders: &[Shader::Off, Shader::Grid, Shader::Lcd3x],
    shader: Shader::Off,
    colour_correction: false,
    bezels: false,
    shoulders: false,
    triggers: false,
    turbo: true,
    rewind: true,
    link: false,
    bios: &[],
    discs: false,
};

pub(crate) const SPECS: [Spec; Platform::COUNT] = [
    Spec {
        name: "Game Boy Advance",
        dir: "GBA",
        extensions: &["gba"],
        native: false,
        picture: (240, 160),
        header: Header::Gba,
        cores: &[Core::Mgba, Core::Gpsp],
        shaders: &Shader::GBA,
        shader: Shader::Grid,
        colour_correction: true,
        shoulders: true,
        link: true,
        ..CONSOLE
    },
    Spec {
        name: "Game Boy",
        dir: "GB",
        extensions: &["gb", "gbc"],
        native: false,
        picture: (160, 144),
        header: Header::Gb,
        cores: &[Core::Gambatte, Core::Mgba],
        shaders: &Shader::GB,
        shader: Shader::Simpletex,
        colour_correction: true,
        bezels: true,
        ..CONSOLE
    },
    Spec {
        name: "Game Boy Color",
        dir: "GBC",
        extensions: &["gb", "gbc"],
        native: false,
        picture: (160, 144),
        header: Header::Gb,
        cores: &[Core::Gambatte, Core::Mgba],
        shaders: &Shader::GB,
        shader: Shader::Simpletex,
        colour_correction: true,
        bezels: true,
        ..CONSOLE
    },
    // --- extras: the rows below are after cloudwithax/unslot's ---
    Spec {
        name: "Nintendo Entertainment System",
        short: "NES",
        dir: "NES",
        extensions: &["nes", "unf", "unif"],
        cores: &[Core::Fceumm],
        ..CONSOLE
    },
    Spec {
        name: "Super Nintendo",
        short: "SNES",
        dir: "SNES",
        extensions: &["sfc", "smc"],
        cores: &[Core::Snes9x, Core::Supafaust],
        shoulders: true,
        turbo: false,
        ..CONSOLE
    },
    Spec {
        name: "PlayStation",
        short: "PS1",
        dir: "PS",
        extensions: &["cue", "chd", "pbp", "m3u"],
        cores: &[Core::PcsxRearmed],
        shoulders: true,
        triggers: true,
        turbo: false,
        rewind: false,
        bios: &[
            "scph5501.bin",
            "scph1001.bin",
            "scph7001.bin",
            "scph101.bin",
            "psxonpsp660.bin",
        ],
        discs: true,
        ..CONSOLE
    },
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_platform_sits_at_its_own_index() {
        for (at, p) in Platform::ALL.into_iter().enumerate() {
            assert_eq!(p.index(), at, "{p:?} is out of place in ALL");
        }
    }

    #[test]
    fn every_spec_is_complete() {
        for p in Platform::ALL {
            let s = p.spec();
            assert!(!s.name.is_empty() && !s.dir.is_empty(), "{p:?} has no name");
            assert!(!s.cores.is_empty(), "{p:?} has no core");
            assert!(
                s.shaders.contains(&s.shader),
                "{p:?} defaults to a shader it does not offer"
            );
            assert!(!s.extensions.is_empty(), "{p:?} opens no files");
            assert!(
                s.native || s.header != Header::None,
                "{p:?} is drawn in the Game Boy canvas but is not a Game Boy"
            );
        }
    }

    #[test]
    fn dir_names_are_unique_and_round_trip() {
        for p in Platform::ALL {
            assert_eq!(Platform::from_key(&p.key()), Some(p));
        }
        assert_eq!(Platform::from_key("nope"), None);
    }
}
