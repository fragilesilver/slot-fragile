use std::fs;
use std::path::{Path, PathBuf};

use crate::atomic::atomic_write;

pub const CHEEVOS_CACHE_DIR: &str = "Config/cheevos_cache";
const PENDING_FILE: &str = "pending.txt";

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct PendingUnlock {
    pub game_id: u32,
    pub achievement_id: u32,
    pub earned_utc: i64,
    pub title: String,
}

fn pending_path(root: &Path) -> PathBuf {
    root.join(CHEEVOS_CACHE_DIR).join(PENDING_FILE)
}

pub fn read_pending_unlocks(root: &Path) -> Vec<PendingUnlock> {
    let Ok(text) = fs::read_to_string(pending_path(root)) else {
        return Vec::new();
    };
    text.lines()
        .filter_map(|line| {
            let parts: Vec<&str> = line.split('\t').collect();
            if parts.len() < 4 {
                return None;
            }
            Some(PendingUnlock {
                game_id: parts[0].parse().ok()?,
                achievement_id: parts[1].parse().ok()?,
                earned_utc: parts[2].parse().ok()?,
                title: parts[3].to_string(),
            })
        })
        .collect()
}

pub fn write_pending_unlocks(root: &Path, items: &[PendingUnlock]) -> std::io::Result<()> {
    let path = pending_path(root);
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    let mut out = String::new();
    for item in items {
        let clean = item.title.replace(['\t', '\n', '\r'], " ");
        out.push_str(&format!(
            "{}\t{}\t{}\t{clean}\n",
            item.game_id, item.achievement_id, item.earned_utc
        ));
    }
    atomic_write(&path, out.as_bytes())
}

pub fn queue_pending_unlock(root: &Path, item: PendingUnlock) -> std::io::Result<()> {
    let mut items = read_pending_unlocks(root);
    if !items
        .iter()
        .any(|u| u.game_id == item.game_id && u.achievement_id == item.achievement_id)
    {
        items.push(item);
        write_pending_unlocks(root, &items)?;
    }
    Ok(())
}

pub fn cache_game_blob(root: &Path, hash: &str, json_or_blob: &[u8]) -> std::io::Result<()> {
    let clean: String = hash
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '-')
        .collect();
    if clean.is_empty() {
        return Ok(());
    }
    let path = root.join(CHEEVOS_CACHE_DIR).join(format!("{clean}.cache"));
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    atomic_write(&path, json_or_blob)
}

pub fn read_game_blob(root: &Path, hash: &str) -> Option<Vec<u8>> {
    let clean: String = hash
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '-')
        .collect();
    if clean.is_empty() {
        return None;
    }
    fs::read(root.join(CHEEVOS_CACHE_DIR).join(format!("{clean}.cache"))).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn offline_unlocks_deduplicate_and_flush_cleanly() {
        let d = tempdir().unwrap();
        let u = PendingUnlock {
            game_id: 101,
            achievement_id: 9001,
            earned_utc: 1_700_000_000,
            title: "First Badge".into(),
        };
        queue_pending_unlock(d.path(), u.clone()).unwrap();
        queue_pending_unlock(d.path(), u).unwrap();
        let list = read_pending_unlocks(d.path());
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].title, "First Badge");
        write_pending_unlocks(d.path(), &[]).unwrap();
        assert!(read_pending_unlocks(d.path()).is_empty());
    }
}
