//! `Shaders/`: single-pass RetroArch `.glsl` files, chosen per console on the Screen page as a
//! Card Shader that draws instead of the built-in one. Listed by file name, sorted without
//! regard to case. Ideas from markbruno/slot-plus, crates/slot-store/src/shaders.rs.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

pub const SHADERS_DIR: &str = "Shaders";
pub const SHADER_CONFIG_DIR: &str = "Config/Shaders";

/// Where a named shader's parameter configuration file is stored: `Config/Shaders/<name>.txt`.
pub fn shader_config_path(root: &Path, name: &str) -> PathBuf {
    root.join(SHADER_CONFIG_DIR).join(format!("{name}.txt"))
}

/// Reads saved parameter values for a shader from `Config/Shaders/<name>.txt`
/// (falling back to `Config/<name>.txt` if present).
pub fn load_shader_params(root: &Path, name: &str) -> HashMap<String, f32> {
    let mut out = HashMap::new();
    let primary = shader_config_path(root, name);
    let fallback = root.join("Config").join(format!("{name}.txt"));
    let path = if primary.exists() {
        primary
    } else if fallback.exists() {
        fallback
    } else {
        return out;
    };

    let Ok(content) = std::fs::read_to_string(path) else {
        return out;
    };

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
            continue;
        }
        if let Some((k, v)) = line.split_once('=') {
            let key = k.trim().to_string();
            if let Ok(val) = v.trim().parse::<f32>() {
                out.insert(key, val);
            }
        }
    }
    out
}

/// Saves parameter values for a shader atomically to `Config/Shaders/<name>.txt`.
pub fn save_shader_params(
    root: &Path,
    name: &str,
    params: &HashMap<String, f32>,
) -> std::io::Result<()> {
    let dir = root.join(SHADER_CONFIG_DIR);
    std::fs::create_dir_all(&dir)?;
    let mut lines = Vec::new();
    lines.push(format!("# Shader parameters for {name}"));
    let mut sorted: Vec<_> = params.iter().collect();
    sorted.sort_by_key(|(k, _)| (*k).clone());
    for (k, v) in sorted {
        lines.push(format!("{k} = {v}"));
    }
    lines.push(String::new());
    let path = shader_config_path(root, name);
    crate::atomic_write(&path, lines.join("\n").as_bytes())
}

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
