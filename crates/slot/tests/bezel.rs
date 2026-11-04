use slot::app::App;
use slot::bezel::{pick, BEZELS};
use slot_store::{
    bezel_for_scale, bezel_scale, read_extras, set_scaling, write_extras, write_slot_state,
    BezelPick, Platform, Scale, Scaling, SlotState,
};

fn card(files: &[&str]) -> tempfile::TempDir {
    let d = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(d.path().join(BEZELS)).unwrap();
    for f in files {
        std::fs::write(d.path().join(BEZELS).join(f), b"").unwrap();
    }
    d
}

fn picked(d: &tempfile::TempDir, panel: (u32, u32)) -> Option<String> {
    pick(d.path(), panel).map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
}

#[test]
fn the_panel_takes_the_bezel_named_for_its_size() {
    let d = card(&["640x480.png", "720x720.png", "1024x768.png"]);
    assert_eq!(picked(&d, (640, 480)).as_deref(), Some("640x480.png"));
    assert_eq!(picked(&d, (720, 720)).as_deref(), Some("720x720.png"));
}

#[test]
fn a_panel_with_no_bezel_of_its_size_takes_the_nearest_of_its_shape() {
    let d = card(&["640x480.png", "1024x768.png", "720x720.png"]);
    assert_eq!(picked(&d, (1280, 960)).as_deref(), Some("1024x768.png"));
    assert_eq!(picked(&d, (720, 480)), None);
}

#[test]
fn files_that_are_not_sized_pngs_are_ignored() {
    let d = card(&["notes.txt", "sp.png", "640x480.jpg", "._640x480.png"]);
    assert_eq!(picked(&d, (640, 480)), None);
    assert_eq!(picked(&tempfile::tempdir().unwrap(), (640, 480)), None);
}

#[test]
fn a_bezel_suffix_names_the_scale_it_is_drawn_for() {
    assert_eq!(bezel_scale("Shell_spscreen.png"), Some(Scale::SpFrame));
    assert_eq!(bezel_scale("Shell_integer.png"), Some(Scale::Integer));
    assert_eq!(bezel_scale("Shell_FIT.PNG"), Some(Scale::Fit));
    assert_eq!(bezel_scale("Shell.png"), None);
    assert_eq!(bezel_scale("fitted.png"), None);
}

#[test]
fn a_suffixed_bezel_has_a_name_for_each_scale() {
    assert_eq!(
        bezel_for_scale("Shell_fit.png", Scale::Integer).as_deref(),
        Some("Shell_integer.png")
    );
    assert_eq!(
        bezel_for_scale("Shell_spscreen.png", Scale::Fit).as_deref(),
        Some("Shell_fit.png")
    );
    assert_eq!(bezel_for_scale("Shell.png", Scale::Fit), None);
}

#[test]
fn a_suffixed_bezel_is_labelled_without_its_suffix() {
    assert_eq!(BezelPick::File("Shell_fit.png".into()).label(), "Shell");
    assert_eq!(
        BezelPick::File("Shell_spscreen.png".into()).label(),
        "Shell"
    );
    assert_eq!(BezelPick::File("Wood.png".into()).label(), "Wood");
}

