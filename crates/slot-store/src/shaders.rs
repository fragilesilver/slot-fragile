//! `Shaders/`: single-pass RetroArch `.glsl` files, chosen per console on the Screen page as a
//! Card Shader that draws instead of the built-in one. Listed by file name, sorted without
//! regard to case. Ideas from markbruno/slot-plus, crates/slot-store/src/shaders.rs.

use std::path::{Path, PathBuf};

pub const SHADERS_DIR: &str = "Shaders";

fn is_glsl(path: &Path) -> bool {
    path.extension()
        .is_some_and(|x| x.eq_ignore_ascii_case("glsl"))
}

/// Every shader on the card, by name, without its extension.
pub fn list_shaders(root: &Path) -> Vec<String> {
    let mut found: Vec<String> = std::fs::read_dir(root.join(SHADERS_DIR))
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| is_glsl(p) && !crate::is_hidden(p))
        .filter_map(|p| Some(p.file_stem()?.to_str()?.to_string()))
        .collect();
    found.sort_by(|a, b| {
        a.to_lowercase()
            .cmp(&b.to_lowercase())
            .then_with(|| a.cmp(b))
    });
    found
}

/// Where a named shader is. The extension is found again rather than assumed, because the
/// card may spell it `.GLSL`.
pub fn shader_path(root: &Path, name: &str) -> Option<PathBuf> {
    std::fs::read_dir(root.join(SHADERS_DIR))
        .ok()?
        .flatten()
        .map(|e| e.path())
        .find(|p| is_glsl(p) && p.file_stem().and_then(|s| s.to_str()) == Some(name))
}
