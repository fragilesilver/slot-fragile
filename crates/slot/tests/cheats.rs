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
use slot_ui::QuickRow;
use tempfile::TempDir;

const CHT: &str = "cheats = 3\n\
cheat0_desc = \"Master Code\"\ncheat0_code = \"D8BAE4D9+4864DCE5\"\ncheat0_enable = false\n\
cheat1_desc = \"Max Money\"\ncheat1_code = \"A86CDBA5+19BB1D4C\"\ncheat1_enable = false\n\
cheat2_desc = \"Empty\"\ncheat2_code = \"\"\n";

fn card_with_cheats() -> TempDir {
    let d = tmp_root_with_carts(&["Emerald", "Fusion"]);
    std::fs::write(d.path().join("Cheats/GBA/Emerald (Code Breaker).cht"), CHT).unwrap();
    d
}

fn press(a: &mut App, btn: Btn) {
    a.apply(Action::GbaDown(btn));
    a.apply(Action::GbaUp(btn));
}

fn open_cheats(a: &mut App) {
    a.apply(Action::QuickMenu);
    for _ in 0..8 {
        if a.in_game_menu() == Some(QuickRow::PlayCheats) {
            break;
        }
        press(a, Btn::Down);
    }
    assert_eq!(a.in_game_menu(), Some(QuickRow::PlayCheats));
    press(a, Btn::A);
}

fn values(a: &App) -> Vec<(String, Option<String>)> {
    a.overlay_page()
        .expect("the Cheats page is not open")
        .rows()
        .iter()
        .map(|r| (r.label.clone(), r.value.clone()))
        .collect()
}

#[test]
fn the_cheats_row_shows_only_for_a_game_that_has_cheats() {
    let d = card_with_cheats();
    let a = app_playing_in(d.path(), "Emerald");
    assert!(!a.in_game_hidden().contains(&QuickRow::PlayCheats));
    let a = app_playing_in(d.path(), "Fusion");
    assert!(a.in_game_hidden().contains(&QuickRow::PlayCheats));
}

#[test]
fn a_turns_a_cheat_on_the_core_gets_its_code_and_the_game_remembers_it() {
    let d = card_with_cheats();
    let mut a = app_playing_in(d.path(), "Emerald");
    assert_eq!(
        a.take_cheats(),
        Some(Vec::new()),
        "the new core was not told"
    );
    assert_eq!(a.take_cheats(), None, "told twice");
    open_cheats(&mut a);
    let off = |name: &str| (name.to_string(), Some("Off".to_string()));
    assert_eq!(values(&a), [off("Master Code"), off("Max Money")]);
    press(&mut a, Btn::Down);
    press(&mut a, Btn::A);
    assert_eq!(values(&a)[1].1.as_deref(), Some("On"));
    assert_eq!(a.take_cheats(), Some(vec!["A86CDBA5+19BB1D4C".to_string()]));
    let kept = std::fs::read_to_string(d.path().join("Config/Cheats/GBA/Emerald.txt")).unwrap();
    assert_eq!(kept, "Max Money\n");
    press(&mut a, Btn::B);
    assert!(a.overlay_page().is_none());
    assert_eq!(a.in_game_menu(), Some(QuickRow::PlayCheats));

    let mut again = app_playing_in(d.path(), "Emerald");
    assert_eq!(again.cheats_on(), ["Max Money"]);
    assert_eq!(
        again.take_cheats(),
        Some(vec!["A86CDBA5+19BB1D4C".to_string()]),
        "the next start did not hand the core the cheat left on"
    );
}

#[test]
fn turning_the_last_cheat_off_forgets_the_game() {
    let d = card_with_cheats();
    let mut a = app_playing_in(d.path(), "Emerald");
    open_cheats(&mut a);
    press(&mut a, Btn::A);
    press(&mut a, Btn::A);
    assert!(!d.path().join("Config/Cheats/GBA/Emerald.txt").exists());
}

#[test]
fn the_android_ports_plain_lists_are_read_too() {
    let d = tmp_root_with_carts(&["Emerald"]);
    std::fs::write(
        d.path().join("Cheats/GBA/Emerald.txt"),
        "D8BAE4D9 4864DCE5 # Master code\n",
    )
    .unwrap();
    std::fs::write(d.path().join("Cheats/Emerald.txt"), "ABCD1234 # Loose\n").unwrap();
    let list = slot::cheats::load(d.path(), Platform::Gba, "Emerald");
    let names: Vec<&str> = list.iter().map(|c| c.name.as_str()).collect();
    assert_eq!(names, ["Master code", "Loose"]);
}

struct Recorder {
    inner: MockCore,
    given: Arc<Mutex<Vec<Vec<String>>>>,
}

impl RetroCore for Recorder {
    fn load(&mut self, rom: &Path) -> Result<(), CoreError> {
        self.inner.load(rom)
    }
    fn run_frame(&mut self, input: ButtonMask) {
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
    fn set_cheats(&mut self, codes: &[String]) {
        self.given.lock().unwrap().push(codes.to_vec());
    }
}

#[test]
fn a_loaded_state_gets_the_cheats_back() {
    let mut sink = StubSink::new();
    sink.open(32_768).unwrap();
    let drained = sink.clone();
    std::thread::spawn(move || loop {
        drained.device_drain();
        std::thread::sleep(Duration::from_millis(2));
    });
    let given = Arc::new(Mutex::new(Vec::new()));
    let core = Recorder {
        inner: MockCore::new(),
        given: given.clone(),
    };
    let emu = EmuHandle::spawn(
        Box::new(core),
        PathBuf::from("mock"),
        sink.ring(),
        None,
        None,
    );
    emu.set_speed(Speed::Normal);
    let deadline = Instant::now() + Duration::from_secs(2);
    while emu.state() != CoreState::Ready && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(2));
    }
    let codes = vec!["A86CDBA5+19BB1D4C".to_string()];
    emu.set_cheats(codes.clone());
    emu.request_load(0u64.to_le_bytes().to_vec());
    let deadline = Instant::now() + Duration::from_secs(2);
    while given.lock().unwrap().len() < 2 && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(2));
    }
    assert_eq!(*given.lock().unwrap(), [codes.clone(), codes]);
}

#[test]
fn a_linked_game_plays_without_cheats_and_gets_them_back_after() {
    let d = card_with_cheats();
    std::fs::create_dir_all(d.path().join("Config/Cheats/GBA")).unwrap();
    std::fs::write(
        d.path().join("Config/Cheats/GBA/Emerald.txt"),
        "Max Money\n",
    )
    .unwrap();
    let mut a = app_playing_in(d.path(), "Emerald");
    let on = vec!["A86CDBA5+19BB1D4C".to_string()];
    assert_eq!(a.take_cheats(), Some(on.clone()));
    a.begin_link(1);
    assert_eq!(
        a.take_cheats(),
        Some(Vec::new()),
        "a link kept the cheats on"
    );
    assert!(a.in_game_hidden().contains(&QuickRow::PlayCheats));
    a.end_link();
    assert_eq!(
        a.take_cheats(),
        Some(on),
        "the cheats did not come back after the link"
    );
}
