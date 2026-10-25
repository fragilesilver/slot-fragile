use slot_store::{parse_stamp, FF_SPEEDS};
use slot_ui::{
    date_time_text, quick_caret_face, quick_label_face, quick_value_face, ClockPicker, QuickRow,
    QuickValue, UndoFace, MENU_PAD,
};

fn inked(f: &UndoFace, x: u32, y: u32) -> bool {
    f.rgba[((y * f.w + x) * 4 + 3) as usize] > 128
}

fn ink_columns(f: &UndoFace) -> (u32, u32) {
    let cols: Vec<u32> = (0..f.w)
        .filter(|&x| (0..f.h).any(|y| inked(f, x, y)))
        .collect();
    (
        *cols.first().expect("an empty face"),
        *cols.last().expect("an empty face"),
    )
}

#[test]
fn the_type_sits_exactly_menu_pad_in_from_both_sides_of_its_face() {
    for f in [
        quick_label_face(QuickRow::FastForwardSound, true),
        quick_label_face(QuickRow::Rumble, true),
        quick_value_face("Off", false),
        quick_value_face(QuickValue::Speed6.text(), true),
        quick_value_face("SEP 15 16:35", true),
    ] {
        let (first, last) = ink_columns(&f);
        let right = f.w - 1 - MENU_PAD;
        assert!(
            (MENU_PAD..MENU_PAD + 4).contains(&first),
            "ink starts at {first} of {}",
            f.w
        );
        assert!(
            (right - 4..=right).contains(&last),
            "ink ends at {last} of {}",
            f.w
        );
    }
}

#[test]
fn a_long_label_is_set_as_large_as_a_short_one() {
    let tall = |f: &UndoFace| {
        (0..f.h)
            .filter(|&y| (0..f.w).any(|x| inked(f, x, y)))
            .count()
    };
    let long = tall(&quick_label_face(QuickRow::FastForwardSound, true));
    let short = tall(&quick_label_face(QuickRow::Rumble, true));
    assert!(long + 1 >= short, "{long} rows of ink against {short}");
}

#[test]
fn the_fast_forward_row_offers_off_and_the_four_ceilings_the_card_can_hold() {
    assert_eq!(FF_SPEEDS, [0, 2, 3, 4, 6]);
    assert_eq!(
        FF_SPEEDS.map(QuickValue::speed),
        [
            Some(QuickValue::Off),
            Some(QuickValue::Speed2),
            Some(QuickValue::Speed3),
            Some(QuickValue::Speed4),
            Some(QuickValue::Speed6),
        ]
    );
    for other in [1, 5, 7, 9, 16, 28, 255] {
        assert_eq!(
            QuickValue::speed(other),
            None,
            "{other}x is not a value the row has"
        );
    }
}

