use std::collections::HashMap;
use std::fmt;
use std::path::{Path, PathBuf};

use crate::gba::header_title_code;
use crate::platform::Platform;

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Cart {
    pub platform: Platform,
    pub stem: String,
    pub rom: PathBuf,
    pub label: Option<PathBuf>,
    pub title: String,
    pub code: String,
    pub shell: Option<crate::ShellChoice>,
}

impl Cart {
    pub fn read_header(&mut self) {
        match self.platform.spec().header {
            crate::Header::Gba => {
                if let Some((title, code)) = header_title_code(&self.rom) {
                    self.title = title;
                    self.code = code;
                }
            }
            crate::Header::Gb => {
                if let Some(title) = crate::gb::title(&self.rom) {
                    self.title = title;
                }
            }
            crate::Header::None => {}
        }
    }
}

#[derive(Debug)]
pub enum StoreError {
    Io(std::io::Error),
}

impl fmt::Display for StoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StoreError::Io(e) => write!(f, "io: {e}"),
        }
    }
}

impl std::error::Error for StoreError {}

impl From<std::io::Error> for StoreError {
    fn from(e: std::io::Error) -> Self {
        StoreError::Io(e)
    }
}

pub fn scan(root: &Path) -> Result<Vec<Cart>, StoreError> {
    let read = |file: &str| std::fs::read_to_string(root.join(file)).unwrap_or_default();
    let shells = crate::cart_shell::layered(
        &read(crate::CART_SHELL_FILE),
        &read(crate::LABELS_SHELL_FILE),
    );
    let mut carts = Vec::new();
    for platform in Platform::ALL {
        let dir = root.join("Games").join(platform.dir_name());
        let entries = match std::fs::read_dir(&dir) {
            Ok(d) => d,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
            Err(e) => {
                eprintln!("slot: scan: {}: {e}", dir.display());
                continue;
            }
        };
        let labels_dir = root.join("Labels").join(platform.dir_name());
        let labels_dir_lower = root.join("Labels").join(platform.dir_name().to_lowercase());
        let cartart_media_dir = dir.join("media/cartart");
        let carts_media_dir = dir.join("media/carts");
        let cartart_dir = dir.join("cartart");
        let cartridges_dir = dir.join("media/cartridges");

        let labels = listing(&labels_dir);
        let labels_lower = listing(&labels_dir_lower);
        let cartart_media = listing(&cartart_media_dir);
        let carts_media = listing(&carts_media_dir);
        let cartart = listing(&cartart_dir);
        let cartridges = listing(&cartridges_dir);

        // Fragile: Game Art's label, where the player has none of their own.
        let scraped_dir = root.join(crate::SCRAPED_LABELS).join(platform.dir_name());
        let scraped = listing(&scraped_dir);
        let make = |rom: PathBuf| {
            let stem = rom.file_stem().and_then(|s| s.to_str())?.to_string();
            let png = format!("{stem}.png").to_lowercase();
            let jpg = format!("{stem}.jpg").to_lowercase();
            let label = labels
                .get(&png)
                .or_else(|| labels.get(&jpg))
                .map(|name| labels_dir.join(name))
                .or_else(|| {
                    labels_lower
                        .get(&png)
                        .or_else(|| labels_lower.get(&jpg))
                        .map(|name| labels_dir_lower.join(name))
                })
                .or_else(|| {
                    cartart_media
                        .get(&png)
                        .or_else(|| cartart_media.get(&jpg))
                        .map(|name| cartart_media_dir.join(name))
                })
                .or_else(|| {
                    carts_media
                        .get(&png)
                        .or_else(|| carts_media.get(&jpg))
                        .map(|name| carts_media_dir.join(name))
                })
                .or_else(|| {
                    cartart
                        .get(&png)
                        .or_else(|| cartart.get(&jpg))
                        .map(|name| cartart_dir.join(name))
                })
                .or_else(|| {
                    cartridges
                        .get(&png)
                        .or_else(|| cartridges.get(&jpg))
                        .map(|name| cartridges_dir.join(name))
                })
                .or_else(|| {
                    scraped
                        .get(&png)
                        .or_else(|| scraped.get(&jpg))
                        .map(|name| scraped_dir.join(name))
                });
            Some(Cart {
                platform,
                shell: shells.get(&crate::cart_shell::key(&stem)).copied(),
                stem,
                title: String::new(),
                code: String::new(),
                label,
                rom,
            })
        };
        for entry in entries {
            let Ok(entry) = entry else {
                continue;
            };
            let rom = entry.path();
            if is_hidden(&rom) || !platform.accepts(&rom) || !is_file(&entry) {
                continue;
            }
            carts.extend(make(rom));
        }
        // Fragile: games on discs in folders of their own, and the discs an .m3u lists kept
        // behind it.
        crate::discs::settle(platform, &dir, &mut carts, &make);
    }
    carts.sort_by(|a, b| {
        (a.platform as u8, sort_key(&a.stem)).cmp(&(b.platform as u8, sort_key(&b.stem)))
    });
    Ok(carts)
}

pub fn listing(dir: &Path) -> HashMap<String, String> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return HashMap::new();
    };
    entries
        .filter_map(|e| e.ok()?.file_name().into_string().ok())
        .filter(|name| !name.starts_with('.'))
        .map(|name| (name.to_lowercase(), name))
        .collect()
}

fn is_file(entry: &std::fs::DirEntry) -> bool {
    match entry.file_type() {
        Ok(t) if t.is_symlink() => entry.path().is_file(),
        Ok(t) => t.is_file(),
        Err(_) => false,
    }
}

pub fn sort_key(stem: &str) -> (u8, String) {
    (group_of(stem), stem.to_uppercase())
}

fn group_of(stem: &str) -> u8 {
    match stem.chars().find(|c| !c.is_whitespace()) {
        Some(c) if c.is_ascii_digit() => 0,
        Some(c) if c.is_alphabetic() => 1,
        _ => 2,
    }
}

pub fn initial(stem: &str) -> char {
    match group_of(stem) {
        1 => stem
            .chars()
            .find(|c| !c.is_whitespace())
            .and_then(|c| c.to_uppercase().next())
            .unwrap_or('#'),
        _ => '#',
    }
}

pub fn is_hidden(p: &Path) -> bool {
    p.file_name()
        .and_then(|n| n.to_str())
        .is_some_and(|n| n.starts_with('.'))
}
