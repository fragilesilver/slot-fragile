// Fragile's guards on the menu, kept out of upstream's quick_menu tests.

use slot_ui::QuickRow;

fn pages() -> Vec<&'static [QuickRow]> {
    vec![
        &QuickRow::MAIN,
        &QuickRow::GENERAL,
        &QuickRow::CONTROL_PAGES,
        &QuickRow::SHELF_KEYS,
        &QuickRow::CONTROLS,
        &QuickRow::SHORTCUTS,
        &QuickRow::SCREEN,
        &QuickRow::SCREEN_GBA,
        &QuickRow::SCREEN_GBC,
        &QuickRow::SCREEN_GB,
        &QuickRow::SCREEN_NES,
        &QuickRow::SCREEN_SNES,
        &QuickRow::SCREEN_PS,
        &QuickRow::SCREEN_NDS,
        &QuickRow::GAME,
        &QuickRow::PLAY,
        &QuickRow::SETTINGS,
        &QuickRow::PLAY_SCREEN,
        &QuickRow::THIS_GAME,
        &QuickRow::NETWORK,
    ]
}

#[test]
fn every_row_is_on_exactly_one_page_and_that_is_its_own() {
    for row in QuickRow::ALL {
        let on: Vec<_> = pages().into_iter().filter(|p| p.contains(&row)).collect();
        assert_eq!(on.len(), 1, "{row:?} is on {} pages", on.len());
        assert_eq!(row.page(), on[0], "{row:?} names another page as its own");
    }
}

#[test]
fn this_game_opens_from_settings_and_backs_out_to_it() {
    assert!(QuickRow::PlayThisGame.opens());
    assert_eq!(QuickRow::PlayThisGame.child(), Some(QuickRow::THIS_GAME[0]));
    for row in QuickRow::THIS_GAME {
        assert_eq!(
            row.parent_in_game(),
            Some(QuickRow::PlayThisGame),
            "{row:?}"
        );
        assert!(!row.help().is_empty(), "{row:?}");
    }
}

#[test]
fn no_page_shows_more_rows_than_fit_between_the_corners_and_the_help_line() {
    for page in pages() {
        for row in page.iter() {
            let (_, shown) = slot_ui::quick_window(*row, &[]);
            assert!(
                shown <= slot_ui::QUICK_VISIBLE,
                "{row:?}'s page shows {shown} rows"
            );
        }
    }
}