#[test]
fn the_rows_run_in_the_order_the_user_chose() {
    assert_eq!(
        QuickRow::MAIN.map(QuickRow::label),
        ["General", "Screen", "Gameplay", "Controls", "Network", "About"]
    );
    assert_eq!(
        QuickRow::GENERAL.map(QuickRow::label),
        [
            "Theme",
            "Wallpaper",
            "Wallpaper Tint",
            "Recents",
            "Game Titles",
            "Favorites",
            "Start Shelf",
            "Clock",
            "Battery Badge",
            "Lid Power Off",
            "Sleep Power Off",
            "Screensaver",
            "Screensaver Style",
            "Date & Time",
            "HDMI Output",
        ]
    );
    assert_eq!(
        QuickRow::CONTROL_PAGES.map(QuickRow::label),
        ["Shelf", "Shortcuts", "Button Guide", "In-Game"]
    );
    assert_eq!(
        QuickRow::SHELF_KEYS.map(QuickRow::label),
        [
            "Previous Shelf",
            "Next Shelf",
            "Favorite",
            "Search",
            "Add to List"
        ]
    );
    assert_eq!(
        QuickRow::SCREEN.map(QuickRow::label),
        [
            "Game Boy Advance",
            "Game Boy Color",
            "Game Boy",
            "NES",
            "Super Nintendo",
            "PlayStation",
            "Nintendo DS",
        ]
    );
    assert_eq!(
        QuickRow::SCREEN_GBA.map(QuickRow::label),
        [
            "Scaling",
            "Shader",
            "Card Shader",
            "Color Correction",
            "Bezel",
            "Motion Blur"
        ]
    );
    assert_eq!(
        QuickRow::SCREEN_GBC.map(QuickRow::label),
        [
            "Scaling",
            "Shader",
            "Card Shader",
            "Color Correction",
            "Bezel",
            "Motion Blur"
        ]
    );
    assert_eq!(
        QuickRow::SCREEN_GB.map(QuickRow::label),
        [
            "Scaling",
            "Shader",
            "Card Shader",
            "Color Correction",
            "Palettes",
            "Bezel",
            "Motion Blur"
        ]
    );
    assert_eq!(
        QuickRow::GAME.map(QuickRow::label),
        [
            "Fast Forward",
            "Fast Forward Sound",
            "Fast Forward Pitch",
            "Rewind",
            "Turbo Buttons",
            "Rumble",
            "Left Stick as D-Pad",
            "Auto Save on Eject"
        ]
    );
    let opens: Vec<QuickRow> = QuickRow::ALL.into_iter().filter(|r| r.opens()).collect();
    assert_eq!(
        opens,
        [
            QuickRow::Screen,
            QuickRow::DateTime,
            QuickRow::About,
            QuickRow::Game,
            QuickRow::Resume,
            QuickRow::UndoSave,
            QuickRow::UndoLoad,
            QuickRow::States,
            QuickRow::Screenshots,
            QuickRow::Settings,
            QuickRow::ExitGame,
            QuickRow::PlayScreen,
            QuickRow::PlayGameplay,
            QuickRow::ScreenGba,
            QuickRow::ScreenGbc,
            QuickRow::ScreenGb,
            QuickRow::General,
            QuickRow::PlayControls,
            QuickRow::Shortcuts,
            QuickRow::Controls,
            QuickRow::ControlsShelf,
            QuickRow::ControlsInGame,
            QuickRow::PlayThisGame,
            QuickRow::ControlsGuide,
            QuickRow::PlayGuide,
            QuickRow::PlayCheats,
            QuickRow::Network,
            QuickRow::WifiNetworks,
            QuickRow::CheevosAccount,
            QuickRow::PlayAchievements,
            QuickRow::ScreenNes,
            QuickRow::ScreenSnes,
            QuickRow::ScreenPs,
            QuickRow::ScreenNds,
            QuickRow::GameArt,
            QuickRow::BtDevices,
            QuickRow::SshPassword,
            QuickRow::ShaderSettings,
            QuickRow::PlayNetwork,
            QuickRow::PlayGeneral,
            QuickRow::Update,
        ]
    );
    for page in [
        &QuickRow::MAIN[..],
        &QuickRow::SCREEN[..],
        &QuickRow::GAME[..],
        &QuickRow::GENERAL[..],
        &QuickRow::PLAY[..],
        &QuickRow::SETTINGS[..],
        &QuickRow::PLAY_SCREEN[..],
        &QuickRow::SCREEN_GBA[..],
        &QuickRow::SCREEN_GBC[..],
        &QuickRow::SCREEN_GB[..],
        &QuickRow::SCREEN_NDS[..],
    ] {
        for &row in page {
            assert_eq!(row.page(), page, "{row:?}");
        }
    }
}

#[test]
fn a_hidden_row_is_stepped_over_and_takes_no_place() {
    let hidden = [QuickRow::WallpaperTint];
    assert_eq!(QuickRow::Wallpaper.down(&hidden), QuickRow::Recents);
    assert_eq!(QuickRow::Recents.up(&hidden), QuickRow::Wallpaper);
    assert_eq!(QuickRow::Theme.up(&hidden), QuickRow::HdmiOutput);
    assert_eq!(QuickRow::Recents.position(&hidden), 2);
    assert_eq!(QuickRow::Wallpaper.down(&[]), QuickRow::WallpaperTint);
    assert_eq!(QuickRow::Recents.position(&[]), 3);
    assert_eq!(
        slot_ui::quick_row_top(QuickRow::Recents, &hidden)
            - slot_ui::quick_row_top(QuickRow::Wallpaper, &hidden),
        slot_ui::QUICK_PITCH
    );
}

