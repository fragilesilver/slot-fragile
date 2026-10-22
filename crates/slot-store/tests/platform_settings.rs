// Fragile keeps upstream's slot.state as it is: the Game Boy family's shader, colour correction
// and bezel stay in its own keys, read and written as upstream does.

use slot_store::{
    platform_settings, read_extras, read_slot_state, write_slot_state, BezelPick, ExtrasState,
    Platform, Shader,
};

const GOLDEN: &str = include_str!("golden/slot.state");

fn card_with(text: &str) -> tempfile::TempDir {
    let d = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(d.path().join("Config")).unwrap();
    std::fs::write(d.path().join("Config/slot.state"), text).unwrap();
    d
}

#[test]
fn an_upstream_slot_state_reads_and_writes_back_byte_for_byte() {
    let d = card_with(GOLDEN);
    let state = read_slot_state(d.path());
    write_slot_state(d.path(), &state).unwrap();
    let back = std::fs::read_to_string(d.path().join("Config/slot.state")).unwrap();
    assert_eq!(back, GOLDEN);
}

#[test]
fn the_game_boy_familys_settings_are_upstreams_own_keys() {
    let d = card_with(GOLDEN);
    let mut state = read_slot_state(d.path());
    let mut extras = ExtrasState::default();
    assert_eq!(
        platform_settings::shader(&state, &extras, Platform::Gb),
        state.shader_gb
    );
    assert_eq!(
        platform_settings::colour(&state, &extras, Platform::Gba),
        state.colour_correction_gba
    );
    platform_settings::set_shader(&mut state, &mut extras, Platform::Gbc, Shader::Lcd3x);
    platform_settings::set_bezel(&mut state, &mut extras, Platform::Gb, BezelPick::None);
    platform_settings::set_colour(&mut state, &mut extras, Platform::Gba, true);
    assert_eq!(state.shader_gbc, Shader::Lcd3x);
    assert_eq!(state.bezel_gb, BezelPick::None);
    assert!(state.colour_correction_gba);
    assert_eq!(
        extras,
        ExtrasState::default(),
        "a Game Boy setting reached extras.state"
    );
    assert!(read_extras(d.path()) == ExtrasState::default());
    for p in [Platform::Gba, Platform::Gb, Platform::Gbc] {
        assert!(!platform_settings::in_extras(p), "{p:?}");
    }
}
