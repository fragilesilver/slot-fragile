//! Main menu → Network: Wi-Fi on or off, the networks in range to join (a password on the
//! keyboard), what it is connected to and its address, and the clock set from the internet.
//! The radio is `crate::wifi` (unslot's) and the clock `crate::timesync` (BaseOS's); the page
//! is Fragile's.

use slot_input::{Action, Btn};
use slot_ui::{Draw, Keyboard, ListPage, ListRow, QuickRow, QuickValue, TexId, HINT_EDGE, MOUTH_H};

use super::overlay::Overlay;
use super::shelves::Typing;
use super::{App, LINK_LEGEND_EDGE};
use crate::timesync::{Sync, TimeSync};
use crate::wifi::{psk_value, Busy, Network, Security, Wifi, WifiView};

/// extras.state: whether the Wi-Fi is on.
const WIFI: &str = "wifi";
const PASSWORD_MAX: usize = 63;
const NO_NETWORKS: &str = "No networks found";
const SEARCHING: &str = "Searching";

#[derive(Default)]
pub struct NetworkState {
    wifi: Option<Wifi>,
    time: Option<Box<dyn TimeSync>>,
    /// The password being typed and the network it is for, with what went wrong last.
    typing: Option<(Keyboard, String, Option<String>)>,
}

fn row_value(n: &Network, view: &WifiView) -> &'static str {
    if view.connected.as_deref() == Some(n.ssid.as_str()) {
        return "Connected";
    }
    if view.busy == Busy::Joining(n.ssid.clone()) {
        return "Joining";
    }
    if n.saved {
        return "Saved";
    }
    match (n.security, n.bars()) {
        (Security::Unsupported, _) => "Can't join",
        (_, 3) => "Strong",
        (_, 2) => "Good",
        _ => "Weak",
    }
}

fn network_rows(view: &WifiView) -> Vec<ListRow> {
    if view.networks.is_empty() {
        let says = match view.busy {
            Busy::Starting | Busy::Scanning => SEARCHING,
            _ => NO_NETWORKS,
        };
        return vec![ListRow::new(says)];
    }
    view.networks
        .iter()
        .map(|n| ListRow::new(n.ssid.as_str()).with_value(row_value(n, view)))
        .collect()
}

impl App {
    /// At boot: the radio and the clock, and the Wi-Fi back on if it was left on.
    pub(super) fn start_network(&mut self, root: &std::path::Path) {
        let wifi = Wifi::for_card(root);
        if self.wifi_on() {
            wifi.turn_on();
        }
        self.overlay.network.wifi = Some(wifi);
        self.overlay.network.time = Some(crate::timesync::for_build());
    }

    fn wifi_on(&self) -> bool {
        self.extras.flag(WIFI, false)
    }

    pub fn wifi_view(&self) -> WifiView {
        self.overlay
            .network
            .wifi
            .as_ref()
            .map(Wifi::view)
            .unwrap_or_default()
    }

    fn time_sync(&self) -> Sync {
        self.overlay
            .network
            .time
            .as_ref()
            .map_or(Sync::Off, |t| t.state())
    }

    /// The rows that only mean something with the Wi-Fi on.
    pub(super) fn network_hidden(&self) -> Vec<QuickRow> {
        match self.wifi_on() {
            true => Vec::new(),
            false => vec![
                QuickRow::WifiNetworks,
                QuickRow::WifiStatus,
                QuickRow::WifiIp,
            ],
        }
    }

    pub(super) fn network_value(&self, row: QuickRow) -> Option<Option<QuickValue>> {
        match row {
            QuickRow::WifiPower => Some(Some(QuickValue::flag(self.wifi_on()))),
            QuickRow::WifiStatus | QuickRow::WifiIp | QuickRow::TimeSync => Some(None),
            _ => None,
        }
    }

