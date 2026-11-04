use std::path::{Path, PathBuf};

use slot_store::{BezelPick, Platform};

pub const BEZELS: &str = "System/bezels";
pub const GAME_BEZELS: &str = "Bezels";

pub fn dir(root: &Path, platform: Platform) -> PathBuf {
    let base = root.join(GAME_BEZELS).join(platform.dir_name());
    if platform == Platform::PlayStation && !base.is_dir() {
        let alt = root.join(GAME_BEZELS).join("PS1");
        if alt.is_dir() {
            return alt;
        }
    }
    base
}

pub fn file_path(root: &Path, platform: Platform, name: &str) -> PathBuf {
    let d = dir(root, platform);
    let path = d.join(name);
    if !path.is_file() && platform == Platform::PlayStation {
        let alt = root.join(GAME_BEZELS).join("PS1").join(name);
        if alt.is_file() {
            return alt;
        }
    }
    path
}

fn list_dir(dir: &Path) -> Vec<String> {
    let mut files: Vec<String> = std::fs::read_dir(dir)
        .ok()
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| !slot_store::is_hidden(p))
        .filter(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("png")))
        .filter_map(|p| Some(p.file_name()?.to_str()?.to_owned()))
        .filter(|name| integer_base(name).is_none())
        .collect();
    files.sort_by(|a, b| {
        a.to_lowercase()
            .cmp(&b.to_lowercase())
            .then_with(|| a.cmp(b))
    });
    files
}

pub fn list(root: &Path, platform: Platform) -> Vec<String> {
    let dir = dir(root, platform);
    let mut files = list_dir(&dir);
    if platform == Platform::PlayStation && files.is_empty() {
        let alt = root.join(GAME_BEZELS).join("PS1");
        if alt.is_dir() {
            files = list_dir(&alt);
        }
    }
    files
}

/// The bezel a `{k}x` file goes with, or `None` for an ordinary bezel.
pub fn integer_base(name: &str) -> Option<&str> {
    let stem = name
        .strip_suffix(".png")
        .or_else(|| name.strip_suffix(".PNG"))?;
    let without_x = stem.strip_suffix('x').or_else(|| stem.strip_suffix('X'))?;
    let space_idx = without_x.rfind(' ')?;
    let k_str = &without_x[space_idx + 1..];
    if space_idx > 0 && !k_str.is_empty() && k_str.chars().all(|c| c.is_ascii_digit()) {
        Some(&stem[..space_idx])
    } else {
        None
    }
}

/// The bezel drawn while Integer Scale is on: the picked file's `{k}x` version when the card has
/// one, otherwise none, since a bezel cut for the band would cover the bigger picture.
/// Native systems have no fixed integer scale factor across all games/resolutions, so their
/// overlay is hidden when Integer Scale is on.
pub fn for_integer(root: &Path, platform: Platform, pick: BezelPick) -> BezelPick {
    if platform.spec().native {
        return BezelPick::None;
    }
    let BezelPick::File(name) = pick else {
        return BezelPick::None;
    };
    let Some(stem) = name
        .strip_suffix(".png")
        .or_else(|| name.strip_suffix(".PNG"))
    else {
        return BezelPick::None;
    };
    let pic = platform.picture();
    if pic.0 == 0 || pic.1 == 0 {
        return BezelPick::None;
    }
    let k = (640 / pic.0).min(480 / pic.1);
    let dir = dir(root, platform);
    let candidates = [
        format!("{stem} {k}x.png"),
        format!("{stem} {k}x.PNG"),
        format!("{stem} {k}X.png"),
        format!("{stem} {k}X.PNG"),
    ];
    for cand in candidates {
        if dir.join(&cand).is_file() {
            return BezelPick::File(cand);
        }
        if platform == Platform::PlayStation
            && root.join(GAME_BEZELS).join("PS1").join(&cand).is_file()
        {
            return BezelPick::File(cand);
        }
    }
    BezelPick::None
}

fn dims(path: &Path) -> Option<(u32, u32)> {
    if !path.extension()?.eq_ignore_ascii_case("png") {
        return None;
    }
    let (w, h) = path.file_stem()?.to_str()?.split_once('x')?;
    let (w, h) = (w.parse::<u32>().ok()?, h.parse::<u32>().ok()?);
    (w > 0 && h > 0).then_some((w, h))
}

pub fn pick(root: &Path, panel: (u32, u32)) -> Option<PathBuf> {
    let mut fits: Vec<((u32, u32), PathBuf)> = std::fs::read_dir(root.join(BEZELS))
        .ok()?
        .flatten()
        .map(|e| e.path())
        .filter(|p| !slot_store::is_hidden(p))
        .filter_map(|p| Some((dims(&p)?, p)))
        .filter(|((w, h), _)| {
            u64::from(*w) * u64::from(panel.1) == u64::from(*h) * u64::from(panel.0)
        })
        .collect();
    fits.sort_by_key(|((w, _), p)| (w.abs_diff(panel.0), p.clone()));
    fits.into_iter().next().map(|(_, p)| p)
}
