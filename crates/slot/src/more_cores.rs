//! Fragile: what the cores of the systems past the Game Boy family are told (Phase 12). Their
//! declared defaults mostly suit a handheld (Supafaust renders on a thread of its own), but slot
//! answers a core only about options it has set, and a core asked about nothing falls back on
//! its code's own value, which is not always the one it declares. Each key and value was checked
//! against the pinned commits in the taskfile.

use slot_retro::RetroCore;
use slot_store::Core;

/// The options each core is told once it is open.
pub fn options(which: Core) -> &'static [(&'static str, &'static str)] {
    match which {
        // Declared as 8 by default, but 0 unless asked: the NES's top and bottom 8 lines,
        // which a television never showed.
        Core::Fceumm => &[
            ("fceumm_overscan_v_top", "8"),
            ("fceumm_overscan_v_bottom", "8"),
        ],
        _ => &[],
    }
}

pub fn apply_options(core: &mut dyn RetroCore, which: Core) {
    for (key, value) in options(which) {
        core.set_option(key, value);
    }
}

/// The option that lets a core skip frames when the audio runs short, set to do so. FCEUmm,
/// Snes9x and Supafaust have none; PCSX ReARMed calls it `pcsx_rearmed_frameskip_type`.
pub fn frameskip_option(which: Core) -> Option<(String, &'static str)> {
    match which {
        Core::Mgba | Core::Gpsp => Some((format!("{}_frameskip", which.as_str()), "auto")),
        Core::PcsxRearmed => Some(("pcsx_rearmed_frameskip_type".into(), "auto")),
        Core::Gambatte | Core::Fceumm | Core::Snes9x | Core::Supafaust | Core::Dsperate => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mgba_and_gpsp_keep_upstreams_frame_skip() {
        for core in [Core::Mgba, Core::Gpsp] {
            assert_eq!(
                frameskip_option(core),
                Some((format!("{}_frameskip", core.as_str()), "auto"))
            );
        }
    }
}
