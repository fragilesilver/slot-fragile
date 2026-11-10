use std::path::Path;

use crate::platform::Platform;

/// Returns a sanitized filename if `name` has leading or trailing spaces, or its stem
/// has trailing dots, or trailing dots on the filename. Returns None if name is already clean.
pub fn clean_filename(name: &str) -> Option<String> {
    let trimmed = name.trim().trim_end_matches('.').trim();
    if trimmed.is_empty() {
        return None;
    }
    let (stem, ext) = match trimmed.rfind('.') {
        Some(dot) if dot > 0 => (&trimmed[..dot], &trimmed[dot..]),
        _ => (trimmed, ""),
    };
    let clean_stem = stem.trim().trim_end_matches('.').trim();
    if clean_stem.is_empty() {
        return None;
    }
    let cleaned = format!("{clean_stem}{ext}");
    if cleaned != name {
        Some(cleaned)
    } else {
        None
    }
}

/// Sweeps the games folders on the card, renaming any ROM files with trailing dots or
/// spaces to prevent filesystem and save corruption. Also renames matching saves, states,
/// and labels if present.
pub fn sweep_names(root: &Path) -> std::io::Result<usize> {
    let mut swept = 0;
    for platform in Platform::ALL {
        let dir = root.join("Games").join(platform.dir_name());
        let entries = match std::fs::read_dir(&dir) {
            Ok(e) => e,
            Err(_) => continue,
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            let file_name = match path.file_name().and_then(|n| n.to_str()) {
                Some(n) => n,
                None => continue,
            };
            if let Some(cleaned) = clean_filename(file_name) {
                let target = dir.join(&cleaned);
                if rename_via_temp(&path, &target).is_ok() {
                    swept += 1;
                    sweep_sidecars(root, platform, file_name, &cleaned);
                }
            }
        }
    }
    Ok(swept)
}

fn sweep_sidecars(root: &Path, platform: Platform, old_name: &str, new_name: &str) {
    let old_stem = match old_name.rfind('.') {
        Some(dot) => &old_name[..dot],
        None => old_name,
    };
    let new_stem = match new_name.rfind('.') {
        Some(dot) => &new_name[..dot],
        None => new_name,
    };
    if old_stem == new_stem {
        return;
    }

    // Saves/<Platform>/<stem>.sav
    let old_save = root
        .join("Saves")
        .join(platform.dir_name())
        .join(format!("{old_stem}.sav"));
    let new_save = root
        .join("Saves")
        .join(platform.dir_name())
        .join(format!("{new_stem}.sav"));
    if old_save.exists() {
        let _ = rename_via_temp(&old_save, &new_save);
    }

    // Labels/<Platform>/<stem>.png and Labels/<stem>.png
    let old_label_plat = root
        .join("Labels")
        .join(platform.dir_name())
        .join(format!("{old_stem}.png"));
    let new_label_plat = root
        .join("Labels")
        .join(platform.dir_name())
        .join(format!("{new_stem}.png"));
    if old_label_plat.exists() {
        let _ = rename_via_temp(&old_label_plat, &new_label_plat);
    }

    let old_label = root.join("Labels").join(format!("{old_stem}.png"));
    let new_label = root.join("Labels").join(format!("{new_stem}.png"));
    if old_label.exists() {
        let _ = rename_via_temp(&old_label, &new_label);
    }
}

fn rename_via_temp(from: &Path, to: &Path) -> std::io::Result<()> {
    if from == to {
        return Ok(());
    }
    let parent = from
        .parent()
        .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::Other, "no parent"))?;
    let temp = parent.join(format!(".sweep_{}", std::process::id()));
    std::fs::rename(from, &temp)?;
    std::fs::rename(&temp, to)
}
