// Fragile: a game on more than one disc, and the note for a disc system with no BIOS (Phase 12).

mod common;

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use common::{app_playing_in, tmp_root_with_carts};
use slot::app::App;
use slot::audio::{AudioSink, StubSink};
use slot::emu::{CoreState, EmuHandle, Speed};
use slot_input::{Action, Btn};
use slot_retro::{AvInfo, ButtonMask, CoreError, MockCore, RetroCore};
use slot_store::Platform;
use slot_ui::{QuickRow, Toast};

fn press(a: &mut App, btn: Btn) {
    a.apply(Action::GbaDown(btn));
    a.apply(Action::GbaUp(btn));
}

fn to_disc_row(a: &mut App) {
    a.apply(Action::QuickMenu);
    for _ in 0..8 {
        if a.in_game_menu() == Some(QuickRow::PlayDisc) {
            return;
        }
        press(a, Btn::Down);
    }
    panic!("the Disc row never came up");
}

#[test]
fn the_disc_row_shows_only_for_a_game_on_more_than_one_disc() {
    let d = tmp_root_with_carts(&["Emerald"]);
    let mut a = app_playing_in(d.path(), "Emerald");
    assert!(a.in_game_hidden().contains(&QuickRow::PlayDisc));
    a.set_discs(Some((0, 1)));
    assert!(a.in_game_hidden().contains(&QuickRow::PlayDisc));
    a.set_discs(Some((0, 3)));
    assert!(!a.in_game_hidden().contains(&QuickRow::PlayDisc));
    assert_eq!(a.named_value(QuickRow::PlayDisc).as_deref(), Some("1 of 3"));
    a.set_discs(None);
    assert!(a.in_game_hidden().contains(&QuickRow::PlayDisc));
}

#[test]
fn left_and_right_ask_for_the_next_disc_round_the_ends() {
    let d = tmp_root_with_carts(&["Emerald"]);
    let mut a = app_playing_in(d.path(), "Emerald");
    a.set_discs(Some((0, 3)));
    to_disc_row(&mut a);
    press(&mut a, Btn::Right);
    assert_eq!(
        a.named_value(QuickRow::PlayDisc).as_deref(),
        Some("2 of 3"),
        "the row waits for the core to show the disc asked for"
    );
    assert_eq!(a.take_disc(), Some(1));
    assert_eq!(a.take_disc(), None, "the disc was asked for twice");
    a.set_discs(Some((1, 3)));
    assert_eq!(a.named_value(QuickRow::PlayDisc).as_deref(), Some("2 of 3"));
    a.set_discs(Some((0, 3)));
    press(&mut a, Btn::Left);
    assert_eq!(a.take_disc(), Some(2), "left from the first went nowhere");
}

#[test]
fn a_disc_system_with_no_bios_says_so_once() {
    let d = tmp_root_with_carts(&["Emerald"]);
    let mut a = app_playing_in(d.path(), "Emerald");
    a.core_opened(Platform::Snes, true);
    assert_eq!(
        a.toast(),
        None,
        "a system that needs no BIOS was told of one"
    );
    a.core_opened(Platform::PlayStation, false);
    assert_eq!(a.toast(), None, "the mock core was taken for the real one");
    a.core_opened(Platform::PlayStation, true);
    assert_eq!(a.toast(), Some(Toast::BuiltInBios));
    let noted = slot_store::read_extras(d.path());
    assert!(noted.flag("bios_noted.ps", false), "{:?}", noted.text());

    let mut again = app_playing_in(d.path(), "Emerald");
    again.core_opened(Platform::PlayStation, true);
    assert_eq!(again.toast(), None, "the note came back");
}

#[test]
fn a_bios_on_the_card_needs_no_note() {
    let d = tmp_root_with_carts(&["Emerald"]);
    std::fs::create_dir_all(d.path().join("BIOS")).unwrap();
    std::fs::write(d.path().join("BIOS/scph5501.bin"), b"").unwrap();
    let mut a = app_playing_in(d.path(), "Emerald");
    a.core_opened(Platform::PlayStation, true);
    assert_eq!(a.toast(), None);
}

/// A mock core with three discs.
struct Discs {
    inner: MockCore,
    at: Arc<Mutex<u32>>,
    /// With a tray: whether it is open, and the frames run while it was.
    tray: Option<Arc<Mutex<(bool, u32)>>>,
}

