//! Fragile's own settings, kept out of `slot.state` so an upstream merge never fights over them
//! and a key one build does not know can never wipe the file. Every key is optional: a missing
//! or unreadable value falls back to its default, and keys this build does not know are kept.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::atomic::atomic_write;
use crate::config::CONFIG_DIR;
use crate::{BatteryDisplay, Blending, Platform, Recent, Zone, DIM_MAX};

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

    pub fn ff_mode(&self) -> FfMode {
        match self.get("ff_mode") {
            Some("toggle" | "Toggle" | "1") => FfMode::Toggle,
            _ => FfMode::Hold,
        }
    }

    pub fn set_ff_mode(&mut self, mode: FfMode) {
        match mode {
            FfMode::Hold => self.set("ff_mode", None),
            FfMode::Toggle => self.set("ff_mode", Some("toggle")),
        }
    }

    pub fn dim(&self) -> u8 {
        self.number::<u8>("dim").unwrap_or(0).min(DIM_MAX)
    }

    pub fn set_dim(&mut self, dim: u8) {
        let clamped = dim.min(DIM_MAX);
        if clamped == 0 {
            self.set("dim", None);
        } else {
            self.set("dim", Some(&clamped.to_string()));
        }
    }

    pub fn zone(&self) -> Option<Zone> {
        self.get("zone").and_then(Zone::parse)
    }

    pub fn set_zone(&mut self, zone: Zone) {
        self.set("zone", Some(zone.id()));
    }

    pub fn blending(&self, platform: Platform) -> Blending {
        let key = format!("blending.{}", platform.key());
        if let Some(b) = self.get(&key).and_then(Blending::parse) {
            return b;
        }
        let legacy_key = format!("motion_blur_{}", platform.key());
        if self.flag(&legacy_key, false) {
            return Blending::Ghosting;
        }
        Blending::Off
    }

    pub fn set_blending(&mut self, platform: Platform, blend: Blending) {
        let key = format!("blending.{}", platform.key());
        if blend == Blending::Off {
            self.set(&key, None);
        } else {
            self.set(&key, Some(blend.key()));
        }
    }

    pub fn battery_display(&self) -> BatteryDisplay {
        if let Some(d) = self.get("battery_display").and_then(BatteryDisplay::parse) {
            return d;
        }
        if self.flag("battery_badge", false) {
            BatteryDisplay::Both
        } else {
            BatteryDisplay::Off
        }
    }

    pub fn set_battery_display(&mut self, display: BatteryDisplay) {
        self.set("battery_display", Some(display.key()));
        self.set_flag("battery_badge", display != BatteryDisplay::Off);
    }

    pub fn cal_warm(&self) -> i8 {
        self.number::<i8>("cal_warm").unwrap_or(0).clamp(-100, 100)
    }

    pub fn set_cal_warm(&mut self, v: i8) {
        let v = v.clamp(-100, 100);
        if v == 0 {
            self.set("cal_warm", None);
        } else {
            self.set("cal_warm", Some(&v.to_string()));
        }
    }

    pub fn cal_tint(&self) -> i8 {
        self.number::<i8>("cal_tint").unwrap_or(0).clamp(-100, 100)
    }

    pub fn set_cal_tint(&mut self, v: i8) {
        let v = v.clamp(-100, 100);
        if v == 0 {
            self.set("cal_tint", None);
        } else {
            self.set("cal_tint", Some(&v.to_string()));
        }
    }

    pub fn cal_curve(&self) -> i8 {
        self.number::<i8>("cal_curve").unwrap_or(0).clamp(-50, 50)
    }

    pub fn set_cal_curve(&mut self, v: i8) {
        let v = v.clamp(-50, 50);
        if v == 0 {
            self.set("cal_curve", None);
        } else {
            self.set("cal_curve", Some(&v.to_string()));
        }
    }

    pub fn nickname(&self) -> &str {
        self.get("nickname")
            .filter(|s| !s.is_empty())
            .unwrap_or("Player")
    }

    pub fn set_nickname(&mut self, name: &str) {
        let clean = name.trim();
        if clean.is_empty() || clean == "Player" {
            self.set("nickname", None);
        } else {
            self.set("nickname", Some(clean));
        }
    }

    pub fn boot_logo(&self) -> &str {
        self.get("boot_logo")
            .filter(|s| !s.is_empty())
            .unwrap_or("slot")
    }

    pub fn set_boot_logo(&mut self, pick: &str) {
        if pick.is_empty() || pick == "slot" {
            self.set("boot_logo", None);
        } else {
            self.set("boot_logo", Some(pick));
        }
    }

    pub fn haptics(&self) -> bool {
        self.flag("haptics", true)
    }

    pub fn set_haptics(&mut self, on: bool) {
        self.set_flag("haptics", on);
    }

    pub fn collections_top_level(&self) -> bool {
        self.flag("collections_top_level", true)
    }

    pub fn set_collections_top_level(&mut self, on: bool) {
        self.set_flag("collections_top_level", on);
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Default, Debug)]
pub enum FfMode {
    #[default]
    Hold,
    Toggle,
}

impl FfMode {
    pub const ALL: [FfMode; 2] = [FfMode::Hold, FfMode::Toggle];

    pub fn label(self) -> &'static str {
        match self {
            FfMode::Hold => "Hold",
            FfMode::Toggle => "Toggle",
        }
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
