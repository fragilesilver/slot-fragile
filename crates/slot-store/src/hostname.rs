use std::fs;
use std::path::{Path, PathBuf};

use crate::atomic::atomic_write;

pub const BASEOS_CONF: &str = "baseos.conf";
pub const DEFAULT_HOSTNAME: &str = "slot";

/// Matches BaseOS's `valid_hostname` rules (`BaseOS/overlay/usr/sbin/baseos-config`).
pub fn valid_hostname(name: &str) -> bool {
    let s = name.trim();
    if s.is_empty() || s.len() > 63 {
        return false;
    }
    if s.starts_with('-') || s.ends_with('-') {
        return false;
    }
    if s.chars().all(|c| c.is_ascii_digit()) {
        return false;
    }
    s.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
}

fn conf_path(root: &Path) -> PathBuf {
    root.join(BASEOS_CONF)
}

pub fn read_hostname(root: &Path) -> String {
    let path = conf_path(root);
    let fallback = Path::new("/run/baseos.conf");
    let text = fs::read_to_string(&path)
        .or_else(|_| fs::read_to_string(fallback))
        .unwrap_or_default();
    for line in text.lines() {
        if let Some((k, v)) = line.trim().split_once('=') {
            if k.trim() == "hostname" && valid_hostname(v) {
                return v.trim().to_string();
            }
        }
    }
    DEFAULT_HOSTNAME.to_string()
}

/// Updates only the `hostname=` line in `baseos.conf` (creating it if absent) without touching
/// other BaseOS keys (`mdns`, `headphone_pop_fix`, etc.).
pub fn write_hostname(root: &Path, hostname: &str) -> Result<(), String> {
    let name = hostname.trim();
    if !valid_hostname(name) {
        return Err("Hostname must be 1–63 letters, digits or hyphens.".into());
    }
    let path = conf_path(root);
    let existing = fs::read_to_string(&path).unwrap_or_default();
    let mut lines = Vec::new();
    let mut replaced = false;
    for line in existing.lines() {
        if let Some((k, _)) = line.trim().split_once('=') {
            if k.trim() == "hostname" {
                lines.push(format!("hostname={name}"));
                replaced = true;
                continue;
            }
        }
        lines.push(line.to_string());
    }
    if !replaced {
        lines.insert(0, format!("hostname={name}"));
    }
    let mut out = lines.join("\n");
    out.push('\n');
    atomic_write(&path, out.as_bytes()).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn validates_and_preserves_other_baseos_conf_keys() {
        assert!(valid_hostname("rg35xx-pro"));
        assert!(!valid_hostname("bad name"));
        assert!(!valid_hostname("-leading"));
        assert!(!valid_hostname("12345"));

        let d = tempdir().unwrap();
        fs::write(
            d.path().join(BASEOS_CONF),
            "hostname=slot\nmdns=1\nheadphone_pop_fix=1\n",
        )
        .unwrap();
        write_hostname(d.path(), "fragile-pro").unwrap();
        let text = fs::read_to_string(d.path().join(BASEOS_CONF)).unwrap();
        assert!(text.contains("hostname=fragile-pro\n"));
        assert!(text.contains("mdns=1\n"));
        assert!(text.contains("headphone_pop_fix=1\n"));
        assert_eq!(read_hostname(d.path()), "fragile-pro");
    }
}
