//! Who made each core and under what licence. A match, so a core added to `Core` does not
//! build until it is credited here; `tests/core_credits.rs` checks each licence file is in
//! `licenses/` and each core is in CREDITS.md. Fragile's.

use crate::core::Core;

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct CoreCredit {
    /// The core's own name, as the core picker shows it.
    pub name: &'static str,
    pub by: &'static str,
    /// The SPDX name of its licence.
    pub license: &'static str,
    /// Its licence text, in `licenses/` and in the release's `licenses` folder.
    pub file: &'static str,
    pub source: &'static str,
}

pub fn credit(core: Core) -> CoreCredit {
    match core {
        Core::Mgba => CoreCredit {
            name: core.text(),
            by: "Jeffrey Pfau and contributors",
            license: "MPL-2.0",
            file: "mgba-MPL-2.0.txt",
            source: "https://github.com/libretro/mgba",
        },
        Core::Gpsp => CoreCredit {
            name: core.text(),
            by: "Exophase, notaz and the libretro team",
            license: "GPL-2.0",
            file: "gpsp-GPL-2.0.txt",
            source: "https://github.com/libretro/gpsp",
        },
        Core::Fceumm => CoreCredit {
            name: core.text(),
            by: "the FCE Ultra teams and the libretro team",
            license: "GPL-2.0",
            file: "fceumm-GPL-2.0.txt",
            source: "https://github.com/libretro/libretro-fceumm",
        },
        Core::Snes9x => CoreCredit {
            name: core.text(),
            by: "the Snes9x team and the libretro team",
            license: "Snes9x (non-commercial)",
            file: "snes9x-LICENSE.txt",
            source: "https://github.com/libretro/snes9x",
        },
        Core::Supafaust => CoreCredit {
            name: core.text(),
            by: "the Mednafen team and the libretro team",
            license: "GPL-2.0",
            file: "supafaust-GPL-2.0.txt",
            source: "https://github.com/libretro/supafaust",
        },
        Core::PcsxRearmed => CoreCredit {
            name: core.text(),
            by: "notaz, the PCSX teams and the libretro team",
            license: "GPL-2.0",
            file: "pcsx_rearmed-GPL-2.0.txt",
            source: "https://github.com/libretro/pcsx_rearmed",
        },
        Core::Gambatte => CoreCredit {
            name: core.text(),
            by: "sinamas and the libretro team",
            license: "GPL-2.0",
            file: "gambatte-GPL-2.0.txt",
            source: "https://github.com/libretro/gambatte-libretro",
        },
    }
}
