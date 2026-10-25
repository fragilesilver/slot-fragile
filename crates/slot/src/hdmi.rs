//! HDMI out: cable hotplug detection, display resolution switching via BaseOS dispdbg,
//! and ALSA audio routing to HDMI.
//!
//! On the device, control flows through `card/System/slothdmi.sh`.
//! At boot and while running, physical connection is checked via extcon.
//! When disconnected, output always stays on the handheld's internal LCD.

use std::path::{Path, PathBuf};

pub const HDMI_ALSA_DEVICE: &str = "plughw:CARD=ahubhdmi,DEV=0";
pub const EXTRAS_KEY: &str = "hdmi_mode";

#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
pub enum HdmiMode {
    #[default]
    Auto,
    Hdmi720p,
    Hdmi1080p,
    Off,
}

impl HdmiMode {
    pub const ALL: [HdmiMode; 4] = [
        HdmiMode::Auto,
        HdmiMode::Hdmi720p,
        HdmiMode::Hdmi1080p,
        HdmiMode::Off,
    ];

    pub fn key(self) -> &'static str {
        match self {
            HdmiMode::Auto => "auto",
            HdmiMode::Hdmi720p => "720p",
            HdmiMode::Hdmi1080p => "1080p",
            HdmiMode::Off => "off",
        }
    }

    pub fn text(self) -> &'static str {
        match self {
            HdmiMode::Auto => "Auto",
            HdmiMode::Hdmi720p => "720p",
            HdmiMode::Hdmi1080p => "1080p",
            HdmiMode::Off => "Off",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "auto" => Some(HdmiMode::Auto),
            "720p" | "720" => Some(HdmiMode::Hdmi720p),
            "1080p" | "1080" => Some(HdmiMode::Hdmi1080p),
            "off" | "0" | "false" => Some(HdmiMode::Off),
            _ => None,
        }
    }

    pub fn step(self, right: bool) -> Self {
        let n = Self::ALL.len();
        let idx = Self::ALL.iter().position(|&m| m == self).unwrap_or(0);
        let next = if right {
            (idx + 1) % n
        } else {
            (idx + n - 1) % n
        };
        Self::ALL[next]
    }

    pub fn is_enabled(self) -> bool {
        self != HdmiMode::Off
    }
}

pub trait HdmiController: Send {
    fn is_connected(&self) -> bool;
    fn switch_on(&mut self, mode: HdmiMode) -> Result<(), String>;
    fn switch_off(&mut self) -> Result<(), String>;
    fn rescue(&mut self) -> Result<(), String>;
}

/// Whether the device hardware possesses an HDMI port.
pub fn present() -> bool {
    #[cfg(all(feature = "device", unix))]
    {
        if let Ok(target) = std::env::var("BASEOS_TARGET") {
            if target == "rg28xx" {
                return false;
            }
        }
        Path::new("/sys/class/extcon/hdmi").exists()
            || Path::new("/sys/class/extcon").exists()
            || Path::new("/sys/kernel/debug/dispdbg").exists()
    }
    #[cfg(not(all(feature = "device", unix)))]
    {
        true
    }
}

/// Reads the configured HDMI mode from `Config/extras.state`, defaulting to `HdmiMode::Auto`.
pub fn read_hdmi_mode(root: Option<&Path>) -> HdmiMode {
    root.and_then(|r| {
        slot_store::read_extras(r)
            .get(EXTRAS_KEY)
            .and_then(HdmiMode::parse)
    })
    .unwrap_or_default()
}

pub fn for_card(root: &Path) -> Box<dyn HdmiController> {
    #[cfg(feature = "device")]
    {
        Box::new(DeviceHdmi {
            root: root.to_path_buf(),
        })
    }
    #[cfg(not(feature = "device"))]
    {
        let _ = root;
        Box::new(SimHdmi::default())
    }
}

pub struct DeviceHdmi {
    root: PathBuf,
}

