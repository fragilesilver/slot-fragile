mod common;

use std::fs;
use std::time::Duration;

use slot::app::App;
use slot::core::ds_console_options;
use slot::persist::{back_up, backups, retire_resume_older_than_sav, sav_path};
use slot_input::{Action, Btn};
use slot_store::{
    core_for_platform, read_play_log, read_slot_state, write_slot_state, Core, KidMode, Platform,
    StateRing, Zone,
};
use slot_ui::{QuickRow, QuickValue};

fn setup_multi_system_root() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let r = dir.path();
    for sub in slot::root::DIRS {
        fs::create_dir_all(r.join(sub)).unwrap();
    }
    for (dir_name, file_name) in [
        ("GBA", "Metroid Fusion.gba"),
        ("GB", "Tetris.gb"),
        ("GBC", "Pokemon Crystal.gbc"),
        ("NES", "Super Mario Bros.nes"),
        ("SNES", "Super Metroid.sfc"),
        ("PS1", "Ridge Racer.chd"),
        ("NDS", "Mario Kart DS.nds"),
        ("PokeDaisy", "PokeDaisy FireRed.gba"),
        ("PokeDaisy", "PokeDaisy Crystal.gbc"),
    ] {
        let p = r.join("Games").join(dir_name);
        fs::create_dir_all(&p).unwrap();
        fs::write(p.join(file_name), b"romdata").unwrap();
    }
    dir
}

#[test]
fn shelf_life_defaults_on_and_unplayed_unsaved_cart_is_wrapped() {
    let d = common::tmp_root_with_carts(&["Emerald", "Sapphire"]);
    let a = common::boot(d.path());
    assert_eq!(a.quick_value(QuickRow::ShrinkWrap), Some(QuickValue::On));
    assert_eq!(a.quick_value(QuickRow::Dust), Some(QuickValue::On));
    assert_eq!(a.quick_value(QuickRow::Wear), Some(QuickValue::On));
    assert_eq!(
        a.quick_value(QuickRow::CleanResets),
        Some(QuickValue::CleanBoth)
    );
    let cart = a.carts().find(|c| c.stem == "Emerald").unwrap();
    let look = a.face_look(cart);
    assert!(look.wrap, "unplayed unsaved cart starts shrink-wrapped");
    assert_eq!(look.dust, 0);
    assert_eq!(look.wear, 0);

    fs::create_dir_all(d.path().join("Saves/GBA")).unwrap();
    fs::write(d.path().join("Saves/GBA/Emerald.sav"), b"save").unwrap();
    assert!(
        !a.face_look(cart).wrap,
        "a cart with a save on disk never renders wrapped"
    );
}

#[test]
fn clean_all_carts_unwraps_every_sealed_cart() {
    let d = common::tmp_root_with_carts(&["Emerald", "Sapphire"]);
    let mut a = common::boot(d.path());
    assert!(a.carts().all(|c| a.face_look(c).wrap));
    a.clean_all_carts();
    assert!(a.carts().all(|c| !a.face_look(c).wrap));
    let log = read_play_log(d.path());
    assert!(log.get(Platform::Gba, "Emerald").unwrap().unwrapped());
    assert!(log.get(Platform::Gba, "Sapphire").unwrap().unwrapped());
}

#[test]
fn pokedaisy_cores_share_one_shelf_and_use_hardware_save_folders() {
    let d = setup_multi_system_root();
    let a = common::boot(d.path());
    assert_eq!(Platform::PdGba.dir_name(), "PokeDaisy");
    assert_eq!(Platform::PdGbc.dir_name(), "PokeDaisy");
    assert_eq!(Platform::PdGb.dir_name(), "PokeDaisy");
    assert_eq!(Platform::PdGba.save_dir_name(), "GBA");
    assert_eq!(Platform::PdGbc.save_dir_name(), "GBC");
    assert_eq!(Platform::PdGb.save_dir_name(), "GB");
    assert_eq!(Platform::PdGba.shelf(), Platform::PdGba);
    assert_eq!(Platform::PdGbc.shelf(), Platform::PdGba);
    assert_eq!(Platform::PdGb.shelf(), Platform::PdGba);
    assert!(a
        .carts()
        .any(|c| c.platform == Platform::PdGba && c.stem == "PokeDaisy FireRed"));
    assert!(a
        .carts()
        .any(|c| c.platform == Platform::PdGbc && c.stem == "PokeDaisy Crystal"));
}

