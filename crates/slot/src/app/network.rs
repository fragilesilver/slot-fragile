//! Main menu → Network: Wi-Fi on or off, the networks in range to join (a password on the
//! keyboard), what it is connected to and its address, the clock set from the internet, and
//! RetroAchievements on or off and the account it signs in to. In game, the Achievements page
//! lists the playing game's. The radio is `crate::wifi` (unslot's), the clock `crate::timesync`
//! (BaseOS's) and achievements `crate::cheevos` (nickclyde's); the pages are Fragile's.

use slot_input::{Action, Btn};
use slot_ui::{Draw, Keyboard, ListPage, ListRow, QuickRow, QuickValue, TexId, HINT_EDGE, MOUTH_H};

use super::overlay::Overlay;
use super::shelves::Typing;
use super::{App, LINK_LEGEND_EDGE};
use crate::timesync::{Sync, TimeSync};
use crate::wifi::{psk_value, Busy, Network, Security, Wifi, WifiView};

/// extras.state: whether the Wi-Fi is on, and whether achievements are followed.
const WIFI: &str = "wifi";
const CHEEVOS: &str = "cheevos";
/// Where the RetroAchievements account is kept, as `slot_cheevos::CHEEVOS_FILE` names it.
const CHEEVOS_FILE: &str = "Config/retroachievements.txt";
const PASSWORD_MAX: usize = 63;
const NAME_MAX: usize = 64;
const NO_NETWORKS: &str = "No networks found";
const SEARCHING: &str = "Searching";
/// How long the in-game list of achievements is kept before it is asked for again.
const ACHIEVEMENTS_FRESH_MS: u64 = 2000;

#[derive(Default)]
pub struct NetworkState {
    wifi: Option<Wifi>,
    pub(super) bluetooth: Option<crate::bluetooth::Bluetooth>,
    pub(super) bt_view: crate::bluetooth::BtView,
    time: Option<Box<dyn TimeSync>>,
    /// What is being typed and what for, with what went wrong last.
    typing: Option<(Keyboard, Typed, Option<String>)>,
    /// The playing game's achievements as last asked for, and when.
    achievements: std::cell::RefCell<(Option<u64>, Vec<Achievement>)>,
}