#[test]
fn the_values_read_as_the_menu_prints_them() {
    assert_eq!(
        QuickValue::ALL.map(QuickValue::text),
        [
            "2×",
            "3×",
            "4×",
            "6×",
            "On",
            "Off",
            "LCD3x",
            "Grid",
            "Dot",
            "Simpletex",
            "Dark",
            "Light",
            "Show",
            "Hide",
            "25%",
            "45%",
            "65%",
            "1 min",
            "3 min",
            "5 min",
            "10 min",
            "15 min",
            "30 min",
            "Last",
            "Recents",
            "Favorites",
            "GBA",
            "GB",
            "GBC",
            "24 Hour",
            "12 Hour",
            "Never",
            "NES",
            "SNES",
            "PS1",
            "NDS",
        ]
    );
    assert_eq!(QuickValue::flag(true), QuickValue::On);
    assert_eq!(QuickValue::flag(false), QuickValue::Off);
}

#[test]
fn the_date_and_time_read_as_a_month_a_day_and_the_carousels_24_hour_clock() {
    let at = |stamp: &str| date_time_text(parse_stamp(stamp).expect("a stamp"));
    assert_eq!(at("2026-09-15_16-35-00"), "SEP 15 16:35");
    assert_eq!(at("2027-01-05_04-07-59"), "JAN 5 04:07");
}

#[test]
fn a_picker_for_a_set_clock_starts_at_the_local_time_and_its_offset() {
    let utc = parse_stamp("2026-09-15_21-35-42").expect("a stamp");
    let p = ClockPicker::local(utc, -300);
    assert_eq!(p.offset_min(), -300);
    assert_eq!(
        p.secs(),
        utc - 42,
        "the picker does not show the minute it was opened in"
    );
    assert!(p.text().starts_with("2026-09-15 16:35"), "{}", p.text());
}

#[test]
fn a_value_is_grey_until_its_row_is_in_hand() {
    let inkiest = |lit| {
        let f = quick_value_face("4×", lit);
        f.rgba
            .chunks(4)
            .max_by_key(|p| p[3])
            .map(|p| [p[0], p[1], p[2]])
            .expect("an empty face")
    };
    assert_eq!(inkiest(true), [0xf6, 0xf4, 0xef]);
    assert_eq!(inkiest(false), [0x9a, 0x9a, 0xa4]);
}

#[test]
fn the_arrows_are_faces_the_height_of_a_value() {
    for right in [false, true] {
        let caret = quick_caret_face(right);
        assert!(
            caret.w > 0 && caret.rgba.chunks(4).any(|p| p[3] > 0),
            "an empty arrow"
        );
        assert_eq!(caret.h, quick_value_face("On", true).h);
    }
}

#[test]
fn the_two_carets_point_different_ways() {
    assert_ne!(
        slot_ui::quick_caret_face(false).rgba,
        slot_ui::quick_caret_face(true).rgba,
        "a caret glyph is missing from the symbols font"
    );
}

#[test]
fn the_pointer_is_a_crisp_pixel_arrow_in_the_menus_ink() {
    let p = slot_ui::quick_pointer_face();
    assert_eq!(p.w, 32);
    let opaque: Vec<&[u8]> = p.rgba.chunks(4).filter(|c| c[3] > 0).collect();
    assert!(!opaque.is_empty(), "an empty pointer");
    assert!(
        opaque.iter().all(|c| c == &[0xf6, 0xf4, 0xef, 0xff]),
        "the pointer is not one solid colour"
    );
    assert!(
        p.rgba.chunks(4).all(|c| c[3] == 0 || c[3] == 0xff),
        "the arrow was smoothed, not drawn in pixels"
    );
}