    /// The Network page's values that are text: what the radio is doing, its address, the
    /// clock.
    pub(super) fn network_text(&self, row: QuickRow) -> Option<String> {
        match row {
            QuickRow::WifiStatus => {
                let v = self.wifi_view();
                Some(match (&v.error, &v.busy, &v.connected) {
                    _ if !v.on => "Off".into(),
                    _ if v.yielded => "Link play".into(),
                    (_, Busy::Starting, _) => "Starting".into(),
                    (_, Busy::Joining(_), _) => "Joining".into(),
                    (Some(e), ..) => e.clone(),
                    (_, _, Some(ssid)) => ssid.clone(),
                    (_, Busy::Scanning, None) => SEARCHING.into(),
                    _ => "Not connected".into(),
                })
            }
            QuickRow::WifiIp => Some(self.wifi_view().ip.unwrap_or_else(|| "None".into())),
            QuickRow::TimeSync => Some(self.time_sync().text().into()),
            _ => None,
        }
    }

    /// Left or Right on a Network row. Says whether it was one.
    pub(super) fn change_network(&mut self, row: QuickRow) -> bool {
        match row {
            QuickRow::WifiPower => {
                let on = !self.wifi_on();
                self.extras.set_flag(WIFI, on);
                self.save_extras();
                if let Some(wifi) = &self.overlay.network.wifi {
                    match on {
                        true => wifi.turn_on(),
                        false => wifi.turn_off(),
                    }
                }
            }
            QuickRow::TimeSync => {
                let on = self.time_sync() == Sync::Off;
                if let Some(time) = self.overlay.network.time.as_mut() {
                    time.set(on);
                }
            }
            _ => return false,
        }
        true
    }

    pub(super) fn open_networks(&mut self) {
        if let Some(wifi) = &self.overlay.network.wifi {
            wifi.scan();
        }
        let page = ListPage::new(network_rows(&self.wifi_view()));
        self.overlay.open = Some(Overlay::Networks(page));
        self.overlay.faces.clear();
    }

    /// Each frame: the Networks page follows what the radio has found.
    pub(super) fn network_tick(&mut self) {
        if !matches!(self.overlay.open, Some(Overlay::Networks(_))) {
            return;
        }
        let rows = network_rows(&self.wifi_view());
        if let Some(Overlay::Networks(page)) = &mut self.overlay.open {
            if page.rows() != rows {
                page.set_rows(rows);
            }
        }
    }

    fn lit_network(&self) -> Option<Network> {
        let Some(Overlay::Networks(page)) = &self.overlay.open else {
            return None;
        };
        let ssid = &page.selected()?.label;
        self.wifi_view()
            .networks
            .into_iter()
            .find(|n| &n.ssid == ssid)
    }

    /// A joins the lit network, asking for its password first when it needs one; X forgets a
    /// saved one; the keyboard takes everything while a password is typed. Says whether it
    /// took the action.
    pub(super) fn networks_input(&mut self, action: Action) -> bool {
        if !matches!(self.overlay.open, Some(Overlay::Networks(_))) {
            return false;
        }
        if self.overlay.network.typing.is_some() {
            self.password_input(action);
            return true;
        }
        match action {
            Action::GbaDown(Btn::A) => {
                let Some(n) = self.lit_network() else {
                    if let Some(wifi) = &self.overlay.network.wifi {
                        wifi.scan();
                    }
                    return true;
                };
                match (n.security, n.saved) {
                    (Security::Unsupported, _) => {}
                    (Security::Open, _) | (_, true) => self.join(&n.ssid, None),
                    (Security::Personal, false) => {
                        let keyboard = Keyboard::password(PASSWORD_MAX);
                        self.overlay.network.typing = Some((keyboard, n.ssid, None));
                    }
                }
                true
            }
            Action::GbaDown(Btn::X) => {
                if let (Some(n), Some(wifi)) = (self.lit_network(), &self.overlay.network.wifi) {
                    if n.saved {
                        wifi.forget(&n.ssid);
                    }
                }
                true
            }
            _ => false,
        }
    }

    fn join(&self, ssid: &str, psk: Option<&str>) {
        if let Some(wifi) = &self.overlay.network.wifi {
            wifi.join(ssid, psk);
        }
    }

