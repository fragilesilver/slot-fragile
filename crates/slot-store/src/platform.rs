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
}

impl Platform {
    pub const COUNT: usize = 6;
    pub const ALL: [Platform; Platform::COUNT] = [
        Platform::Gba,
        Platform::Gb,
        Platform::Gbc,
        Platform::Nes,
        Platform::Snes,
        Platform::PlayStation,
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

    /// The dir name lower-cased, as config files spell it.
    pub fn key(self) -> String {
        self.dir_name().to_ascii_lowercase()
    }

    pub fn from_key(key: &str) -> Option<Platform> {
        Platform::ALL
            .into_iter()
            .find(|p| key.eq_ignore_ascii_case(p.dir_name()))
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
