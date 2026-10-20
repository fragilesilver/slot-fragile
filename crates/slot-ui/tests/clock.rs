use slot_ui::{clock_label, date_time_text_as, hhmm_as, Polaroids};

#[test]
fn recent_states_read_relative_and_old_ones_read_absolute() {
    let now = "2026-08-09_14-36-05";
    assert_eq!(
        Polaroids::relative_time("2026-08-09_14-32-05", now),
        "4 min ago"
    );
    assert_eq!(
        Polaroids::relative_time("2026-08-09_03-00-00", now),
        "11 hr ago"
    );
    assert_eq!(
        Polaroids::relative_time("2026-08-08_20-00-00", now),
        "2026-08-08 20:00"
    );
    assert_eq!(
        Polaroids::relative_time("2025-01-02_09-05-00", now),
        "2025-01-02 09:05"
    );
}

#[test]
fn the_clock_is_24_hour_and_shows_no_seconds() {
    assert_eq!(
        clock_label("2026-08-09_21-07-00"),
        "21:07",
        "not 24 hour, or showing seconds"
    );
}

#[test]
fn the_twelve_hour_clock_turns_over_at_noon_and_midnight() {
    let at = |h: i64, m: i64| h * 3600 + m * 60;
    for (secs, want) in [
        (at(0, 5), "12:05 AM"),
        (at(9, 30), "9:30 AM"),
        (at(12, 0), "12:00 PM"),
        (at(13, 7), "1:07 PM"),
        (at(23, 59), "11:59 PM"),
    ] {
        assert_eq!(hhmm_as(secs, true), want);
    }
    assert_eq!(hhmm_as(at(13, 7), false), "13:07");
}

#[test]
fn the_menus_date_and_old_photos_follow_the_clock() {
    // 2026-08-08 20:00 UTC.
    let secs = 1_786_219_200;
    assert_eq!(date_time_text_as(secs, true), "AUG 8 8:00 PM");
    assert_eq!(date_time_text_as(secs, false), "AUG 8 20:00");
    let now = "2026-08-09_14-36-05";
    assert_eq!(
        Polaroids::relative_time_as("2026-08-08_20-00-00", now, true),
        "2026-08-08 8:00 PM"
    );
    assert_eq!(
        Polaroids::relative_time_as("2026-08-09_14-32-05", now, true),
        "4 min ago"
    );
}
