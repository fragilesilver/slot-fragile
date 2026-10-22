// Ported from nickclyde/slot-plus@c218577, crates/slot/src/cheevos/off.rs.

use std::path::Path;

use slot_gfx::{Compositor, Draw};
use slot_retro::RetroCore;

pub struct Guard;

pub fn start(_root: &Path) {}

pub fn loaded(_rom: &Path, _resumed: bool) -> Guard {
    Guard
}

pub fn idle(_core: &dyn RetroCore) {}

pub fn frame(_core: &dyn RetroCore) {}

pub fn reset() {}

pub fn overlay(_compositor: &mut Compositor, _draws: &mut Vec<Draw>, _canvas_w: f32) {}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Status {
    SignedOut,
    SigningIn,
    SignedIn(String),
    Rejected(String),
}

pub fn status() -> Status {
    Status::SignedOut
}

pub fn set_enabled(_on: bool) {}

pub fn achievements() -> Vec<slot_cheevos_off::Achievement> {
    Vec::new()
}

pub fn sign_in_again(_root: &Path) {}

pub mod slot_cheevos_off {
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct Achievement {
        pub title: String,
        pub description: String,
        pub points: u32,
        pub unlocked: bool,
    }
}
