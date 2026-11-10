use std::collections::HashMap;
use std::path::Path;

use crate::Platform;

pub const SELECTED_CORE_FILE: &str = "Config/selected_core.txt";

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Core {
    #[default]
    Mgba,
    Gpsp,
    // --- extras ---
    Gambatte,
    Fceumm,
    Snes9x,
    Supafaust,
    PcsxRearmed,
    Dsperate,
}

impl Core {
    pub const ALL: [Core; 8] = [
        Core::Mgba,
        Core::Gpsp,
        Core::Gambatte,
        Core::Fceumm,
        Core::Snes9x,
        Core::Supafaust,
        Core::PcsxRearmed,
        Core::Dsperate,
    ];

    pub fn as_str(&self) -> &'static str {
        match self {
            Core::Mgba => "mgba",
            Core::Gpsp => "gpsp",
            Core::Gambatte => "gambatte",
            Core::Fceumm => "fceumm",
            Core::Snes9x => "snes9x",
            Core::Supafaust => "mednafen_supafaust",
            Core::PcsxRearmed => "pcsx_rearmed",
            Core::Dsperate => "dsperate",
        }
    }

    pub fn index(self) -> usize {
        self as usize
    }

    pub fn text(self) -> &'static str {
        match self {
            Core::Mgba => "mGBA",
            Core::Gpsp => "gpSP",
            Core::Gambatte => "Gambatte",
            Core::Fceumm => "FCEUmm",
            Core::Snes9x => "Snes9x",
            Core::Supafaust => "Supafaust",
            Core::PcsxRearmed => "PCSX ReARMed",
            Core::Dsperate => "DSperate",
        }
    }

    pub fn parse(s: &str) -> Option<Core> {
        let s = s.trim();
        Core::ALL
            .into_iter()
            .find(|c| c.as_str().eq_ignore_ascii_case(s))
    }

    /// Whether the system's row in `spec.rs` lists it. Ported from cloudwithax/unslot@6339c18.
    pub fn runs(self, platform: Platform) -> bool {
        platform.spec().cores.contains(&self)
    }

    pub fn default_for(platform: Platform) -> Core {
        platform.spec().cores[0]
    }

    /// Whether two of it can play linked: mGBA's patched link mode and gpSP's link adapter.
    /// Gambatte has neither, so a game linked on it is opened on mGBA.
    pub fn links(self) -> bool {
        matches!(self, Core::Mgba | Core::Gpsp)
    }
}

pub fn read_selected_cores(root: &Path) -> HashMap<String, Core> {
    crate::ini::read(root, SELECTED_CORE_FILE)
        .into_iter()
        .filter_map(|(stem, name)| Core::parse(&name).map(|core| (stem, core)))
        .collect()
}

pub fn core_for(root: &Path, stem: &str) -> Core {
    let mut lines = crate::ini::read(root, SELECTED_CORE_FILE);
    if let Some(core) = lines.remove(stem).as_deref().and_then(Core::parse) {
        return core;
    }
    let suffix = format!("/{stem}");
    lines
        .into_iter()
        .find(|(k, _)| k.ends_with(&suffix))
        .and_then(|(_, v)| Core::parse(&v))
        .unwrap_or_default()
}

pub fn core_for_platform(root: &Path, stem: &str, platform: Platform) -> Core {
    let mut lines = crate::ini::read(root, SELECTED_CORE_FILE);
    lines
        .remove(&core_key(platform, stem))
        .or_else(|| lines.remove(stem))
        .as_deref()
        .and_then(Core::parse)
        .filter(|core| core.runs(platform))
        .unwrap_or_else(|| Core::default_for(platform))
}

pub fn write_selected_core(
    root: &Path,
    platform: Platform,
    stem: &str,
    core: Core,
) -> std::io::Result<()> {
    let path = root.join(SELECTED_CORE_FILE);
    let existing = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(e) => return Err(e),
    };
    let bare = crate::ini::set(&existing, stem, None)?;
    let out = crate::ini::set(&bare, &core_key(platform, stem), Some(core.as_str()))?;
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    crate::atomic::atomic_write(&path, out.as_bytes())
}

fn core_key(platform: Platform, stem: &str) -> String {
    format!("{}/{stem}", platform.dir_name())
}
