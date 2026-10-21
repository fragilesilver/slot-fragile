use std::path::{Path, PathBuf};

use slot_store::{BezelPick, Platform};

pub const BEZELS: &str = "System/bezels";
pub const GAME_BEZELS: &str = "Bezels";

pub fn list(root: &Path, platform: Platform) -> Vec<String> {
    let dir = root.join(GAME_BEZELS).join(platform.dir_name());
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

/// The marker on a bezel made for Integer Scale: `Classic 3x.png` goes with `Classic.png`.
const INTEGER_SUFFIX: &str = " 3x";

/// The bezel a `3x` file goes with, or `None` for an ordinary bezel.
pub fn integer_base(name: &str) -> Option<&str> {
    let stem = name
        .strip_suffix(".png")
        .or_else(|| name.strip_suffix(".PNG"))?;
    stem.strip_suffix(INTEGER_SUFFIX)
}

/// The bezel drawn while Integer Scale is on: the picked file's `3x` version when the card has
/// one, otherwise none, since a bezel cut for the band would cover the bigger picture.
pub fn for_integer(root: &Path, platform: Platform, pick: BezelPick) -> BezelPick {
    let BezelPick::File(name) = pick else {
        return BezelPick::None;
    };
    let Some(stem) = name
        .strip_suffix(".png")
        .or_else(|| name.strip_suffix(".PNG"))
    else {
        return BezelPick::None;
    };
    let three = format!("{stem}{INTEGER_SUFFIX}.png");
    match root
        .join(GAME_BEZELS)
        .join(platform.dir_name())
        .join(&three)
        .is_file()
    {
        true => BezelPick::File(three),
        false => BezelPick::None,
    }
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
