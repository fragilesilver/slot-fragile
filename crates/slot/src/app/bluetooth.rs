//! Main menu → Network → Devices: Bluetooth controllers and headphones.
//!
//! Pairing, connecting, disconnecting and forgetting devices. Sound routes dynamically
//! to connected headphones via BlueALSA.

use slot_input::{Action, Btn};
use slot_ui::{Draw, ListPage, ListRow, TexId};

use super::overlay::Overlay;
use super::App;
use crate::bluetooth::{BtBusy, Device, Kind};

pub const BLUETOOTH: &str = "bluetooth";

impl App {
    pub fn bluetooth_on(&self) -> bool {
        self.extras.flag(BLUETOOTH, false)
    }

    pub fn bt_view(&self) -> crate::bluetooth::BtView {
        self.overlay.network.bt_view.clone()
    }

    pub fn bluetooth(&self) -> Option<&crate::bluetooth::Bluetooth> {
        self.overlay.network.bluetooth.as_ref()
    }

    pub(super) fn open_bluetooth(&mut self) {
        if let Some(bt) = &self.overlay.network.bluetooth {
            if !self
                .overlay
                .network
                .bt_view
                .devices
                .iter()
                .any(|d| d.paired)
            {
                bt.search();
            }
        }
        let page = ListPage::new(self.bt_rows());
        self.overlay.open = Some(Overlay::Bluetooth(page));
        self.overlay.faces.clear();
    }

    pub(super) fn bluetooth_tick(&mut self) {
        let Some(bt) = &self.overlay.network.bluetooth else {
            return;
        };
        let view = bt.view();
        if view.audio().map(|d| &d.mac) != self.overlay.network.bt_view.audio().map(|d| &d.mac) {
            crate::audio::set_route(view.audio().map(|d| crate::bluetooth::alsa_device(&d.mac)));
        }
        self.overlay.network.bt_view = view;
        let rows = self.bt_rows();
        if let Some(Overlay::Bluetooth(page)) = &mut self.overlay.open {
            if page.rows() != rows {
                page.set_rows(rows);
            }
        }
    }

    pub(super) fn bt_rows(&self) -> Vec<ListRow> {
        let v = &self.overlay.network.bt_view;
        if v.devices.is_empty() {
            let says = match v.busy {
                BtBusy::Starting => "Turning on...",
                BtBusy::Searching => "Searching...",
                _ if v.yielded => "Link play in progress",
                _ => "No devices found",
            };
            let help = match (&v.error, v.yielded) {
                (Some(e), _) => Some(e.clone()),
                (_, true) => Some("Bluetooth paused during link play.".into()),
                _ => Some("Put a device in pairing mode, then press Y to search.".into()),
            };
            return vec![ListRow::new(says).with_help(help.unwrap_or_default())];
        }
        v.devices
            .iter()
            .map(|d| {
                let value = match (d.connected, d.paired) {
                    (true, _) => "Connected",
                    (false, true) => "Paired",
                    (false, false) => match d.kind {
                        Kind::Controller => "Controller",
                        Kind::Audio => "Headphones",
                        Kind::Other => "",
                    },
                };
                let help = match (&v.error, &v.busy, d.connected, d.paired) {
                    (Some(e), ..) => e.clone(),
                    (_, BtBusy::Pairing(n), ..) if n == &d.name => format!("Pairing {n}..."),
                    (_, BtBusy::Connecting(n), ..) if n == &d.name => format!("Connecting {n}..."),
                    (_, BtBusy::Searching, ..) => "Searching for devices...".into(),
                    (.., true, _) => "A to disconnect, X to forget.".into(),
                    (.., false, true) => "A to connect, X to forget.".into(),
                    _ => "A to pair and connect.".into(),
                };
                ListRow::new(d.name.as_str())
                    .with_value(value)
                    .with_help(help)
            })
            .collect()
    }

    pub(super) fn lit_bt_device(&self) -> Option<Device> {
        let Some(Overlay::Bluetooth(page)) = &self.overlay.open else {
            return None;
        };
        let devices = &self.overlay.network.bt_view.devices;
        if devices.is_empty() {
            return None;
        }
        let cursor = page.cursor().min(devices.len().saturating_sub(1));
        devices.get(cursor).cloned()
    }

    pub(super) fn bluetooth_input(&mut self, action: Action) -> bool {
        if !matches!(self.overlay.open, Some(Overlay::Bluetooth(_))) {
            return false;
        }
        match action {
            Action::GbaDown(Btn::Y) => {
                if let Some(bt) = &self.overlay.network.bluetooth {
                    bt.search();
                }
                true
            }
            Action::GbaDown(Btn::X) => {
                if let (Some(d), Some(bt)) = (self.lit_bt_device(), &self.overlay.network.bluetooth)
                {
                    if d.paired {
                        bt.forget(&d.mac);
                    }
                }
                true
            }
            Action::GbaDown(Btn::A) => {
                if let Some(bt) = &self.overlay.network.bluetooth {
                    match self.lit_bt_device() {
                        Some(d) if d.connected => bt.disconnect(&d.mac),
                        Some(d) if d.paired => bt.connect(&d.mac),
                        Some(d) => bt.pair(&d.mac),
                        None => bt.search(),
                    }
                }
                true
            }
            _ => false,
        }
    }

    pub(super) fn draw_bluetooth_legend(
        &self,
        back: (TexId, u32),
        y: f32,
        out: &mut Vec<Draw>,
    ) -> bool {
        if !matches!(self.overlay.open, Some(Overlay::Bluetooth(_))) {
            return false;
        }
        let made = &self.overlay.shelves.bt_legends;
        let light = self.light() as usize;
        let base = light * 5;
        let (Some(&search), Some(&forget), Some(&connect), Some(&disconnect), Some(&pair)) = (
            made.get(base),
            made.get(base + 1),
            made.get(base + 2),
            made.get(base + 3),
            made.get(base + 4),
        ) else {
            self.draw_side_legend(back, false, y, out);
            return true;
        };
        let mut legends = vec![back, search];
        let picked = self.lit_bt_device();
        if picked.as_ref().is_some_and(|d| d.paired) {
            legends.push(forget);
        }
        match picked {
            Some(d) if d.connected => legends.push(disconnect),
            Some(d) if d.paired => legends.push(connect),
            Some(_) => legends.push(pair),
            None => {}
        }
        self.draw_spread_legends(&legends, y, out);
        true
    }
}