#[derive(Clone, PartialEq, Eq, Debug)]
enum Typed {
    WifiPassword(String),
    CheevosUser,
    CheevosPassword(String),
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Achievement {
    pub title: String,
    pub points: u32,
    pub unlocked: bool,
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
    /// At boot: the radio and the clock, the Wi-Fi back on if it was left on, and whether
    /// achievements are followed.
    pub(super) fn start_network(&mut self, root: &std::path::Path) {
        let wifi = Wifi::for_card(root);
        if self.wifi_on() {
            wifi.turn_on();
        }
        self.overlay.network.wifi = Some(wifi);
        if crate::bluetooth::present() {
            let bt = crate::bluetooth::Bluetooth::for_card(root);
            if self.bluetooth_on() {
                bt.turn_on();
            }
            self.overlay.network.bluetooth = Some(bt);
        }
        self.overlay.network.time = Some(crate::timesync::for_build());
        crate::cheevos::set_enabled(self.cheevos_on());
    }

    fn wifi_on(&self) -> bool {
        self.extras.flag(WIFI, false)
    }

    fn cheevos_on(&self) -> bool {
        self.extras.flag(CHEEVOS, true)
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

    /// The rows that only mean something with the Wi-Fi or Bluetooth on.
    pub(super) fn network_hidden(&self) -> Vec<QuickRow> {
        let mut hidden = Vec::new();
        if !self.wifi_on() {
            hidden.extend([
                QuickRow::WifiNetworks,
                QuickRow::WifiStatus,
                QuickRow::WifiIp,
            ]);
        }
        if self.overlay.network.bluetooth.is_none() {
            hidden.extend([QuickRow::Bluetooth, QuickRow::BtDevices]);
        } else if !self.bluetooth_on() {
            hidden.push(QuickRow::BtDevices);
        }
        hidden
    }

    pub(super) fn network_value(&self, row: QuickRow) -> Option<Option<QuickValue>> {
        match row {
            QuickRow::WifiPower => Some(Some(QuickValue::flag(self.wifi_on()))),
            QuickRow::Bluetooth => Some(Some(QuickValue::flag(self.bluetooth_on()))),
            QuickRow::Cheevos => Some(Some(QuickValue::flag(self.cheevos_on()))),
            QuickRow::WifiStatus
            | QuickRow::WifiIp
            | QuickRow::TimeSync
            | QuickRow::CheevosAccount
            | QuickRow::GameArt
            | QuickRow::BtDevices => Some(None),
            _ => None,
        }
    }

    /// The Network page's values that are text: what the radio is doing, its address, the
    /// clock, and the RetroAchievements account.
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
            QuickRow::GameArt => Some(self.game_art_text()),
            QuickRow::CheevosAccount => Some(match crate::cheevos::status() {
                crate::cheevos::Status::SignedOut => "Sign in".into(),
                crate::cheevos::Status::SigningIn if !crate::wifi::online() => {
                    "Waiting for Wi-Fi".into()
                }
                crate::cheevos::Status::SigningIn => "Signing in".into(),
                crate::cheevos::Status::SignedIn(user) => user,
                crate::cheevos::Status::Rejected(_) => "Sign-in failed".into(),
            }),
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
            QuickRow::Cheevos => {
                let on = !self.cheevos_on();
                self.extras.set_flag(CHEEVOS, on);
                self.save_extras();
                crate::cheevos::set_enabled(on);
            }
            QuickRow::Bluetooth => {
                let on = !self.bluetooth_on();
                self.extras.set_flag(crate::app::bluetooth::BLUETOOTH, on);
                self.save_extras();
                if let Some(bt) = &self.overlay.network.bluetooth {
                    match on {
                        true => bt.turn_on(),
                        false => bt.turn_off(),
                    }
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
    /// saved one. Says whether it took the action.
    pub(super) fn networks_input(&mut self, action: Action) -> bool {
        if !matches!(self.overlay.open, Some(Overlay::Networks(_))) {
            return false;
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
                        let typed = Typed::WifiPassword(n.ssid);
                        self.overlay.network.typing = Some((keyboard, typed, None));
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

    /// A on Account: the username first, the one kept already filled in, then the password.
    pub(super) fn start_cheevos_sign_in(&mut self) {
        let known = self
            .root
            .as_deref()
            .and_then(|root| slot_store::ini::value(root, CHEEVOS_FILE, "username"))
            .unwrap_or_default();
        let keyboard = Keyboard::default().with_max(NAME_MAX).with_text(&known);
        self.overlay.network.typing = Some((keyboard, Typed::CheevosUser, None));
    }

    /// The keyboard takes everything while it is up, whatever page it was opened from. Says
    /// whether it was up.
    pub(super) fn typing_input(&mut self, action: Action) -> bool {
        let Some((keyboard, ..)) = self.overlay.network.typing.as_mut() else {
            return false;
        };
        match action {
            Action::GbaDown(Btn::Up) => keyboard.up(),
            Action::GbaDown(Btn::Down) => keyboard.down(),
            Action::GbaDown(Btn::Left) => keyboard.left(),
            Action::GbaDown(Btn::Right) => keyboard.right(),
            Action::GbaDown(Btn::A) => {
                if keyboard.press() {
                    self.typed_ok();
                }
            }
            Action::GbaDown(Btn::Y | Btn::R1) => keyboard.type_char(' '),
            Action::GbaDown(Btn::X) => keyboard.shift(),
            Action::GbaDown(Btn::L1) => keyboard.delete(),
            Action::GbaDown(Btn::B) if !keyboard.text().is_empty() => keyboard.delete(),
            Action::GbaDown(Btn::B) | Action::QuickMenu => self.overlay.network.typing = None,
            Action::GbaDown(Btn::Start) => self.typed_ok(),
            _ => {}
        }
        true
    }

    /// OK on the keyboard: a good Wi-Fi password joins, a username asks for its password, and
    /// a RetroAchievements password signs in. Anything wrong says why on the keyboard.
    fn typed_ok(&mut self) {
        let Some((keyboard, typed, said)) = self.overlay.network.typing.as_mut() else {
            return;
        };
        let text = keyboard.text().to_string();
        match typed.clone() {
            Typed::WifiPassword(ssid) => {
                if let Err(why) = psk_value(&text) {
                    *said = Some(why);
                    return;
                }
                self.overlay.network.typing = None;
                self.join(&ssid, Some(&text));
            }
            Typed::CheevosUser => {
                let user = text.trim().to_string();
                if user.is_empty() {
                    *said = Some("Type your username".into());
                    return;
                }
                let keyboard = Keyboard::password(NAME_MAX);
                let typed = Typed::CheevosPassword(user);
                self.overlay.network.typing = Some((keyboard, typed, None));
            }
            Typed::CheevosPassword(user) => {
                if text.is_empty() {
                    *said = Some("Type your password".into());
                    return;
                }
                self.overlay.network.typing = None;
                self.sign_in(&user, &text);
            }
        }
    }

    /// Writes the account to Config/retroachievements.txt, keeping a `host` line, and signs
    /// in. The password stays in the file only until the server answers it with a token.
    fn sign_in(&mut self, user: &str, password: &str) {
        let Some(root) = self.root.clone() else {
            return;
        };
        let path = root.join(CHEEVOS_FILE);
        let old = std::fs::read_to_string(&path).unwrap_or_default();
        let written = slot_store::ini::set(&old, "username", Some(user))
            .and_then(|t| slot_store::ini::set(&t, "password", Some(password)))
            .and_then(|t| slot_store::ini::set(&t, "token", None))
            .and_then(|t| slot_store::atomic_write(&path, t.as_bytes()));
        self.card_writes.wrote("retroachievements", &written);
        match written {
            Ok(()) => crate::cheevos::sign_in_again(&root),
            Err(e) => eprintln!("slot: cheevos: {e}"),
        }
    }

    /// The playing game's achievements, asked for again at most every two seconds.
    pub fn achievements(&self) -> Vec<Achievement> {
        let now = self.now();
        let mut cache = self.overlay.network.achievements.borrow_mut();
        let fresh = cache
            .0
            .is_some_and(|at| now.saturating_sub(at) < ACHIEVEMENTS_FRESH_MS);
        if !fresh {
            let list = crate::cheevos::achievements()
                .into_iter()
                .map(|a| Achievement {
                    title: a.title,
                    points: a.points,
                    unlocked: a.unlocked,
                })
                .collect();
            *cache = (Some(now), list);
        }
        cache.1.clone()
    }

    /// The Achievements row shows once the playing game's achievements are known.
    pub(super) fn achievements_hidden(&self) -> bool {
        self.achievements().is_empty()
    }

    pub(super) fn open_achievements(&mut self) {
        let rows = self
            .achievements()
            .into_iter()
            .map(|a| {
                let value = match a.unlocked {
                    true => "Unlocked".to_string(),
                    false => format!("{} pts", a.points),
                };
                ListRow::new(a.title).with_value(value)
            })
            .collect();
        self.overlay.open = Some(Overlay::Achievements(ListPage::new(rows)));
        self.overlay.faces.clear();
    }

    /// The keyboard's page, for the shared keyboard faces.
    pub(super) fn network_typing(&self) -> Option<Typing> {
        let (keyboard, typed, said) = self.overlay.network.typing.as_ref()?;
        let (line, legend) = match typed {
            Typed::WifiPassword(ssid) => (format!("Password for {ssid}"), 2),
            Typed::CheevosUser => ("RetroAchievements username".to_string(), 3),
            Typed::CheevosPassword(user) => (format!("Password for {user}"), 4),
        };
        Some(Typing {
            shown: keyboard.shown(),
            line: said.clone().unwrap_or(line),
            matches: Vec::new(),
            legend,
        })
    }

    /// The keyboard over everything while it is up. Says whether it drew.
    pub(super) fn draw_network_keyboard(&self, out: &mut Vec<Draw>) -> bool {
        let Some((keyboard, ..)) = &self.overlay.network.typing else {
            return false;
        };
        keyboard.draw(self.canvas(), self.keyboard_faces(), 0.0, out);
        true
    }

    /// Whether the open page's footer spreads a legend across the middle, where the slot is,
    /// so the slot is drawn plugged under it: the Networks page on a saved network.
    pub(super) fn footer_spread(&self) -> bool {
        (matches!(self.overlay.open, Some(Overlay::Networks(_)))
            && self.lit_network().is_some_and(|n| n.saved))
            || matches!(self.overlay.open, Some(Overlay::Bluetooth(_)))
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
        self.draw_spread_legends(&legends, y, out);
        true
    }

    /// Footer legends spread evenly from the left edge to the right, as a binding row's are.
    pub(super) fn draw_spread_legends(
        &self,
        legends: &[(TexId, u32)],
        y: f32,
        out: &mut Vec<Draw>,
    ) {
        let seen = |w: u32| w.saturating_sub(HINT_EDGE) as f32;
        let room = self.canvas().w - 2.0 * LINK_LEGEND_EDGE;
        let gaps = legends.len().saturating_sub(1).max(1) as f32;
        let gap = (room - legends.iter().map(|l| seen(l.1)).sum::<f32>()) / gaps;
        let mut x = LINK_LEGEND_EDGE;
        for &(tex, w) in legends {
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
    }

    pub fn network_keyboard_key(&self) -> Option<slot_ui::Key> {
        self.overlay.network.typing.as_ref().map(|(k, ..)| k.key())
    }

    /// For tests: what the keyboard has typed so far, while it is up.
    pub fn network_password(&self) -> Option<&str> {
        self.overlay.network.typing.as_ref().map(|(k, ..)| k.text())
    }
}
