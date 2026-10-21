// Ported from nickclyde/slot-plus@29b8a81, crates/slot-cheevos/src/notice.rs.

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Notice {
    LoggedIn {
        user: String,
    },
    LoginFailed {
        reason: String,
        needs_password: bool,
    },
    GameLoaded {
        title: String,
        unlocked: u32,
        total: u32,
        badge_url: String,
    },
    Unrecognised,
    Unlocked {
        title: String,
        description: String,
        points: u32,
        badge_url: String,
    },
    Mastered {
        title: String,
        badge_url: String,
    },
    Offline,
    Online,
    ServerError {
        message: String,
    },
}

impl Notice {
    pub fn badge_url(&self) -> Option<&str> {
        match self {
            Notice::GameLoaded { badge_url, .. }
            | Notice::Unlocked { badge_url, .. }
            | Notice::Mastered { badge_url, .. } => {
                Some(badge_url.as_str()).filter(|u| !u.is_empty())
            }
            _ => None,
        }
    }

    pub fn is_unlock(&self) -> bool {
        matches!(self, Notice::Unlocked { .. } | Notice::Mastered { .. })
    }
}

pub const ACHIEVEMENT_TRIGGERED: u32 = 1;
pub const GAME_COMPLETED: u32 = 15;
pub const SERVER_ERROR: u32 = 16;
pub const DISCONNECTED: u32 = 17;
pub const RECONNECTED: u32 = 18;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Event {
    pub kind: u32,
    pub title: String,
    pub description: String,
    pub points: u32,
    pub badge_url: String,
    pub message: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Game {
    pub title: String,
    pub badge_url: String,
    pub console: u32,
    pub unlocked: u32,
    pub total: u32,
}

pub fn from_event(event: Event) -> Option<Notice> {
    match event.kind {
        ACHIEVEMENT_TRIGGERED => Some(Notice::Unlocked {
            title: event.title,
            description: event.description,
            points: event.points,
            badge_url: event.badge_url,
        }),
        GAME_COMPLETED => Some(Notice::Mastered {
            title: event.title,
            badge_url: event.badge_url,
        }),
        SERVER_ERROR => Some(Notice::ServerError {
            message: event.message,
        }),
        DISCONNECTED => Some(Notice::Offline),
        RECONNECTED => Some(Notice::Online),
        _ => None,
    }
}

pub fn loaded(game: Game) -> Notice {
    Notice::GameLoaded {
        title: game.title,
        unlocked: game.unlocked,
        total: game.total,
        badge_url: game.badge_url,
    }
}
