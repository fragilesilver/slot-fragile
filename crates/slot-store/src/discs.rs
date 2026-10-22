//! Fragile: how the scanner finds games that come on discs (Phase 12). A disc game often sits in
//! a folder of its own, a .cue beside its .bin tracks, and one on several discs has an .m3u that
//! lists them; the shelf shows the .m3u and keeps the discs it lists behind it.

use std::collections::HashSet;
use std::path::{Component, Path, PathBuf};

use crate::platform::Platform;
use crate::scan::{is_hidden, Cart};

/// Adds the games in the folders one down from `dir`, then takes off the shelf every disc an
/// .m3u lists. Does nothing for a platform whose games are not on discs.
pub(crate) fn settle(
    platform: Platform,
    dir: &Path,
    carts: &mut Vec<Cart>,
    make: &dyn Fn(PathBuf) -> Option<Cart>,
) {
    if !platform.spec().discs {
        return;
    }
    for folder in entries(dir).into_iter().filter(|p| p.is_dir()) {
        for rom in entries(&folder) {
            if platform.accepts(&rom) && rom.is_file() {
                carts.extend(make(rom));
            }
        }
    }
    let listed: HashSet<String> = carts
        .iter()
        .filter(|c| c.platform == platform && is_m3u(&c.rom))
        .flat_map(|c| members(&c.rom))
        .map(|p| key(&p))
        .collect();
    carts.retain(|c| c.platform != platform || !listed.contains(&key(&c.rom)));
}

fn entries(dir: &Path) -> Vec<PathBuf> {
    let Ok(read) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    read.filter_map(|e| Some(e.ok()?.path()))
        .filter(|p| !is_hidden(p))
        .collect()
}

fn is_m3u(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case("m3u"))
}

/// The discs an .m3u lists, each line a path from the .m3u's own folder; blank lines and
/// comments are skipped.
pub fn members(m3u: &Path) -> Vec<PathBuf> {
    let Ok(text) = std::fs::read_to_string(m3u) else {
        return Vec::new();
    };
    let base = m3u.parent().unwrap_or(Path::new(""));
    text.lines()
        .map(|l| l.trim().trim_start_matches('\u{feff}'))
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(|l| base.join(l.replace('\\', "/")))
        .collect()
}

/// A path as the card's filesystem tells files apart: `.` and `..` worked out, case ignored.
fn key(path: &Path) -> String {
    let mut parts: Vec<String> = Vec::new();
    for c in path.components() {
        match c {
            Component::CurDir => {}
            Component::ParentDir => {
                parts.pop();
            }
            other => parts.push(other.as_os_str().to_string_lossy().to_lowercase()),
        }
    }
    parts.join("/")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_path_matches_however_it_is_spelt() {
        assert_eq!(
            key(Path::new("/g/PS/Game/./Disc 1.CUE")),
            key(Path::new("/g/PS/Game/Sub/../disc 1.cue"))
        );
    }

    #[test]
    fn an_m3u_lists_its_discs_from_its_own_folder() {
        let d = tempfile::tempdir().unwrap();
        let m3u = d.path().join("Game.m3u");
        std::fs::write(
            &m3u,
            "\u{feff}# discs\nGame (Disc 1).chd\r\n\nsub\\Game (Disc 2).chd\n",
        )
        .unwrap();
        assert_eq!(
            members(&m3u),
            vec![
                d.path().join("Game (Disc 1).chd"),
                d.path().join("sub/Game (Disc 2).chd")
            ]
        );
    }
}
