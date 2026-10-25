// Ported from nickclyde/slot-plus@29b8a81, crates/slot-cheevos/src/config.rs.

use std::path::Path;

pub const CHEEVOS_FILE: &str = "Config/retroachievements.txt";

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Secret {
    Password(String),
    Token(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Credentials {
    pub username: String,
    pub secret: Secret,
    pub host: Option<String>,
}

impl Credentials {
    pub fn read(root: &Path) -> Option<Self> {
        Self::parse(&std::fs::read_to_string(root.join(CHEEVOS_FILE)).ok()?)
    }

    pub fn parse(text: &str) -> Option<Self> {
        let mut keys = slot_store::ini::parse(text);
        let mut take = |k: &str| keys.remove(k).filter(|v| !v.is_empty());
        let username = take("username")?;
        let secret = match (take("password"), take("token")) {
            (Some(p), _) => Secret::Password(p),
            (None, Some(t)) => Secret::Token(t),
            (None, None) => return None,
        };
        Some(Self {
            username,
            secret,
            host: take("host"),
        })
    }
}

pub fn remember_token(root: &Path, token: &str) -> std::io::Result<()> {
    let path = root.join(CHEEVOS_FILE);
    let text = std::fs::read_to_string(&path)?;
    let text = slot_store::ini::set(&text, "token", Some(token))?;
    let text = slot_store::ini::set(&text, "password", None)?;
    slot_store::atomic_write(&path, text.as_bytes())
}