#[test]
fn a_label_is_grey_until_its_row_is_in_hand() {
    let inkiest = |lit| {
        let f = quick_label_face(QuickRow::Rumble, lit);
        f.rgba
            .chunks(4)
            .max_by_key(|p| p[3])
            .map(|p| [p[0], p[1], p[2]])
            .expect("an empty face")
    };
    assert_eq!(inkiest(true), [0xf6, 0xf4, 0xef]);
    assert_eq!(inkiest(false), [0x9a, 0x9a, 0xa4]);
}

#[test]
fn the_in_game_pages_hold_their_rows_in_order() {
    assert_eq!(
        QuickRow::PLAY,
        [
            QuickRow::Resume,
            QuickRow::UndoSave,
            QuickRow::UndoLoad,
            QuickRow::States,
            QuickRow::Screenshots,
            QuickRow::PlayDisc,
            QuickRow::PlayCheats,
            QuickRow::PlayAchievements,
            QuickRow::Settings,
            QuickRow::ExitGame
        ]
    );
    assert_eq!(
        QuickRow::SETTINGS.map(QuickRow::label),
        [
            "Screen",
            "Gameplay",
            "Controls",
            "This Game",
            "Network",
            "System",
            "Button Guide"
        ]
    );
    assert_eq!(
        QuickRow::CONTROLS.map(QuickRow::label),
        [
            "Up",
            "Down",
            "Left",
            "Right",
            "A Button",
            "B Button",
            "X Button",
            "Y Button",
            "Select",
            "Start",
            "L Button",
            "R Button",
            "L2 Button",
            "R2 Button",
            "Turbo A",
            "Turbo B"
        ]
    );
    assert_eq!(
        QuickRow::PLAY_SCREEN,
        [
            QuickRow::Scaling,
            QuickRow::PlayNdsLayout,
            QuickRow::Shader,
            QuickRow::CardShader,
            QuickRow::ShaderSettings,
            QuickRow::ColourCorrection,
            QuickRow::Palette,
            QuickRow::Bezel,
            QuickRow::MotionBlur,
        ]
    );
    for row in QuickRow::PLAY {
        assert_eq!(row.parent_in_game(), None, "{row:?}");
    }
}

#[test]
fn in_game_pages_climb_back_through_settings() {
    assert_eq!(QuickRow::Settings.child(), Some(QuickRow::PlayScreen));
    assert_eq!(QuickRow::PlayScreen.child(), Some(QuickRow::Scaling));
    assert_eq!(QuickRow::PlayGameplay.child(), Some(QuickRow::FastForward));
    assert_eq!(
        QuickRow::Shader.parent_in_game(),
        Some(QuickRow::PlayScreen)
    );
    assert_eq!(
        QuickRow::Scaling.parent_in_game(),
        Some(QuickRow::PlayScreen)
    );
    assert_eq!(
        QuickRow::WifiPower.parent_in_game(),
        Some(QuickRow::PlayNetwork)
    );
    assert_eq!(
        QuickRow::Theme.parent_in_game(),
        Some(QuickRow::PlayGeneral)
    );
    assert_eq!(
        QuickRow::PlayNetwork.parent_in_game(),
        Some(QuickRow::Settings)
    );
    assert_eq!(
        QuickRow::PlayGeneral.parent_in_game(),
        Some(QuickRow::Settings)
    );
    assert_eq!(
        QuickRow::PlayGameplay.parent_in_game(),
        Some(QuickRow::Settings)
    );
    assert_eq!(
        QuickRow::Rumble.parent_in_game(),
        Some(QuickRow::PlayGameplay)
    );
    assert_eq!(QuickRow::Rumble.parent(), Some(QuickRow::Game));
}

