mod common;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use common::{app_playing_in, clocked, tmp_root_with_carts};
use slot::app::{App, Phase};
use slot::persist::Snapshot;
use slot_gfx::{FrameBlend, DIM_MAX};
use slot_input::{Action, Btn};
use slot_store::{read_extras, read_slot_state, Shader, Zone, BRIGHTNESS_MAX};
use slot_ui::{ClockPicker, QuickRow, QuickValue};

fn press(a: &mut App, btn: Btn) {
    a.apply(Action::GbaDown(btn));
    a.apply(Action::GbaUp(btn));
}

fn open_shelf_row(a: &mut App, row: QuickRow) {
    a.apply(Action::QuickMenu);
    let mut path = vec![row];
    while let Some(parent) = path.last().and_then(|r| r.parent()) {
        path.push(parent);
    }
    for (depth, step) in path.iter().rev().enumerate() {
        if depth > 0 {
            press(a, Btn::A);
        }
        for _ in 0..step.position(&a.hidden_rows()) {
            press(a, Btn::Down);
        }
    }
}

fn open_play_row(a: &mut App, row: QuickRow) {
    a.apply(Action::QuickMenu);
    let path: &[QuickRow] = match row.page() {
        p if p == QuickRow::PLAY => &[],
        p if p == QuickRow::SETTINGS => &[QuickRow::Settings],
        p if p == QuickRow::PLAY_SCREEN => &[QuickRow::Settings, QuickRow::PlayScreen],
        _ => &[QuickRow::Settings, QuickRow::PlayGameplay],
    };
    for step in path.iter().copied().chain([row]) {
        for _ in 0..16 {
            if a.in_game_menu() == Some(step) {
                break;
            }
            press(a, Btn::Down);
        }
        assert_eq!(a.in_game_menu(), Some(step), "never reached {step:?}");
        if step != row {
            press(a, Btn::A);
        }
    }
}

#[test]
fn timezone_picker_includes_asia_brunei_and_clock_picker_uses_it() {
    let brunei = Zone::parse("Asia/Brunei").expect("Asia/Brunei must be in Zone::ALL");
    assert_eq!(brunei.offset_min(0), 480);

    let d = tmp_root_with_carts(&["Emerald", "Fusion"]);
    clocked(d.path());
    let mut a = App::boot(d.path());
    open_shelf_row(&mut a, QuickRow::Zone);
    assert_eq!(a.quick_menu(), Some(QuickRow::Zone));

    let mut found = false;
    for _ in 0..slot_store::Zone::ALL.len() {
        if a.zone().map(|z| z.id()) == Some("Asia/Brunei") {
            found = true;
            break;
        }
        press(&mut a, Btn::Right);
    }
    assert!(found, "stepping QuickRow::Zone did not reach Asia/Brunei");
    assert_eq!(a.zone_label(), "Brunei");
    assert_eq!(read_slot_state(d.path()).utc_offset_min, 480);
    assert_eq!(read_extras(d.path()).get("zone"), Some("Asia/Brunei"));

    // Booting with clock_set = false and Zone = Asia/Brunei starts ClockPicker on local Brunei time.
    let mut s = read_slot_state(d.path());
    s.clock_set = false;
    slot_store::write_slot_state(d.path(), &s).expect("write slot.state");
    let booted = App::boot(d.path());
    let expected = ClockPicker::local_zone(0, brunei);
    assert_eq!(expected.offset_min(), 480);
    assert_eq!(expected.zone(), Some(brunei));
    assert!(matches!(booted.phase(), Phase::SetClock { .. }));
}

