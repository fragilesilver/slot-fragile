// Ported from nickclyde/slot-plus@29b8a81, crates/slot-cheevos/tests/notice.rs.

use slot_cheevos::notice::{self, from_event, Event, Game};
use slot_cheevos::Notice;

fn event(kind: u32) -> Event {
    Event {
        kind,
        title: "First Step".into(),
        description: "Set the flag".into(),
        points: 5,
        badge_url: "https://media/b.png".into(),
        message: "boom".into(),
    }
}

#[test]
fn a_triggered_achievement_is_an_unlock() {
    assert_eq!(
        from_event(event(notice::ACHIEVEMENT_TRIGGERED)),
        Some(Notice::Unlocked {
            title: "First Step".into(),
            description: "Set the flag".into(),
            points: 5,
            badge_url: "https://media/b.png".into(),
        })
    );
}

#[test]
fn completing_the_game_is_a_mastery() {
    let n = from_event(event(notice::GAME_COMPLETED)).unwrap();
    assert!(matches!(n, Notice::Mastered { ref title, .. } if title == "First Step"));
    assert!(n.is_unlock());
}

#[test]
fn connection_events_are_reported() {
    assert_eq!(
        from_event(event(notice::DISCONNECTED)),
        Some(Notice::Offline)
    );
    assert_eq!(from_event(event(notice::RECONNECTED)), Some(Notice::Online));
    assert_eq!(
        from_event(event(notice::SERVER_ERROR)),
        Some(Notice::ServerError {
            message: "boom".into()
        })
    );
}

#[test]
fn leaderboard_progress_and_challenge_events_are_ignored() {
    for kind in [2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 19] {
        assert_eq!(from_event(event(kind)), None, "{kind}");
    }
}

#[test]
fn only_achievement_and_game_notices_carry_a_badge() {
    let loaded = notice::loaded(Game {
        title: "G".into(),
        badge_url: "u".into(),
        console: 5,
        unlocked: 1,
        total: 2,
    });
    assert_eq!(loaded.badge_url(), Some("u"));
    assert!(!loaded.is_unlock());
    assert_eq!(Notice::Offline.badge_url(), None);
    assert_eq!(
        Notice::Mastered {
            title: "G".into(),
            badge_url: String::new()
        }
        .badge_url(),
        None
    );
}