#[test]
fn in_game_rows_read_as_the_menu_shows_them() {
    let labels = [
        (QuickRow::Resume, "Resume"),
        (QuickRow::States, "States"),
        (QuickRow::Settings, "Settings"),
        (QuickRow::ExitGame, "Exit Game"),
        (QuickRow::PlayScreen, "Screen"),
        (QuickRow::PlayGameplay, "Gameplay"),
        (QuickRow::Shader, "Shader"),
        (QuickRow::ColourCorrection, "Color Correction"),
        (QuickRow::Palette, "Palette"),
    ];
    for (row, label) in labels {
        assert_eq!(row.label(), label);
    }
    for row in QuickRow::PLAY.into_iter().chain(QuickRow::SETTINGS) {
        assert!(row.centred(), "{row:?}");
    }
    for row in QuickRow::PLAY_SCREEN {
        if row != QuickRow::ShaderSettings {
            assert!(!row.opens(), "{row:?}");
        }
        assert!(!row.centred(), "{row:?}");
    }
    assert!(QuickRow::General.centred());
    assert!(!QuickRow::Rumble.centred());
}

#[test]
fn a_hidden_palette_and_bezel_leave_two_screen_rows() {
    let hidden = [
        QuickRow::Scaling,
        QuickRow::PlayNdsLayout,
        QuickRow::Palette,
        QuickRow::Bezel,
        QuickRow::CardShader,
        QuickRow::ShaderSettings,
        QuickRow::MotionBlur,
    ];
    assert_eq!(
        QuickRow::Shader.shown(&hidden),
        vec![QuickRow::Shader, QuickRow::ColourCorrection]
    );
    assert_eq!(QuickRow::ColourCorrection.down(&hidden), QuickRow::Shader);
}

#[test]
fn screen_opens_a_page_per_console_and_each_backs_out_to_its_console() {
    assert_eq!(QuickRow::Screen.child(), Some(QuickRow::ScreenGba));
    for (console, page) in [
        (QuickRow::ScreenGba, &QuickRow::SCREEN_GBA[..]),
        (QuickRow::ScreenGbc, &QuickRow::SCREEN_GBC[..]),
        (QuickRow::ScreenGb, &QuickRow::SCREEN_GB[..]),
    ] {
        assert_eq!(console.parent(), Some(QuickRow::Screen));
        assert_eq!(console.child(), Some(page[0]));
        for &row in page {
            assert_eq!(row.parent(), Some(console), "{row:?}");
        }
    }
    assert!(QuickRow::ScreenGb.centred());
    assert!(!QuickRow::GbShader.centred());
}

#[test]
fn screen_rows_in_game_sit_in_a_panel_at_the_bottom() {
    let hidden = [
        QuickRow::Scaling,
        QuickRow::PlayNdsLayout,
        QuickRow::Palette,
        QuickRow::CardShader,
        QuickRow::ShaderSettings,
        QuickRow::MotionBlur,
    ];
    let shader = slot_ui::quick_row_top(QuickRow::Shader, &hidden);
    let colour = slot_ui::quick_row_top(QuickRow::ColourCorrection, &hidden);
    assert_eq!(colour - shader, slot_ui::QUICK_PITCH);
    assert!(
        shader > slot_ui::OUT_H as f32 / 2.0,
        "the panel starts at {shader}"
    );
    assert!(slot_ui::quick_row_top(QuickRow::FastForward, &[]) < shader);
}

#[test]
fn the_wallpaper_tint_reads_as_how_much_it_darkens() {
    assert_eq!(
        [QuickValue::Low, QuickValue::Medium, QuickValue::High].map(QuickValue::text),
        ["25%", "45%", "65%"]
    );
}

#[test]
fn show_titles_sits_under_recents_on_the_general_page() {
    let at = |r: QuickRow| QuickRow::GENERAL.iter().position(|g| *g == r);
    assert_eq!(
        at(QuickRow::ShowTitles),
        at(QuickRow::Recents).map(|i| i + 1)
    );
    assert_eq!(QuickRow::ShowTitles.parent(), Some(QuickRow::General));
}

#[test]
fn every_row_in_all_matches_enum_discriminant() {
    for (i, &row) in QuickRow::ALL.iter().enumerate() {
        assert_eq!(
            row.index(),
            i,
            "row {row:?} at ALL[{i}] has discriminant {}",
            row.index()
        );
    }
}

