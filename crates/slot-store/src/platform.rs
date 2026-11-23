use std::path::Path;

use crate::spec::{Spec, SPECS};

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
pub enum Platform {
    #[default]
    Gba,
    Gb,
    Gbc,
    // --- extras ---
    Nes,
    Snes,
    PlayStation,
    Nds,
    /// The PokeDaisy shelf (`Games/PokeDaisy/`): GBA, GB and GBC games gathered on one
    /// shelf and played on PokeDaisy's companion cores; each keeps its own hardware identity
    /// for saves, carts and RetroAchievements (`hardware`).
    PdGba,
    PdGb,
    PdGbc,
}

impl Platform {
    pub const COUNT: usize = 10;
    pub const ALL: [Platform; Platform::COUNT] = [
        Platform::Gba,
        Platform::Gb,
        Platform::Gbc,
        Platform::Nes,
        Platform::Snes,
        Platform::PlayStation,
        Platform::Nds,
        Platform::PdGba,
        Platform::PdGb,
        Platform::PdGbc,
    ];

    // --- extras: the names below read the system's row in `spec.rs` ---

    pub fn spec(self) -> &'static Spec {
        &SPECS[self.index()]
    }

    /// Where the platform sits in `ALL`, for tables kept one entry a platform.
    pub const fn index(self) -> usize {
        self as usize
    }

    /// The name, or its short form where it has one.
    pub fn short_name(self) -> &'static str {
        match self.spec().short {
            "" => self.name(),
            short => short,
        }
    }

    /// The real hardware platform a cart is for (PokeDaisy carts map to GBA/GB/GBC).
    pub fn hardware(self) -> Platform {
        match self {
            Platform::PdGba => Platform::Gba,
            Platform::PdGb => Platform::Gb,
            Platform::PdGbc => Platform::Gbc,
            p => p,
        }
    }

    /// Whether the cart lives on the PokeDaisy shelf (`Games/PokeDaisy/`).
    pub fn is_pokedaisy(self) -> bool {
        matches!(self, Platform::PdGba | Platform::PdGb | Platform::PdGbc)
    }

    /// The canonical platform representing a shelf (`PdGba` gathers all PokeDaisy systems).
    pub fn shelf(self) -> Platform {
        if self.is_pokedaisy() {
            Platform::PdGba
        } else {
            self
        }
    }

    /// The folder under `Saves/` that holds battery saves (`Saves/GBA|GB|GBC` for PokeDaisy).
    pub fn save_dir_name(self) -> &'static str {
        if self.is_pokedaisy() {
            self.hardware().dir_name()
        } else {
            self.dir_name()
        }
    }

    /// The key used in config files (`recents.txt`, `favorites.txt`, `slot.state`).
    pub fn key(self) -> String {
        match self {
            Platform::PdGb => "pokedaisy-gb".into(),
            Platform::PdGbc => "pokedaisy-gbc".into(),
            p => p.dir_name().to_ascii_lowercase(),
        }
    }

    pub fn from_key(key: &str) -> Option<Platform> {
        Platform::ALL
            .into_iter()
            .find(|p| p.key().eq_ignore_ascii_case(key) || key.eq_ignore_ascii_case(p.dir_name()))
    }

    pub fn name(self) -> &'static str {
        self.spec().name
    }

    pub fn dir_name(self) -> &'static str {
        self.spec().dir
    }

    pub fn extensions(self) -> &'static [&'static str] {
        self.spec().extensions
    }

    pub fn picture(self) -> (u32, u32) {
        self.spec().picture
    }

    pub fn accepts(self, path: &Path) -> bool {
        path.extension()
            .and_then(|e| e.to_str())
            .is_some_and(|ext| {
                self.extensions()
                    .iter()
                    .any(|k| ext.eq_ignore_ascii_case(k))
            })
    }
}