impl RetroCore for Discs {
    fn load(&mut self, rom: &Path) -> Result<(), CoreError> {
        self.inner.load(rom)
    }
    fn run_frame(&mut self, input: ButtonMask) {
        if let Some(tray) = &self.tray {
            let mut t = tray.lock().unwrap();
            if t.0 {
                t.1 += 1;
            }
        }
        self.inner.run_frame(input)
    }
    fn video_xrgb8888(&self) -> &[u8] {
        self.inner.video_xrgb8888()
    }
    fn take_audio(&mut self) -> Vec<i16> {
        self.inner.take_audio()
    }
    fn serialize(&mut self) -> Result<Vec<u8>, CoreError> {
        self.inner.serialize()
    }
    fn unserialize(&mut self, data: &[u8]) -> Result<(), CoreError> {
        self.inner.unserialize(data)
    }
    fn save_ram(&self) -> Option<Vec<u8>> {
        self.inner.save_ram()
    }
    fn load_save_ram(&mut self, data: &[u8]) -> Result<(), CoreError> {
        self.inner.load_save_ram(data)
    }
    fn av_info(&self) -> AvInfo {
        self.inner.av_info()
    }
    fn discs(&self) -> Option<(u32, u32)> {
        Some((*self.at.lock().unwrap(), 3))
    }
    fn set_disc(&mut self, index: u32) -> bool {
        *self.at.lock().unwrap() = index;
        if let Some(tray) = &self.tray {
            tray.lock().unwrap().0 = false;
        }
        true
    }
    fn open_tray(&mut self) -> bool {
        let Some(tray) = &self.tray else {
            return false;
        };
        tray.lock().unwrap().0 = true;
        true
    }
}

fn until(what: impl Fn() -> bool) -> bool {
    let deadline = Instant::now() + Duration::from_secs(2);
    while !what() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(2));
    }
    what()
}

#[test]
fn the_emulator_puts_the_disc_in_and_says_which_is_in() {
    let mut sink = StubSink::new();
    sink.open(32_768).unwrap();
    let drained = sink.clone();
    std::thread::spawn(move || loop {
        drained.device_drain();
        std::thread::sleep(Duration::from_millis(2));
    });
    let at = Arc::new(Mutex::new(0));
    let core = Discs {
        inner: MockCore::new(),
        at: at.clone(),
        tray: None,
    };
    let emu = EmuHandle::spawn(
        Box::new(core),
        PathBuf::from("mock"),
        sink.ring(),
        None,
        None,
    );
    emu.set_speed(Speed::Normal);
    assert!(until(|| emu.state() == CoreState::Ready));
    assert_eq!(emu.discs(), Some((0, 3)));
    emu.set_disc(2);
    assert!(until(|| emu.discs() == Some((2, 3))), "{:?}", emu.discs());
    assert_eq!(*at.lock().unwrap(), 2);
}

/// Fragile: with a tray, the disc goes in only after the game has run a second with it open, and
/// not while the game is paused, as it is under the menu.
#[test]
fn a_disc_goes_in_after_the_game_has_seen_the_tray_open() {
    let mut sink = StubSink::new();
    sink.open(32_768).unwrap();
    let drained = sink.clone();
    std::thread::spawn(move || loop {
        drained.device_drain();
        std::thread::sleep(Duration::from_millis(2));
    });
    let at = Arc::new(Mutex::new(0));
    let tray = Arc::new(Mutex::new((false, 0)));
    let core = Discs {
        inner: MockCore::new(),
        at: at.clone(),
        tray: Some(tray.clone()),
    };
    let emu = EmuHandle::spawn(
        Box::new(core),
        PathBuf::from("mock"),
        sink.ring(),
        None,
        None,
    );
    emu.set_speed(Speed::Normal);
    assert!(until(|| emu.state() == CoreState::Ready));
    emu.set_speed(Speed::Paused);
    emu.set_disc(2);
    assert!(until(|| tray.lock().unwrap().0), "the tray never opened");
    std::thread::sleep(Duration::from_millis(200));
    assert_eq!(emu.discs(), Some((0, 3)), "the disc went in while paused");
    emu.set_speed(Speed::Normal);
    assert!(until(|| emu.discs() == Some((2, 3))), "{:?}", emu.discs());
    let (open, ran) = *tray.lock().unwrap();
    assert!(!open, "the tray was left open");
    assert!(
        ran >= 60,
        "the game ran only {ran} frames with the tray open"
    );
}