#[test]
fn every_row_but_the_top_menus_explains_itself() {
    for row in QuickRow::ALL {
        let help = row.help();
        assert!(help.len() <= 52, "{row:?} help runs past one line: {help}");
        if row.page() != QuickRow::MAIN || row.opens() {
            assert!(!help.is_empty(), "{row:?} has no help line");
        }
    }
}

#[test]
fn help_face_fits_the_panel() {
    for row in QuickRow::ALL.into_iter().filter(|r| !r.help().is_empty()) {
        let face = slot_ui::quick_description_face(&[row.help()]);
        assert!(face.w <= 640, "{row:?} help is {} wide", face.w);
    }
}

#[test]
fn long_value_is_clipped_and_marquee_scrolled() {
    use slot_gfx::Draw;
    use slot_ui::{Canvas, FacePair, QuickMenu, QuickMenuFaces, TexId};

    let id = TexId::from_raw;
    let faces = QuickMenuFaces {
        labels: QuickRow::ALL
            .iter()
            .map(|_| [(id(10), 200, 30), (id(11), 200, 30)])
            .collect(),
        pointer: (id(20), 10, 10),
        values: vec![[(id(30), 50, 30), (id(31), 50, 30)]],
        carets: [(id(40), 12, 20), (id(41), 12, 20)],
        scroll: [(id(50), 10, 10), (id(51), 10, 10)],
        legend: [(id(60), 50); 8],
        legend_light: [(id(60), 50); 8],
        descriptions: vec![None; QuickRow::ALL.len()],
    };

    // A long named value (e.g. 350px wide) on a 640px wide canvas.
    // Label ends at (64 - 8) + 200 = 256. With safe gap 20, safe_left = 276.
    // Right is 640 - 32 = 608.
    // Value of 350px would collide (608 - 12 - 8 - 350 = 238 < 276).
    let named_pair: FacePair = [(id(100), 350, 30), (id(101), 350, 30)];
    let named: Vec<Option<FacePair>> = QuickRow::ALL
        .iter()
        .map(|r| {
            if *r == QuickRow::CardShader {
                Some(named_pair)
            } else {
                None
            }
        })
        .collect();

    // At time 0ms (pause at start):
    let mut out_t0 = Vec::new();
    let menu_t0 = QuickMenu {
        row: QuickRow::CardShader,
        hidden: &[],
        values: [None; QuickRow::ALL.len()],
        clock: None,
        wallpaper: None,
        faces: Some(&faces),
        ground: None,
        palette: None,
        gba_bezel: None,
        gb_bezel: None,
        gbc_bezel: None,
        bezel: None,
        bound: None,
        named: Some(&named),
        description: None,
        over_game: false,
        footer_legends: true,
        footer_top: 480.0,
        canvas: Canvas { w: 640.0, h: 480.0 },
        time_ms: 0,
    };
    menu_t0.draw(&mut out_t0);
    // Verify Draw::Part was emitted for the overflowing value:
    let part_t0 = out_t0
        .iter()
        .find_map(|d| match d {
            Draw::Part { x, w, uv, .. } => Some((*x, *w, *uv)),
            _ => None,
        })
        .expect("Draw::Part should be emitted for overflowing value");

    assert!(
        part_t0.0 >= 266.0,
        "Value x {} must be >= safe_left 266",
        part_t0.0
    );
    assert_eq!(
        part_t0.2[0], 0.0,
        "At time 0, uv scroll offset should be 0.0"
    );

    // At time 2000ms (scrolling):
    let mut out_t2 = Vec::new();
    let mut menu_t2 = menu_t0;
    menu_t2.time_ms = 2000;
    menu_t2.draw(&mut out_t2);

    let part_t2 = out_t2
        .iter()
        .find_map(|d| match d {
            Draw::Part { uv, .. } => Some(*uv),
            _ => None,
        })
        .expect("Draw::Part should be emitted");

    assert!(
        part_t2[0] > 0.0,
        "At time 2000ms, uv scroll offset should be > 0.0 (scrolling)"
    );
}
