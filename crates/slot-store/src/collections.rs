// Ported from nickclyde/slot-plus@02c5890, crates/slot-store/src/collections.rs. Writing lists
// (add, remove, a new list) is Fragile's own, at the end.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::find::fold;
use crate::platform::Platform;
use crate::scan::{is_hidden, sort_key, Cart};

pub const LISTS_DIR: &str = "Config/Lists";

pub type CartKey = (Platform, String);

#[derive(Default)]
pub struct Index {
    exact: HashMap<(Platform, String), String>,
    bare: HashMap<String, Vec<CartKey>>,
    loose: HashMap<String, Vec<CartKey>>,
}

impl Index {
    pub fn new<'a>(carts: impl IntoIterator<Item = &'a Cart>) -> Self {
        let mut index = Index::default();
        for cart in carts {
            let key = (cart.platform, cart.stem.clone());
            let lower = cart.stem.to_lowercase();
            index
                .exact
                .entry((cart.platform, lower.clone()))
                .or_insert_with(|| cart.stem.clone());
            index.bare.entry(lower).or_default().push(key.clone());
            index.loose.entry(loose(&cart.stem)).or_default().push(key);
        }
        for keys in index.bare.values_mut().chain(index.loose.values_mut()) {
            keys.sort_by_key(|(p, _)| *p as u8);
            keys.dedup();
        }
        index
    }

    pub fn resolve(&self, line: &str) -> Option<CartKey> {
        let line = line.trim().trim_start_matches('\u{feff}').trim();
        if line.is_empty() || line.starts_with('#') {
            return None;
        }
        let line = line.replace('\\', "/");
        let parts: Vec<&str> = line.split('/').filter(|p| !p.is_empty()).collect();
        let (&name, before) = parts.split_last()?;
        let platform = before.last().and_then(|dir| platform_of(dir));
        let name = strip_rom_extension(name, platform);
        let lower = name.to_lowercase();
        if let Some(platform) = platform {
            if let Some(stem) = self.exact.get(&(platform, lower.clone())) {
                return Some((platform, stem.clone()));
            }
        } else if let Some(first) = self.bare.get(&lower).and_then(|keys| keys.first()) {
            return Some(first.clone());
        }
        let mut near = self
            .loose
            .get(&loose(name))?
            .iter()
            .filter(|(p, _)| platform.is_none_or(|want| *p == want));
        match (near.next(), near.next()) {
            (Some(only), None) => Some(only.clone()),
            _ => None,
        }
    }
}

fn platform_of(dir: &str) -> Option<Platform> {
    Platform::ALL
        .into_iter()
        .find(|p| p.dir_name().eq_ignore_ascii_case(dir))
}

fn strip_rom_extension(name: &str, platform: Option<Platform>) -> &str {
    let Some((stem, ext)) = name.rsplit_once('.') else {
        return name;
    };
    let known = |p: Platform| p.extensions().iter().any(|k| k.eq_ignore_ascii_case(ext));
    let rom = match platform {
        Some(p) => known(p),
        None => Platform::ALL.into_iter().any(known),
    };
    if rom && !stem.is_empty() {
        stem
    } else {
        name
    }
}

fn loose(name: &str) -> String {
    fold(name).joined
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    pub raw: String,
    pub hit: Option<CartKey>,
}

pub fn parse_entries(text: &str, index: &Index) -> Vec<Entry> {
    text.lines()
        .map(|line| line.trim().trim_start_matches('\u{feff}').trim())
        .filter(|line| !line.is_empty())
        .map(|line| Entry {
            raw: line.to_string(),
            hit: index.resolve(line),
        })
        .collect()
}

pub fn read_entries(path: &Path, index: &Index) -> Vec<Entry> {
    match std::fs::read_to_string(path) {
        Ok(text) => parse_entries(&text, index),
        Err(_) => Vec::new(),
    }
}

pub fn hits(entries: &[Entry]) -> Vec<CartKey> {
    let mut out: Vec<CartKey> = Vec::new();
    for key in entries.iter().filter_map(|e| e.hit.as_ref()) {
        if !out.contains(key) {
            out.push(key.clone());
        }
    }
    out
}

pub struct List {
    pub name: String,
    pub carts: Vec<CartKey>,
}

pub fn read_lists(root: &Path, index: &Index) -> Vec<List> {
    let Ok(dir) = std::fs::read_dir(root.join(LISTS_DIR)) else {
        return Vec::new();
    };
    let mut files: Vec<PathBuf> = dir
        .filter_map(|e| Some(e.ok()?.path()))
        .filter(|p| !is_hidden(p) && p.is_file())
        .filter(|p| {
            p.extension()
                .and_then(|e| e.to_str())
                .is_some_and(|e| e.eq_ignore_ascii_case("txt"))
        })
        .collect();
    files.sort_by_key(|p| sort_key(&file_stem(p)));
    files
        .into_iter()
        .map(|path| List {
            name: file_stem(&path),
            carts: hits(&read_entries(&path, index)),
        })
        .collect()
}

fn file_stem(path: &Path) -> String {
    path.file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default()
}

// --- extras ---

/// The line a list is given for a cart: its console's folder and its name, which finds it
/// again exactly.
pub fn list_line(key: &CartKey) -> String {
    format!("{}/{}", key.0.dir_name(), key.1)
}

pub fn list_path(root: &Path, name: &str) -> PathBuf {
    root.join(LISTS_DIR).join(format!("{name}.txt"))
}

/// A list's name as a file name: trimmed, without the characters a FAT card refuses.
pub fn list_name(typed: &str) -> Option<String> {
    let name: String = typed
        .chars()
        .filter(|c| !matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|'))
        .collect();
    let name = name.trim().trim_start_matches('.').trim();
    (!name.is_empty()).then(|| name.to_string())
}

/// Whether the list holds this cart, by any line that finds it.
pub fn in_list(root: &Path, name: &str, key: &CartKey, index: &Index) -> bool {
    hits(&read_entries(&list_path(root, name), index)).contains(key)
}

/// Puts the cart at the end of the list, making the list if there is none.
pub fn add_to_list(root: &Path, name: &str, key: &CartKey) -> std::io::Result<()> {
    let path = list_path(root, name);
    std::fs::create_dir_all(root.join(LISTS_DIR))?;
    let mut text = std::fs::read_to_string(&path).unwrap_or_default();
    if !text.is_empty() && !text.ends_with('\n') {
        text.push('\n');
    }
    text.push_str(&list_line(key));
    text.push('\n');
    crate::atomic_write(&path, text.as_bytes())
}

/// Takes every line that finds the cart out of the list, and keeps the rest as written.
pub fn remove_from_list(
    root: &Path,
    name: &str,
    key: &CartKey,
    index: &Index,
) -> std::io::Result<()> {
    let path = list_path(root, name);
    let text = std::fs::read_to_string(&path)?;
    let kept: String = text
        .lines()
        .filter(|line| index.resolve(line).as_ref() != Some(key))
        .map(|line| format!("{line}\n"))
        .collect();
    crate::atomic_write(&path, kept.as_bytes())
}
