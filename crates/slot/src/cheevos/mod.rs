// Ported from nickclyde/slot-plus@c218577, crates/slot/src/cheevos/mod.rs.

#[cfg(feature = "cheevos")]
pub mod boot;
#[cfg(feature = "cheevos")]
mod live;
#[cfg(not(feature = "cheevos"))]
mod off;
#[cfg(feature = "cheevos")]
pub mod overlay;

#[cfg(feature = "cheevos")]
pub use live::*;
#[cfg(not(feature = "cheevos"))]
pub use off::*;

#[derive(Clone, Debug)]
pub struct SetBadge {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

#[derive(Clone, Debug)]
pub struct SetEntry {
    pub title: String,
    pub description: String,
    pub points: u32,
    pub badge: Option<String>,
    pub earned: bool,
    pub earned_at: Option<u64>,
}

#[derive(Clone, Debug)]
pub struct CachedSet {
    pub game: String,
    pub entries: Vec<SetEntry>,
}

impl CachedSet {
    pub fn progress_label(&self) -> Option<String> {
        let total = self.entries.len();
        if total == 0 {
            return None;
        }
        let earned = self.entries.iter().filter(|e| e.earned).count();
        Some(if earned >= total {
            "Mastered".to_string()
        } else {
            format!("{earned} / {total}")
        })
    }
}
