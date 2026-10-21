// A system drawn at its own size: the core's picture, its shape and its save-state thumbnail
// keep the size the core drew, while the Game Boy family keeps its 240x160 canvas.

use std::path::PathBuf;
use std::time::{Duration, Instant};

use slot::audio::{AudioSink, StubSink};
use slot::emu::{CoreState, EmuHandle, Speed};
use slot::persist::Snapshot;
use slot_retro::{MockCore, RetroCore, GBA_H, GBA_W, MOCK_NATIVE, MOCK_NATIVE_ASPECT};

fn wait_for(cond: impl Fn() -> bool) -> bool {
    let deadline = Instant::now() + Duration::from_secs(2);
    while Instant::now() < deadline {
        if cond() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(2));
    }
    false
}

fn spawn(native: bool) -> EmuHandle {
    let mut sink = StubSink::new();
    sink.open(32_768).expect("the stub refused to open");
    let drained = sink.clone();
    std::thread::spawn(move || loop {
        drained.device_drain();
        std::thread::sleep(Duration::from_millis(2));
    });
    let mut core = MockCore::new();
    core.set_native(native);
    let emu = EmuHandle::spawn(
        Box::new(core),
        PathBuf::from("mock"),
        sink.ring(),
        None,
        None,
    );
    emu.set_speed(Speed::Normal);
    assert!(wait_for(|| emu.state() == CoreState::Ready));
    emu
}

#[test]
fn a_native_cores_frames_come_at_its_own_size_and_shape() {
    let emu = spawn(true);
    assert!(wait_for(|| emu.published_count() > 2));
    let frame = std::iter::repeat_with(|| emu.latest_frame())
        .take(500)
        .find_map(|f| {
            std::thread::sleep(Duration::from_millis(2));
            f
        })
        .expect("nothing was published");
    let shape = frame.shape();
    assert_eq!(shape.size, MOCK_NATIVE);
    assert_eq!(shape.aspect, Some(MOCK_NATIVE_ASPECT));
    assert_eq!(frame.len(), (MOCK_NATIVE.0 * MOCK_NATIVE.1 * 4) as usize);
}

#[test]
fn the_game_boy_family_keeps_its_canvas_and_no_shape() {
    let emu = spawn(false);
    assert!(wait_for(|| emu.published_count() > 2));
    let frame = std::iter::repeat_with(|| emu.latest_frame())
        .take(500)
        .find_map(|f| {
            std::thread::sleep(Duration::from_millis(2));
            f
        })
        .expect("nothing was published");
    assert_eq!(frame.shape().size, (GBA_W, GBA_H));
    assert_eq!(frame.shape().aspect, None);
}

#[test]
fn a_native_save_state_thumbnail_is_the_size_the_core_drew() {
    let emu = spawn(true);
    let png = emu.snapshot().thumb().expect("no thumbnail");
    let reader = png::Decoder::new(png.as_slice()).read_info().unwrap();
    let info = reader.info();
    assert_eq!((info.width, info.height), MOCK_NATIVE);
}