#[test]
fn brightness_steps_below_zero_into_shader_dim_levels_and_clamps() {
    let d = tmp_root_with_carts(&["Emerald"]);
    let mut a = app_playing_in(d.path(), "Emerald");

    // Step all the way down to hardware 0 + max shader dim (DIM_MAX = 6 -> -6 total).
    for _ in 0..=(BRIGHTNESS_MAX + DIM_MAX + 2) {
        a.apply(Action::BrightnessDown);
    }
    assert_eq!(a.brightness(), 0);
    assert_eq!(a.dim_step(), DIM_MAX);
    assert_eq!(read_extras(d.path()).get("dim"), Some("6"));

    // Stepping back up clears shader dim first, then increases backlight up to BRIGHTNESS_MAX.
    for _ in 0..DIM_MAX {
        a.apply(Action::BrightnessUp);
    }
    assert_eq!(a.dim_step(), 0);
    assert_eq!(a.brightness(), 0);

    for _ in 0..=(BRIGHTNESS_MAX + 2) {
        a.apply(Action::BrightnessUp);
    }
    assert_eq!(a.dim_step(), 0);
    assert_eq!(a.brightness(), BRIGHTNESS_MAX);

    // QuickRow::PlayBrightness works down through Dim1..Dim6 too and persists on menu close.
    open_play_row(&mut a, QuickRow::PlayBrightness);
    for _ in 0..(BRIGHTNESS_MAX + 3) {
        press(&mut a, Btn::Left);
    }
    assert_eq!(a.dim_step(), 3);
    assert_eq!(
        a.quick_value(QuickRow::PlayBrightness),
        Some(QuickValue::Dim3)
    );
    for _ in 0..4 {
        press(&mut a, Btn::B);
    }
    assert_eq!(read_slot_state(d.path()).brightness, 0);
    assert_eq!(read_extras(d.path()).get("dim"), Some("3"));
}

#[test]
fn screen_calibration_shimmerless_grid_and_blending_persist() {
    let d = tmp_root_with_carts(&["Emerald", "Fusion"]);
    clocked(d.path());
    let mut a = App::boot(d.path());

    // Screen -> General -> Warmth / Tint / Gamma Curve
    open_shelf_row(&mut a, QuickRow::CalWarm);
    assert_eq!(a.cal_warm(), 0);
    press(&mut a, Btn::Right);
    press(&mut a, Btn::Right);
    assert_eq!(a.cal_warm(), 20);

    press(&mut a, Btn::Down);
    assert_eq!(a.quick_menu(), Some(QuickRow::CalTint));
    press(&mut a, Btn::Left);
    assert_eq!(a.cal_tint(), -10);

    press(&mut a, Btn::Down);
    assert_eq!(a.quick_menu(), Some(QuickRow::CalCurve));
    press(&mut a, Btn::Right);
    assert_eq!(a.cal_curve(), 5);

    let extras = read_extras(d.path());
    assert_eq!(extras.get("cal_warm"), Some("20"));
    assert_eq!(extras.get("cal_tint"), Some("-10"));
    assert_eq!(extras.get("cal_curve"), Some("5"));

    // Screen -> GB -> Shader includes ShimmerlessGrid.
    press(&mut a, Btn::B);
    for _ in 0..4 {
        if a.quick_menu() == Some(QuickRow::ScreenGb) {
            break;
        }
        press(&mut a, Btn::Down);
    }
    assert_eq!(a.quick_menu(), Some(QuickRow::ScreenGb));
    press(&mut a, Btn::A);
    for _ in 0..4 {
        if a.quick_menu() == Some(QuickRow::GbShader) {
            break;
        }
        press(&mut a, Btn::Down);
    }
    assert_eq!(a.quick_menu(), Some(QuickRow::GbShader));

    let mut hit_shimmerless = false;
    for _ in 0..8 {
        if a.quick_value(QuickRow::GbShader) == Some(QuickValue::ShimmerlessGrid) {
            hit_shimmerless = true;
            break;
        }
        press(&mut a, Btn::Right);
    }
    assert!(hit_shimmerless);
    assert_eq!(
        Shader::parse("shimmerless-grid"),
        Some(Shader::ShimmerlessGrid)
    );

    // Screen -> GBA -> Blending includes Off/Mix/Ghosting (and menu close persists shader_gb).
    press(&mut a, Btn::B);
    for _ in 0..4 {
        if a.quick_menu() == Some(QuickRow::ScreenGba) {
            break;
        }
        press(&mut a, Btn::Up);
    }
    assert_eq!(a.quick_menu(), Some(QuickRow::ScreenGba));
    press(&mut a, Btn::A);
    for _ in 0..6 {
        if a.quick_menu() == Some(QuickRow::MotionBlurGba) {
            break;
        }
        press(&mut a, Btn::Down);
    }
    assert_eq!(a.quick_menu(), Some(QuickRow::MotionBlurGba));
    assert_eq!(a.frame_blend(), FrameBlend::Off);
    press(&mut a, Btn::Right);
    assert_eq!(a.frame_blend(), FrameBlend::Mix);
    press(&mut a, Btn::Right);
    assert_eq!(a.frame_blend(), FrameBlend::Ghosting);
    for _ in 0..4 {
        press(&mut a, Btn::B);
    }
    assert_eq!(read_slot_state(d.path()).shader_gb, Shader::ShimmerlessGrid);
    assert_eq!(read_extras(d.path()).get("blending.gba"), Some("ghosting"));
}