fn bezels_in(
    platform: Platform,
    panel: (u32, u32),
    scale: Scale,
    picked: &str,
    files: &[&str],
) -> (Vec<String>, BezelPick) {
    let pick = BezelPick::File(picked.into());
    let d = tempfile::tempdir().unwrap();
    let dir = d.path().join("Bezels").join(platform.dir_name());
    std::fs::create_dir_all(&dir).unwrap();
    for f in files {
        std::fs::write(dir.join(f), b"").unwrap();
    }
    let mut e = read_extras(d.path());
    match scale {
        Scale::SpFrame => e.set(&format!("scale_{}", platform.key()), Some("spscreen")),
        Scale::Integer => set_scaling(&mut e, platform, Scaling::Integer),
        Scale::Fit => set_scaling(&mut e, platform, Scaling::Fit),
        Scale::FullScreen => set_scaling(&mut e, platform, Scaling::Fill),
    }
    write_extras(d.path(), &e).unwrap();
    let mut state = SlotState {
        cart: Some("Game".into()),
        cart_platform: Some(platform),
        clock_set: true,
        ..SlotState::default()
    };
    match platform {
        Platform::Gba => state.bezel_gba = pick,
        _ => state.bezel_gb = pick,
    }
    write_slot_state(d.path(), &state).unwrap();
    let mut a = App::boot(d.path());
    a.set_panel(panel);
    let mut shown: Vec<String> = a
        .bezel_files(platform)
        .into_iter()
        .filter(|name| {
            slot_store::bezel_scale(name).is_none_or(|s| s == a.resolved_scale(platform))
        })
        .collect();
    shown.sort();
    (shown, a.resolved_bezel(platform))
}

const ALL: [&str; 4] = [
    "Shell_fit.png",
    "Shell_integer.png",
    "Shell_spscreen.png",
    "Wood.png",
];

#[test]
fn the_bezel_list_holds_plain_bezels_and_those_for_the_scale() {
    for (platform, panel, scale, want) in [
        (
            Platform::Gb,
            (640, 480),
            Scale::SpFrame,
            ["Shell_spscreen.png", "Wood.png"],
        ),
        (
            Platform::Gb,
            (640, 480),
            Scale::Integer,
            ["Shell_integer.png", "Wood.png"],
        ),
        (
            Platform::Gb,
            (640, 480),
            Scale::Fit,
            ["Shell_fit.png", "Wood.png"],
        ),
        (
            Platform::Gb,
            (720, 480),
            Scale::Integer,
            ["Shell_integer.png", "Wood.png"],
        ),
        (
            Platform::Gba,
            (640, 480),
            Scale::Fit,
            ["Shell_fit.png", "Wood.png"],
        ),
    ] {
        let (shown, _) = bezels_in(platform, panel, scale, "Wood.png", &ALL);
        assert_eq!(shown, want, "{platform:?} {panel:?} {scale:?}");
    }
}

#[test]
fn a_plain_bezel_stays_picked_in_every_scale() {
    for scale in [Scale::SpFrame, Scale::Integer, Scale::Fit] {
        let (_, pick) = bezels_in(Platform::Gb, (640, 480), scale, "Wood.png", &ALL);
        assert_eq!(pick, BezelPick::File("Wood.png".into()), "{scale:?}");
    }
}

#[test]
fn a_suffixed_bezel_follows_the_scale_to_its_matching_file() {
    let (_, pick) = bezels_in(
        Platform::Gb,
        (640, 480),
        Scale::Integer,
        "Shell_fit.png",
        &ALL,
    );
    assert_eq!(pick, BezelPick::File("Shell_integer.png".into()));
    let only_fit = ["Shell_fit.png", "Wood.png"];
    let (_, pick) = bezels_in(
        Platform::Gb,
        (640, 480),
        Scale::Integer,
        "Shell_fit.png",
        &only_fit,
    );
    assert_eq!(pick, BezelPick::None);
}

#[test]
fn a_bezel_solid_below_lifts_the_game_to_the_top() {
    let d = card(&[]);
    let gba_dir = d.path().join("Bezels/GBA");
    std::fs::create_dir_all(&gba_dir).unwrap();
    std::fs::write(gba_dir.join("Solid.png"), b"").unwrap();
    let mut a = App::boot(d.path());
    a.set_panel((640, 480));
    let solid_chin: Vec<u8> = (0..480).map(|y| if y < 427 { 0 } else { 255 }).collect();
    a.set_bezel_column(solid_chin);
    a.set_game_bezel(Some(slot_gfx::TexId::from_raw(1)));
    a.set_platform_bezel(Platform::Gba, BezelPick::File("Solid.png".into()));
    assert!(a.picture_on_top());

    let open: Vec<u8> = vec![0u8; 480];
    a.set_bezel_column(open);
    assert!(!a.picture_on_top());
}