    fn password_input(&mut self, action: Action) {
        let Some((keyboard, ..)) = self.overlay.network.typing.as_mut() else {
            return;
        };
        match action {
            Action::GbaDown(Btn::Up) => keyboard.up(),
            Action::GbaDown(Btn::Down) => keyboard.down(),
            Action::GbaDown(Btn::Left) => keyboard.left(),
            Action::GbaDown(Btn::Right) => keyboard.right(),
            Action::GbaDown(Btn::A) => {
                if keyboard.press() {
                    self.join_typed();
                }
            }
            Action::GbaDown(Btn::Y | Btn::R1) => keyboard.type_char(' '),
            Action::GbaDown(Btn::X) => keyboard.shift(),
            Action::GbaDown(Btn::L1) => keyboard.delete(),
            Action::GbaDown(Btn::B) if !keyboard.text().is_empty() => keyboard.delete(),
            Action::GbaDown(Btn::B) | Action::QuickMenu => self.overlay.network.typing = None,
            Action::GbaDown(Btn::Start) => self.join_typed(),
            _ => {}
        }
    }

    /// OK on a password: a good one joins and the page comes back; a bad one says why.
    fn join_typed(&mut self) {
        let Some((keyboard, ssid, said)) = self.overlay.network.typing.as_mut() else {
            return;
        };
        let typed = keyboard.text().to_string();
        if let Err(why) = psk_value(&typed) {
            *said = Some(why);
            return;
        }
        let ssid = ssid.clone();
        self.overlay.network.typing = None;
        self.join(&ssid, Some(&typed));
    }

    /// The password's keyboard page, for the shared keyboard faces.
    pub(super) fn network_typing(&self) -> Option<Typing> {
        let (keyboard, ssid, said) = self.overlay.network.typing.as_ref()?;
        Some(Typing {
            shown: keyboard.shown(),
            line: said
                .clone()
                .unwrap_or_else(|| format!("Password for {ssid}")),
            matches: Vec::new(),
            legend: 2,
        })
    }

    /// The password keyboard over everything while it is up. Says whether it drew.
    pub(super) fn draw_network_keyboard(&self, out: &mut Vec<Draw>) -> bool {
        let Some((keyboard, ..)) = &self.overlay.network.typing else {
            return false;
        };
        keyboard.draw(self.canvas(), self.keyboard_faces(), 0.0, out);
        true
    }

    /// The Networks page's footer: Back, then Forget for a saved network, then Join. Says
    /// whether it drew.
    pub(super) fn draw_networks_legend(
        &self,
        back: (TexId, u32),
        y: f32,
        out: &mut Vec<Draw>,
    ) -> bool {
        if !matches!(self.overlay.open, Some(Overlay::Networks(_))) {
            return false;
        }
        let made = &self.overlay.shelves.network_legends;
        let light = self.light() as usize;
        let (Some(&join), Some(&forget)) = (made.get(light * 2), made.get(light * 2 + 1)) else {
            self.draw_side_legend(back, false, y, out);
            return true;
        };
        let mut legends = vec![back];
        if self.lit_network().is_some_and(|n| n.saved) {
            legends.push(forget);
        }
        legends.push(join);
        let seen = |w: u32| w.saturating_sub(HINT_EDGE) as f32;
        let room = self.canvas().w - 2.0 * LINK_LEGEND_EDGE;
        let gap =
            (room - legends.iter().map(|l| seen(l.1)).sum::<f32>()) / (legends.len() - 1) as f32;
        let mut x = LINK_LEGEND_EDGE;
        for (tex, w) in legends {
            out.push(Draw::Tex {
                x: x.round(),
                y,
                w: w as f32,
                h: MOUTH_H,
                tex,
                alpha: 1.0,
            });
            x += seen(w) + gap;
        }
        true
    }

    pub fn network_keyboard_key(&self) -> Option<slot_ui::Key> {
        self.overlay.network.typing.as_ref().map(|(k, ..)| k.key())
    }

    /// For tests: the password typed so far, while the keyboard is up.
    pub fn network_password(&self) -> Option<&str> {
        self.overlay.network.typing.as_ref().map(|(k, ..)| k.text())
    }
}