#[test]
fn start_opens_boards_across_all_systems_including_alone_boards() {
    let d = setup_multi_system_root();
    let r = d.path();
    let mut st = read_slot_state(r);
    st.clock_set = true;
    write_slot_state(r, &st).unwrap();
    let mut a = App::boot(r);

    let nes_core = core_for_platform(r, "Super Mario Bros", Platform::Nes);
    assert_eq!(nes_core, Core::Fceumm);

    // Navigate until NES cart is under cursor and open with Start.
    for _ in 0..20 {
        if a.selected_stem() == Some("Super Mario Bros") {
            break;
        }
        a.apply(Action::NextShelf);
    }
    assert_eq!(a.selected_stem(), Some("Super Mario Bros"));
    a.apply(Action::GbaDown(Btn::Start));
    assert!(
        a.core_picker().is_some(),
        "Start opens NES board view even with a single core"
    );
}

#[test]
fn ds_console_options_pass_nickname_and_zone_offset() {
    let bn = Zone::parse("Asia/Brunei").unwrap();
    let opts = ds_console_options("  FragilePlayer ", bn, 1_800_000_000);
    assert_eq!(
        opts,
        vec![
            ("dsperate_nickname", "FragilePla".to_string()),
            ("dsperate_utc_offset", "480".to_string()),
        ]
    );
}

#[test]
fn kid_allowance_refuses_launch_once_daily_limit_is_reached() {
    let d = common::tmp_root_with_carts(&["Emerald"]);
    let r = d.path();
    let mut st = read_slot_state(r);
    st.clock_set = true;
    st.kid_mode = KidMode::OneShelf;
    st.kid_allowance = 15;
    st.kid_day = 20_000;
    st.kid_used = 15 * 60;
    write_slot_state(r, &st).unwrap();

    let mut a = App::boot(r);
    assert_eq!(
        a.quick_value(QuickRow::KidAllowance),
        Some(QuickValue::Min15)
    );
    // Even if kid_day rolls on real wall clock, check kid_tick + kid_time_up explicitly.
    a.kid_tick(1.0);
    // Exhaust again in the current day bucket:
    for _ in 0..(15 * 60 + 5) {
        a.kid_tick(0.5);
        a.kid_tick(0.5);
    }
    assert!(a.kid_time_up());
}

#[test]
fn rolling_sav_backups_and_stale_resume_retirement() {
    let d = tempfile::tempdir().unwrap();
    let r = d.path();
    let stem = "Emerald";
    let platform = Platform::Gba;
    let core = Core::Mgba;

    back_up(r, platform, stem, b"v1", 10_000).unwrap();
    // Same hour bucket -> coalesces.
    back_up(r, platform, stem, b"v2", 10_100).unwrap();
    assert_eq!(backups(r, platform, stem).len(), 1);
    back_up(r, platform, stem, b"v3", 14_000).unwrap();
    back_up(r, platform, stem, b"v4", 18_000).unwrap();
    back_up(r, platform, stem, b"v5", 22_000).unwrap();
    assert_eq!(backups(r, platform, stem).len(), 3, "capped at 3 backups");

    // Stale resume older than battery save gets retired instead of clobbering .sav.
    let ring = StateRing::new(r, platform, core, stem);
    ring.write_resume(b"old-state").unwrap();
    std::thread::sleep(Duration::from_millis(6_000));
    let sav = sav_path(r, platform, stem);
    fs::create_dir_all(sav.parent().unwrap()).unwrap();
    fs::write(&sav, b"newer-sav").unwrap();

    let moved =
        retire_resume_older_than_sav(r, platform, core, stem, "2026-11-23_12-00-00").unwrap();
    assert!(moved.is_some(), "stale resume should be moved aside");
    assert!(ring.read_resume().unwrap().is_none());
}
