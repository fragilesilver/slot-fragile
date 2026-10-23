mod common;

use std::path::Path;
use std::time::{Duration, Instant};

use common::{repo_root, session_with_platform, tmp_root_with_carts, tmp_root_with_gb_carts};
use slot::app::Phase;
use slot::session::Session;
use slot_input::{Action, Btn, Millis, RawEvent};
use slot_store::{read_slot_state, write_slot_state, GbPalette, SlotState};
use slot_ui::Toast;
use tempfile::TempDir;

const FRAME_MS: Millis = 16;
const DT: f32 = 1.0 / 60.0;

fn step(s: &mut Session, now: &mut Millis) {
    *now += FRAME_MS;
    s.feed([], *now);
    s.update(DT);
}

fn event(s: &mut Session, ev: RawEvent, now: &mut Millis) {
    *now += FRAME_MS;
    s.feed([ev], *now);
    s.update(DT);
}

fn play(s: &mut Session, now: &mut Millis) {
    event(s, RawEvent::Down(Btn::A), now);
    event(s, RawEvent::Up(Btn::A), now);
    let deadline = Instant::now() + Duration::from_secs(10);
    while !matches!(s.app().phase(), Phase::Playing { .. }) {
        assert!(Instant::now() < deadline, "the cart never seated");
        step(s, now);
        std::thread::sleep(Duration::from_millis(1));
    }
}

fn catrap_with_colour_flag(cgb: u8, palettes: bool) -> Option<TempDir> {
    let real = repo_root().join("sdcard/Games/GB/Catrap (USA).gb");
    // Fragile: without Catrap on this machine, a cart of zeros: only its header is read.
    let mut rom = std::fs::read(&real).unwrap_or_else(|_| vec![0; 0x8000]);
    rom[0x143] = cgb;
    let d = tmp_root_with_gb_carts(&["Catrap"]);
    std::fs::write(d.path().join("Games/GB/Catrap.gb"), rom).expect("write rom");
    palettes_on(d.path(), palettes);
    Some(d)
}

fn palettes_on(root: &Path, on: bool) {
    write_slot_state(
        root,
        &SlotState {
            gb_palettes: on,
            ..SlotState::default()
        },
    )
    .expect("write slot.state");
}

#[test]
fn select_r2_steps_the_palette_and_names_it_in_a_game_boy_only_game() {
    let Some(d) = catrap_with_colour_flag(0x00, true) else {
        eprintln!("no Catrap on this machine's card, skipping");
        return;
    };
    let (mut s, _motor) = session_with_platform(d.path());
    let mut now = 0;
    play(&mut s, &mut now);
    assert!(
        s.app().palette_live(),
        "palettes are on and the cart is Game Boy only"
    );

    let before = s.app().gb_palette().expect("palettes are on");
    s.app_mut().apply(Action::PaletteNext);
    let after = s.app().gb_palette().expect("palettes are on");
    assert_eq!(after, before.next());
    assert_eq!(s.app().toast(), Some(Toast::Palette(after)));
    assert_eq!(
        read_slot_state(d.path()).gb_palette,
        after,
        "the pick was not saved"
    );
}

#[test]
fn select_l2_steps_the_palette_back_and_names_it() {
    let Some(d) = catrap_with_colour_flag(0x00, true) else {
        eprintln!("no Catrap on this machine's card, skipping");
        return;
    };
    let (mut s, _motor) = session_with_platform(d.path());
    let mut now = 0;
    play(&mut s, &mut now);
    let before = s.app().gb_palette().expect("palettes are on");
    s.app_mut().apply(Action::PalettePrev);
    let after = s.app().gb_palette().expect("palettes are on");
    assert_eq!(after, before.prev());
    assert_eq!(s.app().toast(), Some(Toast::Palette(after)));
    assert_eq!(read_slot_state(d.path()).gb_palette, after);
}

/// Fragile: with palettes off, the chord turns them on at the palette last chosen, in place of
/// doing nothing.
#[test]
fn select_r2_turns_palettes_on_when_they_are_off() {
    let Some(d) = catrap_with_colour_flag(0x00, false) else {
        return;
    };
    let (mut s, _motor) = session_with_platform(d.path());
    let mut now = 0;
    play(&mut s, &mut now);
    assert!(
        s.app().palette_live(),
        "a Game Boy only cart can take a palette"
    );
    assert_eq!(s.app().gb_palette(), None, "palettes start off");
    s.app_mut().apply(Action::PaletteNext);
    assert_eq!(s.app().gb_palette(), Some(GbPalette::DEFAULT));
    assert_eq!(s.app().toast(), Some(Toast::Palette(GbPalette::DEFAULT)));
    let saved = read_slot_state(d.path());
    assert!(saved.gb_palettes, "palettes were not saved on");
    s.app_mut().apply(Action::PaletteNext);
    assert_eq!(s.app().gb_palette(), Some(GbPalette::DEFAULT.next()));
}

#[test]
fn select_r2_does_nothing_in_a_colour_game() {
    for (cgb, on) in [(0x80, true), (0xC0, false)] {
        let Some(d) = catrap_with_colour_flag(cgb, on) else {
            eprintln!("no Catrap on this machine's card, skipping");
            return;
        };
        let (mut s, _motor) = session_with_platform(d.path());
        let mut now = 0;
        play(&mut s, &mut now);
        assert!(!s.app().palette_live(), "cgb={cgb:#x} on={on}");
        let before = read_slot_state(d.path()).gb_palette;
        s.app_mut().apply(Action::PaletteNext);
        assert_eq!(
            s.app().toast(),
            None,
            "cgb={cgb:#x} on={on}: a palette toast showed"
        );
        assert_eq!(read_slot_state(d.path()).gb_palette, before);
        assert_eq!(GbPalette::DEFAULT, before);
    }
}

#[test]
fn select_r2_on_the_carousel_does_nothing() {
    let d = tmp_root_with_carts(&["Emerald"]);
    palettes_on(d.path(), true);
    let (mut s, _motor) = session_with_platform(d.path());
    s.app_mut().apply(Action::PaletteNext);
    assert_eq!(s.app().toast(), None);
    assert_eq!(read_slot_state(d.path()).gb_palette, GbPalette::DEFAULT);
}