impl DeviceHdmi {
    pub fn new(root: &Path) -> Self {
        DeviceHdmi {
            root: root.to_path_buf(),
        }
    }

    fn run_script(&self, args: &[&str]) -> bool {
        let mut cmd = std::process::Command::new("/bin/sh");
        cmd.arg(self.root.join("System/slothdmi.sh")).args(args);
        cmd.output().map(|s| s.status.success()).unwrap_or(false)
    }
}

impl HdmiController for DeviceHdmi {
    fn is_connected(&self) -> bool {
        self.run_script(&["status"])
    }

    fn switch_on(&mut self, mode: HdmiMode) -> Result<(), String> {
        let arg = match mode {
            HdmiMode::Hdmi1080p => "1080p",
            _ => "720p",
        };
        if self.run_script(&["on", arg]) {
            Ok(())
        } else {
            Err("Failed to switch display output to HDMI".into())
        }
    }

    fn switch_off(&mut self) -> Result<(), String> {
        if self.run_script(&["off"]) {
            Ok(())
        } else {
            Err("Failed to switch display output to LCD".into())
        }
    }

    fn rescue(&mut self) -> Result<(), String> {
        let extras = self.root.join("Config/extras.state");
        if self.run_script(&["rescue", extras.to_str().unwrap_or("")]) {
            Ok(())
        } else {
            Err("Failed to rescue display output".into())
        }
    }
}

#[derive(Default)]
pub struct SimHdmi {
    pub connected: bool,
    pub active_mode: Option<HdmiMode>,
}

impl HdmiController for SimHdmi {
    fn is_connected(&self) -> bool {
        self.connected
    }

    fn switch_on(&mut self, mode: HdmiMode) -> Result<(), String> {
        self.active_mode = Some(mode);
        Ok(())
    }

    fn switch_off(&mut self) -> Result<(), String> {
        self.active_mode = None;
        Ok(())
    }

    fn rescue(&mut self) -> Result<(), String> {
        self.active_mode = None;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hdmi_mode_parses_and_keys() {
        assert_eq!(HdmiMode::parse("auto"), Some(HdmiMode::Auto));
        assert_eq!(HdmiMode::parse("720p"), Some(HdmiMode::Hdmi720p));
        assert_eq!(HdmiMode::parse("1080p"), Some(HdmiMode::Hdmi1080p));
        assert_eq!(HdmiMode::parse("off"), Some(HdmiMode::Off));
        assert_eq!(HdmiMode::parse("invalid"), None);

        assert_eq!(HdmiMode::Auto.key(), "auto");
        assert_eq!(HdmiMode::Hdmi720p.key(), "720p");
        assert_eq!(HdmiMode::Hdmi1080p.key(), "1080p");
        assert_eq!(HdmiMode::Off.key(), "off");
    }

    #[test]
    fn hdmi_mode_steps_forward_and_backward() {
        let m = HdmiMode::Auto;
        let m = m.step(true);
        assert_eq!(m, HdmiMode::Hdmi720p);
        let m = m.step(true);
        assert_eq!(m, HdmiMode::Hdmi1080p);
        let m = m.step(true);
        assert_eq!(m, HdmiMode::Off);
        let m = m.step(true);
        assert_eq!(m, HdmiMode::Auto);
        let m = m.step(false);
        assert_eq!(m, HdmiMode::Off);
    }

    #[test]
    fn sim_hdmi_tracks_state() {
        let mut sim = SimHdmi::default();
        assert!(!sim.is_connected());
        assert_eq!(sim.active_mode, None);

        sim.connected = true;
        assert!(sim.is_connected());

        sim.switch_on(HdmiMode::Hdmi720p).unwrap();
        assert_eq!(sim.active_mode, Some(HdmiMode::Hdmi720p));

        sim.switch_off().unwrap();
        assert_eq!(sim.active_mode, None);
    }
}
