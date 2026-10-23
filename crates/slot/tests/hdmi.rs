mod common;

use common::{boot, tmp_root_with_carts};
use slot::app::App;
use slot::hdmi::{HdmiController, HdmiMode, SimHdmi, EXTRAS_KEY};
use slot_input::{Action, Btn};
use slot_store::read_extras;
use slot_ui::QuickRow;

fn press(a: &mut App, btn: Btn) {
    a.apply(Action::GbaDown(btn));
    a.apply(Action::GbaUp(btn));
}

fn go(a: &mut App, row: QuickRow) {
    for _ in 0..16 {
        if a.quick_menu() == Some(row) {
            return;
        }
        press(a, Btn::Down);
    }
    panic!("no {row:?} on the page, at {:?}", a.quick_menu());
}

fn general_page(a: &mut App) {
    a.apply(Action::QuickMenu);
    go(a, QuickRow::General);
    press(a, Btn::A);
    assert_eq!(a.quick_menu(), Some(QuickRow::Theme));
}

#[test]
fn hdmi_mode_defaults_to_auto() {
    let d = tmp_root_with_carts(&["Emerald", "Fusion"]);
    let a = boot(d.path());
    assert_eq!(a.hdmi_mode(), HdmiMode::Auto);
    assert_eq!(a.named_value(QuickRow::HdmiOutput).as_deref(), Some("Auto"));
}

#[test]
fn hdmi_mode_cycles_through_all_options_and_persists() {
    let d = tmp_root_with_carts(&["Emerald", "Fusion"]);
    let mut a = boot(d.path());
    general_page(&mut a);

    go(&mut a, QuickRow::HdmiOutput);
    assert_eq!(a.named_value(QuickRow::HdmiOutput).as_deref(), Some("Auto"));

    // Step right: Auto -> 720p
    press(&mut a, Btn::Right);
    assert_eq!(a.hdmi_mode(), HdmiMode::Hdmi720p);
    assert_eq!(a.named_value(QuickRow::HdmiOutput).as_deref(), Some("720p"));
    assert_eq!(read_extras(d.path()).get(EXTRAS_KEY), Some("720p"));

    // Step right: 720p -> 1080p
    press(&mut a, Btn::Right);
    assert_eq!(a.hdmi_mode(), HdmiMode::Hdmi1080p);
    assert_eq!(
        a.named_value(QuickRow::HdmiOutput).as_deref(),
        Some("1080p")
    );
    assert_eq!(read_extras(d.path()).get(EXTRAS_KEY), Some("1080p"));

    // Step right: 1080p -> Off
    press(&mut a, Btn::Right);
    assert_eq!(a.hdmi_mode(), HdmiMode::Off);
    assert_eq!(a.named_value(QuickRow::HdmiOutput).as_deref(), Some("Off"));
    assert_eq!(read_extras(d.path()).get(EXTRAS_KEY), Some("off"));

    // Step right: Off -> Auto
    press(&mut a, Btn::Right);
    assert_eq!(a.hdmi_mode(), HdmiMode::Auto);
    assert_eq!(a.named_value(QuickRow::HdmiOutput).as_deref(), Some("Auto"));

    // Step left: Auto -> Off
    press(&mut a, Btn::Left);
    assert_eq!(a.hdmi_mode(), HdmiMode::Off);
    assert_eq!(a.named_value(QuickRow::HdmiOutput).as_deref(), Some("Off"));

    // Step left: Off -> 1080p
    press(&mut a, Btn::Left);
    assert_eq!(a.hdmi_mode(), HdmiMode::Hdmi1080p);
    assert_eq!(
        a.named_value(QuickRow::HdmiOutput).as_deref(),
        Some("1080p")
    );
}

#[test]
fn sim_hdmi_controller_manages_state() {
    let mut sim = SimHdmi::default();
    assert!(!sim.is_connected());
    assert_eq!(sim.active_mode, None);

    sim.connected = true;
    assert!(sim.is_connected());

    sim.switch_on(HdmiMode::Hdmi720p).unwrap();
    assert_eq!(sim.active_mode, Some(HdmiMode::Hdmi720p));

    sim.switch_on(HdmiMode::Hdmi1080p).unwrap();
    assert_eq!(sim.active_mode, Some(HdmiMode::Hdmi1080p));

    sim.switch_off().unwrap();
    assert_eq!(sim.active_mode, None);

    sim.switch_on(HdmiMode::Auto).unwrap();
    sim.rescue().unwrap();
    assert_eq!(sim.active_mode, None);
}

#[test]
fn hdmi_rescue_script_resets_extras_state() {
    let d = tmp_root_with_carts(&["Emerald", "Fusion"]);
    let mut a = boot(d.path());
    a.set_hdmi_mode(HdmiMode::Hdmi720p);
    assert_eq!(read_extras(d.path()).get(EXTRAS_KEY), Some("720p"));

    let extras_path = d.path().join("Config/extras.state");
    assert!(extras_path.exists());

    // Place slothdmi.sh in System/slothdmi.sh
    let system_dir = d.path().join("System");
    std::fs::create_dir_all(&system_dir).unwrap();
    let script_source = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../card/System/slothdmi.sh"),
    )
    .unwrap();
    std::fs::write(system_dir.join("slothdmi.sh"), script_source).unwrap();

    let mut hdmi = slot::hdmi::DeviceHdmi::new(d.path());
    assert!(hdmi.rescue().is_ok());

    // Verify extras.state now has hdmi_mode=off
    let reloaded = read_extras(d.path());
    assert_eq!(reloaded.get(EXTRAS_KEY), Some("off"));
}
