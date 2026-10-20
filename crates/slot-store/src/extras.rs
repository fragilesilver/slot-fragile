//! Fragile's own settings, kept out of `slot.state` so an upstream merge never fights over them
//! and a key one build does not know can never wipe the file. Every key is optional: a missing
//! or unreadable value falls back to its default, and keys this build does not know are kept.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::atomic::atomic_write;
use crate::config::CONFIG_DIR;
use crate::{Platform, Recent};

pub const EXTRAS_FILE: &str = "extras.state";

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct ExtrasState {
    entries: BTreeMap<String, String>,
}

impl ExtrasState {
    pub fn parse(text: &str) -> Self {
        ExtrasState {
            entries: crate::ini::parse(text).into_iter().collect(),
        }
    }

    pub fn text(&self) -> String {
        self.entries
            .iter()
            .map(|(k, v)| format!("{k}={v}\n"))
            .collect()
    }

    pub fn get(&self, key: &str) -> Option<&str> {
        self.entries.get(key).map(String::as_str)
    }

    /// `None` forgets the key, so the default applies again.
    pub fn set(&mut self, key: &str, value: Option<&str>) {
        let clean = |s: &str| !s.contains(['\n', '\r', '=']) && !s.trim().is_empty();
        match value {
            Some(v) if clean(key) && !v.contains(['\n', '\r']) => {
                self.entries
                    .insert(key.trim().to_string(), v.trim().to_string());
            }
            _ => {
                self.entries.remove(key);
            }
        }
    }

    pub fn flag(&self, key: &str, default: bool) -> bool {
        match self.get(key) {
            Some("1" | "on" | "true") => true,
            Some("0" | "off" | "false") => false,
            _ => default,
        }
    }

    pub fn set_flag(&mut self, key: &str, on: bool) {
        self.set(key, Some(if on { "1" } else { "0" }));
    }

    pub fn number<T: std::str::FromStr>(&self, key: &str) -> Option<T> {
        self.get(key)?.parse().ok()
    }

    /// A cart, written `gba/Emerald` the way `slot.state` writes one.
    pub fn cart(&self, key: &str) -> Option<Recent> {
        let (dir, stem) = self.get(key)?.split_once('/')?;
        let platform = Platform::ALL
            .into_iter()
            .find(|p| dir.eq_ignore_ascii_case(p.dir_name()))?;
        Some(Recent {
            platform,
            stem: stem.to_string(),
        })
        .filter(|r| !r.stem.is_empty())
    }

    pub fn set_cart(&mut self, key: &str, cart: Option<&Recent>) {
        let value =
            cart.map(|r| format!("{}/{}", r.platform.dir_name().to_ascii_lowercase(), r.stem));
        self.set(key, value.as_deref());
    }
}

pub fn extras_path(root: &Path) -> PathBuf {
    root.join(CONFIG_DIR).join(EXTRAS_FILE)
}

pub fn read_extras(root: &Path) -> ExtrasState {
    std::fs::read_to_string(extras_path(root))
        .map(|t| ExtrasState::parse(&t))
        .unwrap_or_default()
}

pub fn write_extras(root: &Path, s: &ExtrasState) -> std::io::Result<()> {
    let path = extras_path(root);
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    atomic_write(&path, s.text().as_bytes())
}
