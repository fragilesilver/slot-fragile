// Screensaver: idle inactivity triggers screensaver styles (Black, Dim, Clock, Cart Art, Auto-Doze)
// and waking absorbs the triggering button press.

mod common;

use std::time::Duration;

use common::{app_playing_in, boot, panel, tmp_root_with_carts};
use slot::app::{App, Phase, ScreensaverStyle};
use slot_gfx::Draw;
use slot_input::{Action, Btn};
use slot_store::{read_extras, write_extras};
use slot_ui::QuickRow;

fn advance_secs(a: &mut App, seconds: f32) {
    let frames = (seconds * 60.0).round() as usize;
    for _ in 0..frames {
        a.update(1.0 / 60.0);
    }
}

#[test]
fn screensaver_settings_defaults_and_stepping() {
    let d = tmp_root_with_carts(&["Emerald"]);
    let mut a = boot(d.path());

    assert_eq!(a.screensaver_timeout_min(), 0);
    assert_eq!(a.screensaver_timeout_text(), "Off");
    assert_eq!(a.screensaver_style(), ScreensaverStyle::Black);
    assert_eq!(a.named_value(QuickRow::Screensaver), Some("Off".into()));
    assert_eq!(
        a.named_value(QuickRow::ScreensaverStyle),
        Some("Black".into())
    );

    // Step timeout right
    a.step_screensaver_timeout(true);
    assert_eq!(a.screensaver_timeout_min(), 1);
    assert_eq!(a.screensaver_timeout_text(), "1 min");

    a.step_screensaver_timeout(true);
    assert_eq!(a.screensaver_timeout_min(), 2);

    a.step_screensaver_timeout(true);
    assert_eq!(a.screensaver_timeout_min(), 5);

    a.step_screensaver_timeout(true);
    assert_eq!(a.screensaver_timeout_min(), 10);

    a.step_screensaver_timeout(true);
    assert_eq!(a.screensaver_timeout_min(), 0);

    // Step style right
    a.step_screensaver_style(true);
    assert_eq!(a.screensaver_style(), ScreensaverStyle::Dim);
    assert_eq!(
        a.named_value(QuickRow::ScreensaverStyle),
        Some("Dim".into())
    );

    a.step_screensaver_style(true);
    assert_eq!(a.screensaver_style(), ScreensaverStyle::Clock);

    a.step_screensaver_style(true);
    assert_eq!(a.screensaver_style(), ScreensaverStyle::CartArt);

    a.step_screensaver_style(true);
    assert_eq!(a.screensaver_style(), ScreensaverStyle::AutoDoze);

    a.step_screensaver_style(true);
    assert_eq!(a.screensaver_style(), ScreensaverStyle::Black);
}

#[test]
fn idle_timeout_triggers_screensaver_and_mutes_audio() {
    let d = tmp_root_with_carts(&["Emerald"]);
    let mut e = read_extras(d.path());
    e.set("screensaver_timeout", Some("1"));
    write_extras(d.path(), &e).unwrap();

    let mut a = app_playing_in(d.path(), "Emerald");
    a.set_power(panel(d.path(), Duration::from_secs(180)).0);

    assert_eq!(a.screensaver_timeout_min(), 1);
    assert!(!a.screensaver_active());

    // Advance 58 seconds - not idle yet
    advance_secs(&mut a, 58.0);
    assert!(!a.screensaver_active());

    // Advance 3 more seconds (61 seconds total) - idle triggered!
    advance_secs(&mut a, 3.0);
    assert!(a.screensaver_active());
    assert_eq!(a.output_volume(), 0, "audio should be muted in Black mode");

    let mut out = Vec::new();
    a.draw(&mut out);
    assert!(
        out.iter()
            .any(|d| matches!(d, Draw::Rect { colour, .. } if colour == &[0.0, 0.0, 0.0, 1.0])),
        "screensaver should render black rect"
    );
}

#[test]
fn wake_on_press_absorbs_input() {
    let d = tmp_root_with_carts(&["Emerald", "Fusion"]);
    let mut e = read_extras(d.path());
    e.set("screensaver_timeout", Some("1"));
    write_extras(d.path(), &e).unwrap();

    let mut a = boot(d.path());
    a.set_power(panel(d.path(), Duration::from_secs(180)).0);

    advance_secs(&mut a, 61.0);
    assert!(a.screensaver_active());

    // Waking with A button should exit screensaver without launching the game
    a.apply(Action::GbaDown(Btn::A));
    assert!(!a.screensaver_active(), "screensaver should be deactivated");
    assert!(
        matches!(a.phase(), Phase::Shelf),
        "A button press should have been absorbed, not launching game"
    );

    // Subsequent press when awake functions normally
    a.apply(Action::GbaDown(Btn::A));
    a.apply(Action::GbaUp(Btn::A));
    assert!(
        !matches!(a.phase(), Phase::Shelf),
        "Subsequent A press after waking should launch game"
    );
}

#[test]
fn dim_style_overlays_and_keeps_volume() {
    let d = tmp_root_with_carts(&["Emerald"]);
    let mut e = read_extras(d.path());
    e.set("screensaver_timeout", Some("1"));
    e.set("screensaver_style", Some("dim"));
    write_extras(d.path(), &e).unwrap();

    let mut a = app_playing_in(d.path(), "Emerald");
    a.set_power(panel(d.path(), Duration::from_secs(180)).0);

    advance_secs(&mut a, 61.0);
    assert!(a.screensaver_active());
    assert_eq!(a.screensaver_style(), ScreensaverStyle::Dim);
    assert!(a.output_volume() > 0, "volume should continue in Dim style");

    let mut out = Vec::new();
    a.draw(&mut out);
    assert!(
        out.iter()
            .any(|d| matches!(d, Draw::Rect { colour, .. } if colour[3] > 0.5 && colour[3] < 0.7)),
        "dim style should overlay translucent rect"
    );
}

#[test]
fn auto_doze_style_triggers_doze() {
    let d = tmp_root_with_carts(&["Emerald"]);
    let mut e = read_extras(d.path());
    e.set("screensaver_timeout", Some("1"));
    e.set("screensaver_style", Some("auto_doze"));
    write_extras(d.path(), &e).unwrap();

    let mut a = app_playing_in(d.path(), "Emerald");
    a.set_power(panel(d.path(), Duration::from_secs(180)).0);

    advance_secs(&mut a, 61.0);
    assert!(matches!(a.phase(), Phase::Doze { .. }));
}

#[test]
fn activity_resets_idle_timer() {
    let d = tmp_root_with_carts(&["Emerald"]);
    let mut e = read_extras(d.path());
    e.set("screensaver_timeout", Some("1"));
    write_extras(d.path(), &e).unwrap();

    let mut a = app_playing_in(d.path(), "Emerald");
    a.set_power(panel(d.path(), Duration::from_secs(180)).0);

    advance_secs(&mut a, 40.0);
    assert!(!a.screensaver_active());

    // User activity resets the timer
    a.apply(Action::GbaDown(Btn::Right));
    a.apply(Action::GbaUp(Btn::Right));

    // Advance 30s (total 70s, but only 30s since last input)
    advance_secs(&mut a, 30.0);
    assert!(!a.screensaver_active(), "activity should reset idle timer");
}
