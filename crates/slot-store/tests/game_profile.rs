// Ported from suprsomething/slot.advance@bdf47c3, crates/slot-store/tests/game_profile.rs.

use slot_store::{
    GameProfile, GameProfiles, Platform, ProfileField, Shader, SlotState, GAME_PROFILES_FILE,
};
use tempfile::tempdir;

#[test]
fn a_game_with_no_profile_follows_every_default() {
    let defaults = SlotState {
        shader_gba: Shader::Grid,
        colour_correction_gba: true,
        ff_speed: 4,
        ff_sound: true,
        ..Default::default()
    };
    let r = GameProfiles::default()
        .get(Platform::Gba, "Mother 3")
        .resolve(Platform::Gba, &defaults);
    assert_eq!(r.shader, Shader::Grid);
    assert!(r.colour);
    assert_eq!(r.ff_speed, 4);
    assert!(r.ff_sound);
}

#[test]
fn a_game_overrides_only_what_it_changed() {
    let mut p = GameProfiles::default();
    assert!(p.set(Platform::Gba, "Mother 3", ProfileField::FfSpeed(3)));
    let defaults = SlotState {
        shader_gba: Shader::Dot,
        ..Default::default()
    };
    let r = p
        .get(Platform::Gba, "Mother 3")
        .resolve(Platform::Gba, &defaults);
    assert_eq!(r.ff_speed, 3);
    assert_eq!(
        r.shader,
        Shader::Dot,
        "an unchanged setting did not follow the default"
    );
}

#[test]
fn one_games_profile_never_reaches_another() {
    let mut p = GameProfiles::default();
    p.set(
        Platform::Gba,
        "Mother 3",
        ProfileField::Shader(Shader::Lcd3x),
    );
    p.set(Platform::Gba, "Mother 3", ProfileField::Colour(true));
    assert_eq!(p.get(Platform::Gba, "Golden Sun"), GameProfile::default());
    assert_eq!(
        p.get(Platform::Gb, "Mother 3"),
        GameProfile::default(),
        "a Game Boy cart with the same name took the GBA game's settings"
    );
}

#[test]
fn profiles_round_trip_through_the_card() {
    let d = tempdir().unwrap();
    let mut p = GameProfiles::default();
    p.set(
        Platform::Gba,
        "Mother 3 (Japan)",
        ProfileField::Shader(Shader::Lcd3x),
    );
    p.set(
        Platform::Gba,
        "Mother 3 (Japan)",
        ProfileField::Colour(false),
    );
    p.set(Platform::Gba, "Mother 3 (Japan)", ProfileField::FfSpeed(2));
    p.set(
        Platform::Gba,
        "Mother 3 (Japan)",
        ProfileField::FfSound(true),
    );
    p.set(
        Platform::Gba,
        "Pokemon Emerald v1.0.1",
        ProfileField::FfSpeed(6),
    );
    p.write(d.path()).unwrap();
    let text = std::fs::read_to_string(d.path().join(GAME_PROFILES_FILE)).unwrap();
    assert!(
        text.contains("GBA/Mother 3 (Japan).shader = lcd3x"),
        "{text}"
    );
    assert!(text.contains("GBA/Pokemon Emerald v1.0.1.ff = 6"), "{text}");
    assert_eq!(GameProfiles::read(d.path()), p);
}

#[test]
fn hand_edits_and_junk_cost_only_their_own_line() {
    let p = GameProfiles::parse(
        "# a comment\n\
         GBA/Mother 3.shader = lcd3x\n\
         GBA/Mother 3.shader = pixelperfect\n\
         GBA/Mother 3.ff = 5\n\
         GBA/Mother 3.ffsound = maybe\n\
         GBA/Mother 3.colour = on\n\
         GBA/Mother 3.volume = 9\n\
         N64/Mario.ff = 3\n\
         GBA/.ff = 3\n\
         no equals sign here\n\
         GBA/Golden Sun.shader = simpletex\n",
    );
    let m = p.get(Platform::Gba, "Mother 3");
    assert_eq!(
        m.shader,
        Some(Shader::Lcd3x),
        "a later bad value wiped a good one"
    );
    assert_eq!(m.ff_speed, None, "5x is not a speed");
    assert_eq!(m.ff_sound, None);
    assert_eq!(m.colour, Some(true));
    assert_eq!(
        p.get(Platform::Gba, "Golden Sun").shader,
        None,
        "a Game Boy-only shader was accepted for a GBA game"
    );
    assert_eq!(p.len(), 1);
}

#[test]
fn a_name_the_file_cannot_hold_is_not_remembered() {
    let mut p = GameProfiles::default();
    assert!(!p.set(Platform::Gba, "A = B", ProfileField::FfSpeed(3)));
    assert!(!p.set(Platform::Gba, " Padded", ProfileField::FfSpeed(3)));
    assert!(p.is_empty());
}

#[test]
fn the_file_lives_in_config_so_updates_keep_it() {
    assert!(GAME_PROFILES_FILE.starts_with("Config/"));
}

#[test]
fn each_platform_falls_back_to_its_own_defaults() {
    let defaults = SlotState {
        shader_gb: Shader::Lcd3x,
        shader_gbc: Shader::Simpletex,
        colour_correction_gb: false,
        colour_correction_gbc: true,
        ..Default::default()
    };
    let none = GameProfile::default();
    let gb = none.resolve(Platform::Gb, &defaults);
    let gbc = none.resolve(Platform::Gbc, &defaults);
    assert_eq!((gb.shader, gb.colour), (Shader::Lcd3x, false));
    assert_eq!((gbc.shader, gbc.colour), (Shader::Simpletex, true));
}

#[test]
fn clearing_a_game_puts_it_back_on_the_defaults() {
    let mut p = GameProfiles::default();
    p.set(Platform::Gba, "Mother 3", ProfileField::FfSpeed(3));
    p.set(Platform::Gba, "Golden Sun", ProfileField::FfSpeed(6));
    assert!(p.clear(Platform::Gba, "Mother 3"));
    assert!(
        !p.clear(Platform::Gba, "Mother 3"),
        "a second clear found settings"
    );
    assert_eq!(p.get(Platform::Gba, "Mother 3"), GameProfile::default());
    assert_eq!(p.get(Platform::Gba, "Golden Sun").ff_speed, Some(6));
}

/// Fragile: a game's label style is kept, and a disc refuses one it cannot take.
#[test]
fn a_label_style_round_trips_and_a_disc_has_no_cart_label() {
    use slot_store::LabelStyle;
    let mut p = GameProfiles::default();
    assert!(p.set(
        Platform::Gba,
        "Fusion",
        ProfileField::Label(LabelStyle::Logo)
    ));
    let back = GameProfiles::parse(&p.to_text());
    assert_eq!(
        back.get(Platform::Gba, "Fusion").label,
        Some(LabelStyle::Logo)
    );
    let disc = GameProfiles::parse("PS/Tactics.label = label\nPS/Vagrant.label = logo\n");
    assert_eq!(disc.get(Platform::PlayStation, "Tactics").label, None);
    assert_eq!(
        disc.get(Platform::PlayStation, "Vagrant").label,
        Some(LabelStyle::Logo)
    );
}
