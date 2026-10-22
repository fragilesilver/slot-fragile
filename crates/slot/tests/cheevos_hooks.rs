// The one-line hooks RetroAchievements hangs on, so a merge from upstream that drops one fails
// here rather than on the device. After nickclyde/slot-plus@c218577,
// crates/slot/tests/cheevos_hooks.rs.

const HOOKS: &[(&str, &str, &str)] = &[
    ("lib.rs", include_str!("../src/lib.rs"), "pub mod cheevos;"),
    (
        "frontend.rs",
        include_str!("../src/frontend.rs"),
        "crate::cheevos::start(platform.root());",
    ),
    (
        "frontend.rs",
        include_str!("../src/frontend.rs"),
        "crate::cheevos::overlay(compositor, &mut self.draws, canvas_w);",
    ),
    (
        "emu.rs",
        include_str!("../src/emu.rs"),
        "let _cheevos = crate::cheevos::loaded(&rom, resume.is_some());",
    ),
    (
        "emu.rs",
        include_str!("../src/emu.rs"),
        "crate::cheevos::idle(core.as_ref());",
    ),
    (
        "emu.rs",
        include_str!("../src/emu.rs"),
        "crate::cheevos::frame(core.as_ref());",
    ),
    (
        "slot-retro lib.rs",
        include_str!("../../slot-retro/src/lib.rs"),
        "pub use memory::MemoryDescriptor;",
    ),
    (
        "slot-retro core.rs",
        include_str!("../../slot-retro/src/core.rs"),
        "fn memory_map(&self) -> &[crate::memory::MemoryDescriptor] {",
    ),
    (
        "slot-retro libretro.rs",
        include_str!("../../slot-retro/src/libretro.rs"),
        "crate::memory::SET_MEMORY_MAPS | crate::memory::SET_MEMORY_MAPS_PLAIN =>",
    ),
];

#[test]
fn every_retroachievements_hook_survived_the_merge() {
    for (file, text, hook) in HOOKS {
        assert_eq!(text.matches(hook).count(), 1, "{file}: {hook}");
    }
}

#[test]
fn state_loads_and_the_cable_restore_reset_achievements() {
    let emu = include_str!("../src/emu.rs");
    assert_eq!(emu.matches("crate::cheevos::reset();").count(), 2);
}

#[test]
fn the_feature_is_on_for_host_and_device_builds() {
    let manifest = include_str!("../Cargo.toml");
    assert!(manifest.contains(r#"default = ["host", "cheevos"]"#));
    assert!(manifest.contains(r#"device = ["slot-gfx/device", "cheevos"]"#));
}
