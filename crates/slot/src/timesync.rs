//! Network time, as BaseOS keeps it: `timedatectl set-ntp` turns BaseOS's ntpd supervisor on or
//! off (remembered in /data/ntp-enabled), and the supervisor writes
//! /run/baseos-ntp-synchronized once the clock is set (BaseOS docs/01-rootfs-and-init.md, 05).
//! It sets the system clock, which slot reads as UTC, so the time zone set on the clock screen
//! stays as it is. Off the device a pretend one stands in. Fragile's.

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Sync {
    Off,
    /// On, and no time from the network yet: no Wi-Fi, or still asking.
    Waiting,
    Synced,
}

impl Sync {
    pub fn text(self) -> &'static str {
        match self {
            Sync::Off => "Off",
            Sync::Waiting => "On, waiting",
            Sync::Synced => "On, synced",
        }
    }
}

pub trait TimeSync: Send {
    fn state(&self) -> Sync;
    fn set(&mut self, on: bool);
}

/// The one for this build: BaseOS's on the device, a pretend one anywhere else.
pub fn for_build() -> Box<dyn TimeSync> {
    #[cfg(all(feature = "device", unix))]
    {
        Box::new(BaseOs)
    }
    #[cfg(not(all(feature = "device", unix)))]
    {
        Box::new(Pretend::default())
    }
}

#[derive(Default)]
pub struct Pretend {
    on: bool,
}

impl TimeSync for Pretend {
    fn state(&self) -> Sync {
        match self.on {
            true => Sync::Waiting,
            false => Sync::Off,
        }
    }

    fn set(&mut self, on: bool) {
        self.on = on;
    }
}

#[cfg(all(feature = "device", unix))]
pub struct BaseOs;

#[cfg(all(feature = "device", unix))]
impl TimeSync for BaseOs {
    fn state(&self) -> Sync {
        let on = std::fs::read_to_string("/data/ntp-enabled").is_ok_and(|s| s.trim() == "yes");
        let synced = std::fs::metadata("/run/baseos-ntp-synchronized").is_ok_and(|m| m.len() > 0);
        match (on, synced) {
            (false, _) => Sync::Off,
            (true, false) => Sync::Waiting,
            (true, true) => Sync::Synced,
        }
    }

    fn set(&mut self, on: bool) {
        let said = std::process::Command::new("timedatectl")
            .args(["set-ntp", if on { "yes" } else { "no" }])
            .status();
        if !said.is_ok_and(|s| s.success()) {
            eprintln!("slot: timedatectl set-ntp failed");
        }
    }
}
