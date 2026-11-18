use std::fs;
use std::path::Path;

pub const ROMM_FILE: &str = "Config/romm.txt";

#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct RommConfig {
    pub url: String,
    pub username: String,
    pub password: String,
    pub token: String,
}

impl RommConfig {
    pub fn configured(&self) -> bool {
        self.url.starts_with("http://") || self.url.starts_with("https://")
    }
}

pub fn read_romm(root: &Path) -> Option<RommConfig> {
    let text = fs::read_to_string(root.join(ROMM_FILE)).ok()?;
    let mut cfg = RommConfig::default();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((k, v)) = line.split_once('=') else {
            continue;
        };
        let v = v.trim().to_string();
        match k.trim() {
            "url" | "host" => cfg.url = v.trim_end_matches('/').to_string(),
            "username" | "user" => cfg.username = v,
            "password" | "pass" => cfg.password = v,
            "token" => cfg.token = v,
            _ => {}
        }
    }
    cfg.configured().then_some(cfg)
}
