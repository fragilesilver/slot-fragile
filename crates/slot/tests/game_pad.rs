// The buttons past upstream's twelve: X, Y, L2 and R2 for a system whose pad has them. The
// Game Boy family's pads, its controls file and its turbo are as upstream has them.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use slot::audio::{AudioSink, StubSink};
use slot::emu::{CoreState, EmuHandle, Speed};
use slot::input::Pad;
use slot_input::{Action, Btn};
use slot_retro::{AvInfo, ButtonMask, CoreError, MockCore, RetroCore};
use slot_store::{read_controls, write_controls, Binding, Controls, Device, GameButton, Platform};

#[test]
fn the_game_boy_familys_pads_are_upstreams() {
    assert_eq!(GameButton::of(Platform::Gba), &GameButton::ALL[..]);
    for p in [Platform::Gb, Platform::Gbc] {
        let pad = GameButton::of(p);
        assert!(!pad.contains(&GameButton::L) && !pad.contains(&GameButton::R));
        assert!(pad.contains(&GameButton::TurboA));
        for g in GameButton::EXTRA {
            assert!(!pad.contains(&g), "{p:?} has {g:?}");
        }
    }
}

#[test]
fn the_controls_file_names_an_extra_button_only_once_it_is_moved() {
    let d = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(d.path().join("Config")).unwrap();
    write_controls(d.path(), &Controls::default()).unwrap();
    let text = std::fs::read_to_string(d.path().join("Config/controls.txt")).unwrap();
    assert_eq!(text.lines().count(), 12, "{text}");
    let mut moved = Controls::default();
    moved.set_game(GameButton::L2, Binding::plain(Device::L1));
    write_controls(d.path(), &moved).unwrap();
    let text = std::fs::read_to_string(d.path().join("Config/controls.txt")).unwrap();
    assert!(text.contains("l2=l1"), "{text}");
    assert_eq!(read_controls(d.path()), moved);
}

#[test]
fn only_the_playing_pads_extra_buttons_reach_the_core() {
    let controls = Controls::default();
    let mut pad = Pad::default();
    pad.apply(Action::GbaDown(Btn::X));
    pad.apply(Action::GbaDown(Btn::L2));
    let face = [GameButton::X, GameButton::Y, GameButton::L2, GameButton::R2];
    assert_eq!(
        pad.extra_mask(&controls, &face),
        ButtonMask(ButtonMask::X | ButtonMask::L2)
    );
    assert_eq!(
        pad.extra_mask(&controls, GameButton::of(Platform::Gba)),
        ButtonMask(0),
        "the GBA's pad took an extra button"
    );
}

struct Recorder {
    inner: MockCore,
    seen: Arc<Mutex<Vec<u16>>>,
}

impl RetroCore for Recorder {
    fn load(&mut self, rom: &Path) -> Result<(), CoreError> {
        self.inner.load(rom)
    }
    fn run_frame(&mut self, input: ButtonMask) {
        self.seen.lock().unwrap().push(input.0);
        self.inner.run_frame(input);
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
}

fn seen_with(face_buttons: bool) -> Vec<u16> {
    let mut sink = StubSink::new();
    sink.open(32_768).unwrap();
    let drained = sink.clone();
    std::thread::spawn(move || loop {
        drained.device_drain();
        std::thread::sleep(Duration::from_millis(2));
    });
    let seen = Arc::new(Mutex::new(Vec::new()));
    let core = Recorder {
        inner: MockCore::new(),
        seen: seen.clone(),
    };
    let emu = EmuHandle::spawn(
        Box::new(core),
        PathBuf::from("mock"),
        sink.ring(),
        None,
        None,
    );
    emu.set_speed(Speed::Normal);
    emu.set_turbo(true);
    emu.set_face_buttons(face_buttons);
    let deadline = Instant::now() + Duration::from_secs(2);
    while emu.state() != CoreState::Ready && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(2));
    }
    emu.set_input(ButtonMask(ButtonMask::X));
    std::thread::sleep(Duration::from_millis(200));
    let frames = seen.lock().unwrap().clone();
    frames.into_iter().filter(|m| *m != 0).collect()
}

#[test]
fn a_pad_of_its_own_x_sends_x_where_turbo_would_have_fired_a() {
    let face = seen_with(true);
    assert!(!face.is_empty(), "the core never saw the button");
    assert!(
        face.iter().all(|m| *m == ButtonMask::X),
        "X did not reach the core as X: {face:x?}"
    );
    let turbo = seen_with(false);
    assert!(
        turbo.iter().all(|m| *m & ButtonMask::X == 0) && turbo.contains(&ButtonMask::A),
        "turbo no longer turns X into a pulsed A: {turbo:x?}"
    );
}
