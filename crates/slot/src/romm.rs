use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver};
use std::time::Duration;

use slot_store::{
    atomic_write,
    romm::{read_romm, RommConfig},
    Platform,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RommEntry {
    pub id: u64,
    pub name: String,
    pub file_name: String,
    pub platform: Platform,
}

pub enum RommEvent {
    Listed(Result<Vec<RommEntry>, String>),
    Downloaded(Result<(Platform, PathBuf), String>),
}

pub struct RommClient {
    rx: Receiver<RommEvent>,
}

fn platform_from_slug(slug: &str) -> Option<Platform> {
    match slug.to_ascii_lowercase().as_str() {
        "gba" => Some(Platform::Gba),
        "gb" => Some(Platform::Gb),
        "gbc" => Some(Platform::Gbc),
        "nes" | "famicom" => Some(Platform::Nes),
        "snes" | "sfam" | "super-nintendo" => Some(Platform::Snes),
        "psx" | "ps1" | "ps" | "playstation" => Some(Platform::PlayStation),
        "nds" | "ndsi" => Some(Platform::Nds),
        _ => None,
    }
}

fn apply_auth(req: ureq::Request, cfg: &RommConfig) -> ureq::Request {
    if !cfg.token.is_empty() {
        req.set("Authorization", &format!("Bearer {}", cfg.token))
    } else if !cfg.username.is_empty() {
        let creds = format!("{}:{}", cfg.username, cfg.password);
        req.set(
            "Authorization",
            &format!("Basic {}", base64_encode(creds.as_bytes())),
        )
    } else {
        req
    }
}

fn base64_encode(bytes: &[u8]) -> String {
    const ALPHA: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let b0 = u32::from(chunk[0]);
        let b1 = u32::from(*chunk.get(1).unwrap_or(&0));
        let b2 = u32::from(*chunk.get(2).unwrap_or(&0));
        let n = (b0 << 16) | (b1 << 8) | b2;
        out.push(ALPHA[((n >> 18) & 63) as usize] as char);
        out.push(ALPHA[((n >> 12) & 63) as usize] as char);
        out.push(if chunk.len() > 1 {
            ALPHA[((n >> 6) & 63) as usize] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            ALPHA[(n & 63) as usize] as char
        } else {
            '='
        });
    }
    out
}

fn build_agent() -> ureq::Agent {
    ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(8))
        .timeout_read(Duration::from_secs(60))
        .build()
}

pub fn spawn_list(root: &Path) -> Option<RommClient> {
    let cfg = read_romm(root)?;
    let (tx, rx) = mpsc::channel();
    let _ = std::thread::Builder::new()
        .name("slot-romm-list".into())
        .spawn(move || {
            let res = fetch_list(&cfg);
            let _ = tx.send(RommEvent::Listed(res));
        });
    Some(RommClient { rx })
}

pub fn spawn_download(root: &Path, entry: RommEntry) -> Option<RommClient> {
    let cfg = read_romm(root)?;
    let root = root.to_path_buf();
    let (tx, rx) = mpsc::channel();
    let _ = std::thread::Builder::new()
        .name("slot-romm-dl".into())
        .spawn(move || {
            let res = download_rom(&cfg, &root, &entry);
            let _ = tx.send(RommEvent::Downloaded(res));
        });
    Some(RommClient { rx })
}

impl RommClient {
    pub fn poll(&self) -> Option<RommEvent> {
        self.rx.try_recv().ok()
    }
}

fn fetch_list(cfg: &RommConfig) -> Result<Vec<RommEntry>, String> {
    let agent = build_agent();
    let url = format!("{}/api/roms?limit=250", cfg.url);
    let resp = apply_auth(agent.get(&url), cfg)
        .call()
        .map_err(|e| format!("RomM request failed: {e}"))?;
    let body: serde_json::Value =
        serde_json::from_reader(resp.into_reader()).map_err(|e| format!("RomM JSON error: {e}"))?;
    let items = body
        .get("items")
        .and_then(|v| v.as_array())
        .or_else(|| body.as_array())
        .ok_or_else(|| "Unexpected RomM response".to_string())?;
    let mut out = Vec::new();
    for item in items {
        let Some(id) = item.get("id").and_then(|v| v.as_u64()) else {
            continue;
        };
        let slug = item
            .get("platform_slug")
            .or_else(|| item.get("platform_fs_slug"))
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let Some(platform) = platform_from_slug(slug) else {
            continue;
        };
        let file_name = item
            .get("fs_name")
            .or_else(|| item.get("file_name"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        if file_name.is_empty() {
            continue;
        }
        let name = item
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or(&file_name)
            .to_string();
        out.push(RommEntry {
            id,
            name,
            file_name,
            platform,
        });
    }
    out.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    Ok(out)
}

fn download_rom(
    cfg: &RommConfig,
    root: &Path,
    entry: &RommEntry,
) -> Result<(Platform, PathBuf), String> {
    let agent = build_agent();
    let safe_name = Path::new(&entry.file_name)
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| "Invalid filename".to_string())?;
    let url = format!("{}/api/roms/{}/content/{}", cfg.url, entry.id, safe_name);
    let resp = apply_auth(agent.get(&url), cfg)
        .call()
        .map_err(|e| format!("Download failed: {e}"))?;
    let mut bytes = Vec::new();
    std::io::Read::read_to_end(&mut resp.into_reader(), &mut bytes)
        .map_err(|e| format!("Read failed: {e}"))?;
    let dir = root.join("Games").join(entry.platform.dir_name());
    std::fs::create_dir_all(&dir).map_err(|e| format!("Dir failed: {e}"))?;
    let dst = dir.join(safe_name);
    atomic_write(&dst, &bytes).map_err(|e| format!("Write failed: {e}"))?;
    Ok((entry.platform, dst))
}