#[test]
fn system_menu_reorganisation_and_boot_logo_and_haptics() {
    // Reorganized v1.15.3 structure: Update & About sit under System, not MAIN.
    assert!(!QuickRow::MAIN.contains(&QuickRow::Update));
    assert!(!QuickRow::MAIN.contains(&QuickRow::About));
    assert!(QuickRow::MAIN.contains(&QuickRow::System));
    assert!(QuickRow::SYSTEM.contains(&QuickRow::Zone));
    assert!(QuickRow::SYSTEM.contains(&QuickRow::Hostname));
    assert!(QuickRow::SYSTEM.contains(&QuickRow::Nickname));
    assert!(QuickRow::SYSTEM.contains(&QuickRow::BootLogo));
    assert!(QuickRow::SYSTEM.contains(&QuickRow::Haptics));
    assert!(QuickRow::SYSTEM.contains(&QuickRow::Update));
    assert!(QuickRow::SYSTEM.contains(&QuickRow::About));

    let d = tmp_root_with_carts(&["Emerald", "Fusion"]);
    clocked(d.path());

    // Drop a valid PNG under Bootlogos/Custom.png
    let logos = d.path().join("Bootlogos");
    std::fs::create_dir_all(&logos).expect("Bootlogos dir");
    let f = std::fs::File::create(logos.join("Custom.png")).expect("create logo png");
    let mut enc = png::Encoder::new(std::io::BufWriter::new(f), 2, 2);
    enc.set_color(png::ColorType::Rgba);
    enc.set_depth(png::BitDepth::Eight);
    enc.write_header()
        .expect("png header")
        .write_image_data(&[255u8; 16])
        .expect("png pixels");

    let mut a = App::boot(d.path());
    open_shelf_row(&mut a, QuickRow::BootLogo);
    assert_eq!(a.quick_menu(), Some(QuickRow::BootLogo));
    assert_eq!(a.boot_logo_label(), "Slot");

    press(&mut a, Btn::Right);
    assert_eq!(a.boot_logo_label(), "Custom");
    assert!(d.path().join("Config/bootlogo.bmp").exists());

    press(&mut a, Btn::Down);
    assert_eq!(a.quick_menu(), Some(QuickRow::Haptics));
    assert!(a.haptics_enabled());
    press(&mut a, Btn::Right);
    assert!(!a.haptics_enabled());
    assert_eq!(read_extras(d.path()).get("haptics"), Some("0"));
}

struct ResettableSnapshot {
    reset_called: Arc<AtomicBool>,
    loaded: Arc<Mutex<Option<Vec<u8>>>>,
    state: Vec<u8>,
}

impl Snapshot for ResettableSnapshot {
    fn state(&self) -> Option<Vec<u8>> {
        Some(self.state.clone())
    }

    fn save_ram(&self) -> Option<Vec<u8>> {
        None
    }

    fn thumb(&self) -> Option<Vec<u8>> {
        None
    }

    fn load(&self, state: Vec<u8>) {
        *self.loaded.lock().expect("loaded lock") = Some(state);
    }

    fn reset(&self) {
        self.reset_called.store(true, Ordering::SeqCst);
    }
}

#[test]
fn reset_game_shortcut_and_undo_reset_lifecycle() {
    let d = tmp_root_with_carts(&["Emerald"]);
    let mut a = app_playing_in(d.path(), "Emerald");
    let reset_called = Arc::new(AtomicBool::new(false));
    let loaded = Arc::new(Mutex::new(None));
    a.set_snapshot(Box::new(ResettableSnapshot {
        reset_called: reset_called.clone(),
        loaded: loaded.clone(),
        state: vec![42u8; 16],
    }));

    // UndoReset starts hidden until a reset runs.
    assert!(a.in_game_hidden().contains(&QuickRow::UndoReset));

    a.apply_at(Action::ResetGame, 100);
    assert!(reset_called.load(Ordering::SeqCst));
    assert!(!a.in_game_hidden().contains(&QuickRow::UndoReset));

    // Choosing UndoReset from the in-game menu restores the prior state.
    open_play_row(&mut a, QuickRow::UndoReset);
    press(&mut a, Btn::A);
    assert_eq!(*loaded.lock().expect("loaded lock"), Some(vec![42u8; 16]));
    assert!(a.in_game_hidden().contains(&QuickRow::UndoReset));
}
