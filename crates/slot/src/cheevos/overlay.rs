// Ported from nickclyde/slot-plus@c218577, crates/slot/src/cheevos/overlay.rs.

use std::collections::VecDeque;
use std::time::Duration;

use slot_cheevos::badge::BADGE;
use slot_cheevos::Notice;
use slot_ui::{text, HUD_INK};

pub const FACE_W: u32 = 480;
pub const FACE_H: u32 = 84;
pub const SHOW: Duration = Duration::from_millis(4000);
pub const FADE: Duration = Duration::from_millis(250);

const PANEL: [u8; 4] = [0x14, 0x12, 0x10, 0xd8];
const PAD: u32 = 10;
const MARGIN: f32 = 16.0;
const MAX_FACES: usize = 8;

pub struct Face {
    pub rgba: Vec<u8>,
    pub unlock: bool,
}

#[derive(Default)]
pub struct Queue(VecDeque<Face>);

impl Queue {
    pub fn push(&mut self, face: Face) {
        if self.0.len() >= MAX_FACES {
            match self.0.iter().position(|f| !f.unlock) {
                Some(i) => {
                    self.0.remove(i);
                }
                None if !face.unlock => return,
                None => {}
            }
        }
        self.0.push_back(face);
    }

    pub fn pop(&mut self) -> Option<Face> {
        self.0.pop_front()
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

pub fn rect() -> (f32, f32, f32, f32) {
    (MARGIN, MARGIN, FACE_W as f32, FACE_H as f32)
}

pub fn alpha(shown_for: Duration) -> Option<f32> {
    if shown_for >= SHOW {
        return None;
    }
    let fade = FADE.as_secs_f32();
    let into = shown_for.as_secs_f32();
    let left = (SHOW - shown_for).as_secs_f32();
    Some((into / fade).min(left / fade).min(1.0))
}

pub fn lines(notice: &Notice) -> (String, String) {
    match notice {
        Notice::LoggedIn { user } => ("RetroAchievements".into(), format!("Signed in as {user}")),
        Notice::LoginFailed {
            needs_password: true,
            ..
        } => (
            "RetroAchievements login failed".into(),
            "Sign in again on Settings > Network".into(),
        ),
        Notice::LoginFailed { reason, .. } => {
            let hint = if reason.to_ascii_lowercase().contains("certificate") {
                "Check the clock".to_string()
            } else {
                reason.clone()
            };
            ("RetroAchievements login failed".into(), hint)
        }
        Notice::GameLoaded {
            title,
            unlocked,
            total,
            ..
        } => (
            title.clone(),
            if *total == 0 {
                "No achievements yet".into()
            } else {
                format!("{unlocked} of {total} achievements")
            },
        ),
        Notice::Unrecognised => (
            "Not recognised".into(),
            "RetroAchievements does not know this ROM".into(),
        ),
        Notice::Unlocked {
            title,
            description,
            points,
            ..
        } => (format!("{title} - {points} pts"), description.clone()),
        Notice::Mastered { title, .. } => ("Completed".into(), title.clone()),
        Notice::Offline => (
            "RetroAchievements offline".into(),
            "Unlocks will be sent once back online".into(),
        ),
        Notice::Online => (
            "RetroAchievements back online".into(),
            "Pending unlocks sent".into(),
        ),
        Notice::ServerError { message } => ("RetroAchievements error".into(), message.clone()),
    }
}

pub fn compose(notice: &Notice, badge: Option<&[u8]>) -> Face {
    let mut rgba: Vec<u8> = PANEL
        .iter()
        .copied()
        .cycle()
        .take((FACE_W * FACE_H * 4) as usize)
        .collect();
    let mut left = PAD + 6;
    if let Some(badge) = badge.filter(|b| b.len() == BADGE * BADGE * 4) {
        let b = BADGE as u32;
        blit(&mut rgba, badge, b, b, PAD, (FACE_H - b) / 2);
        left = PAD + b + PAD;
    }
    let width = FACE_W - left - PAD - 6;
    let (head, detail) = lines(notice);
    if let Some(font) = text::label_font() {
        let head = text::fit(font, &head, width as f32, 1, 20.0, 12.0);
        let detail = text::fit(font, &detail, width as f32, 2, 15.0, 10.0);
        let mut row = |layout: &text::Layout, y: u32, h: u32| {
            let mut ink = vec![0u8; (width * h * 4) as usize];
            text::draw_centred(&mut ink, width, h, layout, HUD_INK);
            blit(&mut rgba, &ink, width, h, left, y);
        };
        row(&head, 8, 30);
        row(&detail, 40, 38);
    }
    Face {
        rgba,
        unlock: notice.is_unlock(),
    }
}

fn blit(dst: &mut [u8], src: &[u8], w: u32, h: u32, x: u32, y: u32) {
    for row in 0..h {
        for col in 0..w {
            let s = &src[((row * w + col) * 4) as usize..][..4];
            let a = s[3] as u32;
            if a == 0 {
                continue;
            }
            let d = &mut dst[(((y + row) * FACE_W + x + col) * 4) as usize..][..4];
            let under = d[3] as u32 * (255 - a) / 255;
            let out = a + under;
            for c in 0..3 {
                d[c] = ((s[c] as u32 * a + d[c] as u32 * under + out / 2) / out) as u8;
            }
            d[3] = out as u8;
        }
    }
}
