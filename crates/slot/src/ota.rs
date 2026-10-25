//! Over-The-Air (OTA) update system for Fragile.
//!
//! Checks GitHub releases or a custom manifest for newer versions of Fragile,
//! downloads release assets asynchronously with live progress reporting,
//! validates SHA-256 integrity and ELF binary validity, and stages the update
//! at `$SLOT_ROOT/System/slot.new` for atomic swap and crash-safe reboot.

use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use crate::sha256::{to_hex, Sha256};

pub const DEFAULT_REPO: &str = "fragkicul/fragile";
const CHUNK_SIZE: usize = 65536;
const CONNECT_TIMEOUT_SECS: u64 = 15;
const READ_TIMEOUT_SECS: u64 = 180;

/// Parsed semantic version for comparison.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct Version {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
    pub rev: u32,
}

impl Version {
    pub fn parse(s: &str) -> Option<Self> {
        let clean = s.trim().trim_start_matches(['v', 'V']);
        let mut nums = Vec::new();
        for seg in clean.split(['.', '-', '_']) {
            if let Ok(n) = seg.parse::<u32>() {
                nums.push(n);
            } else {
                let digits: String = seg.chars().filter(|c| c.is_ascii_digit()).collect();
                if let Ok(n) = digits.parse::<u32>() {
                    nums.push(n);
                }
            }
        }
        if nums.is_empty() {
            return None;
        }
        Some(Version {
            major: nums.first().copied().unwrap_or(0),
            minor: nums.get(1).copied().unwrap_or(0),
            patch: nums.get(2).copied().unwrap_or(0),
            rev: nums.get(3).copied().unwrap_or(0),
        })
    }

    pub fn current() -> Self {
        Self::parse(crate::build_info::SLOT_RELEASE).unwrap_or(Version {
            major: 1,
            minor: 8,
            patch: 0,
            rev: 0,
        })
    }
}

/// Metadata describing an available update.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UpdateInfo {
    pub version: String,
    pub download_url: String,
    pub sha256: Option<String>,
    pub size: u64,
}

/// State of the OTA update process.
#[derive(Clone, Debug, PartialEq)]
pub enum OtaStatus {
    Idle,
    Checking,
    UpToDate(String),
    Available {
        version: String,
        url: String,
        sha256: Option<String>,
        size: u64,
    },
    Downloading {
        version: String,
        received: u64,
        total: u64,
        percent: u8,
    },
    Verifying,
    ReadyToRestart {
        version: String,
    },
    Failed(String),
}

impl OtaStatus {
    pub fn is_busy(&self) -> bool {
        matches!(
            self,
            OtaStatus::Checking | OtaStatus::Downloading { .. } | OtaStatus::Verifying
        )
    }

    pub fn menu_text(&self) -> String {
        match self {
            OtaStatus::Idle => "Check".into(),
            OtaStatus::Checking => "Checking...".into(),
            OtaStatus::UpToDate(_) => "Up to date".into(),
            OtaStatus::Available { version, .. } => format!("v{version} ready"),
            OtaStatus::Downloading { percent, .. } => format!("Downloading {percent}%"),
            OtaStatus::Verifying => "Verifying...".into(),
            OtaStatus::ReadyToRestart { .. } => "Restart to apply".into(),
            OtaStatus::Failed(_) => "Check (retry)".into(),
        }
    }
}

/// Lightweight JSON AST for release parsing without heavy external dependencies.
#[derive(Clone, Debug, PartialEq)]
pub enum Json {
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    Array(Vec<Json>),
    Object(Vec<(String, Json)>),
}

impl Json {
    pub fn parse(input: &str) -> Option<Json> {
        let mut chars = input.trim().chars().peekable();
        Self::parse_val(&mut chars)
    }

    pub fn get(&self, key: &str) -> Option<&Json> {
        match self {
            Json::Object(pairs) => pairs.iter().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Json::String(s) => Some(s.as_str()),
            _ => None,
        }
    }

    pub fn as_u64(&self) -> Option<u64> {
        match self {
            Json::Number(n) => Some(*n as u64),
            _ => None,
        }
    }

    pub fn as_array(&self) -> Option<&[Json]> {
        match self {
            Json::Array(arr) => Some(arr.as_slice()),
            _ => None,
        }
    }

    fn skip_ws<I: Iterator<Item = char>>(chars: &mut std::iter::Peekable<I>) {
        while let Some(&c) = chars.peek() {
            if c.is_whitespace() {
                chars.next();
            } else {
                break;
            }
        }
    }

    fn parse_val<I: Iterator<Item = char>>(chars: &mut std::iter::Peekable<I>) -> Option<Json> {
        Self::skip_ws(chars);
        match chars.peek()? {
            '{' => Self::parse_obj(chars),
            '[' => Self::parse_arr(chars),
            '"' => Self::parse_str(chars).map(Json::String),
            't' | 'f' => Self::parse_bool(chars).map(Json::Bool),
            'n' => {
                let s: String = chars.by_ref().take(4).collect();
                if s == "null" {
                    Some(Json::Null)
                } else {
                    None
                }
            }
            c if *c == '-' || c.is_ascii_digit() => Self::parse_num(chars).map(Json::Number),
            _ => None,
        }
    }

    fn parse_obj<I: Iterator<Item = char>>(chars: &mut std::iter::Peekable<I>) -> Option<Json> {
        chars.next(); // skip '{'
        let mut pairs = Vec::new();
        loop {
            Self::skip_ws(chars);
            if let Some(&'}') = chars.peek() {
                chars.next();
                return Some(Json::Object(pairs));
            }
            let key = Self::parse_str(chars)?;
            Self::skip_ws(chars);
            if chars.next()? != ':' {
                return None;
            }
            let val = Self::parse_val(chars)?;
            pairs.push((key, val));
            Self::skip_ws(chars);
            match chars.peek() {
                Some(',') => {
                    chars.next();
                }
                Some('}') => {
                    chars.next();
                    return Some(Json::Object(pairs));
                }
                _ => return None,
            }
        }
    }

    fn parse_arr<I: Iterator<Item = char>>(chars: &mut std::iter::Peekable<I>) -> Option<Json> {
        chars.next(); // skip '['
        let mut items = Vec::new();
        loop {
            Self::skip_ws(chars);
            if let Some(&']') = chars.peek() {
                chars.next();
                return Some(Json::Array(items));
            }
            let val = Self::parse_val(chars)?;
            items.push(val);
            Self::skip_ws(chars);
            match chars.peek() {
                Some(',') => {
                    chars.next();
                }
                Some(']') => {
                    chars.next();
                    return Some(Json::Array(items));
                }
                _ => return None,
            }
        }
    }

    fn parse_str<I: Iterator<Item = char>>(chars: &mut std::iter::Peekable<I>) -> Option<String> {
        if chars.next()? != '"' {
            return None;
        }
        let mut out = String::new();
        while let Some(c) = chars.next() {
            match c {
                '"' => return Some(out),
                '\\' => {
                    let esc = chars.next()?;
                    match esc {
                        '"' => out.push('"'),
                        '\\' => out.push('\\'),
                        '/' => out.push('/'),
                        'n' => out.push('\n'),
                        'r' => out.push('\r'),
                        't' => out.push('\t'),
                        'u' => {
                            let hex: String = chars.by_ref().take(4).collect();
                            if let Ok(u) = u32::from_str_radix(&hex, 16) {
                                if let Some(ch) = char::from_u32(u) {
                                    out.push(ch);
                                }
                            }
                        }
                        _ => out.push(esc),
                    }
                }
                other => out.push(other),
            }
        }
        None
    }

    fn parse_bool<I: Iterator<Item = char>>(chars: &mut std::iter::Peekable<I>) -> Option<bool> {
        if let Some(&'t') = chars.peek() {
            let s: String = chars.by_ref().take(4).collect();
            if s == "true" {
                Some(true)
            } else {
                None
            }
        } else {
            let s: String = chars.by_ref().take(5).collect();
            if s == "false" {
                Some(false)
            } else {
                None
            }
        }
    }

    fn parse_num<I: Iterator<Item = char>>(chars: &mut std::iter::Peekable<I>) -> Option<f64> {
        let mut num_str = String::new();
        while let Some(&c) = chars.peek() {
            if c.is_ascii_digit() || c == '.' || c == '-' || c == '+' || c == 'e' || c == 'E' {
                num_str.push(c);
                chars.next();
            } else {
                break;
            }
        }
        num_str.parse().ok()
    }
}

/// Extract release information from GitHub Releases JSON or a custom manifest.
pub fn parse_release_manifest(raw: &str) -> Option<UpdateInfo> {
    let json = Json::parse(raw)?;

    // 1. Version tag
    let version_str = json
        .get("tag_name")
        .and_then(|v| v.as_str())
        .or_else(|| json.get("version").and_then(|v| v.as_str()))?;
    let version = version_str.trim_start_matches(['v', 'V']).to_string();

    // 2. Direct manifest with url field
    if let Some(url) = json.get("url").and_then(|v| v.as_str()) {
        let sha256 = json
            .get("sha256")
            .and_then(|v| v.as_str())
            .map(|s| s.to_ascii_lowercase());
        let size = json.get("size").and_then(|v| v.as_u64()).unwrap_or(0);
        return Some(UpdateInfo {
            version,
            download_url: url.to_string(),
            sha256,
            size,
        });
    }

    // 3. GitHub release with assets array
    let assets = json.get("assets").and_then(|v| v.as_array())?;

    // Look for slot binary or release asset
    let mut download_url = None;
    let mut size = 0;
    let mut sha256 = None;

    // First scan assets for slot binary
    for a in assets {
        let name = a.get("name").and_then(|v| v.as_str()).unwrap_or("");
        let url = a.get("browser_download_url").and_then(|v| v.as_str());
        let asset_size = a.get("size").and_then(|v| v.as_u64()).unwrap_or(0);

        if name == "slot"
            || name == "slot-device"
            || name == "slot.bin"
            || name.starts_with("slot-device")
        {
            download_url = url.map(|s| s.to_string());
            size = asset_size;
            break;
        }
    }

    // Fallback: take the first asset if none named slot
    if download_url.is_none() && !assets.is_empty() {
        if let Some(a) = assets.first() {
            download_url = a
                .get("browser_download_url")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            size = a.get("size").and_then(|v| v.as_u64()).unwrap_or(0);
        }
    }

    // Search body or sha256 asset for expected checksum
    if let Some(body) = json.get("body").and_then(|v| v.as_str()) {
        sha256 = extract_sha256_from_text(body);
    }

    let download_url = download_url?;
    Some(UpdateInfo {
        version,
        download_url,
        sha256,
        size,
    })
}

/// Extract 64-character hex SHA-256 checksum from text if present.
pub fn extract_sha256_from_text(text: &str) -> Option<String> {
    for line in text.lines() {
        for word in line.split_whitespace() {
            let clean = word.trim_matches(|c: char| !c.is_ascii_hexdigit());
            if clean.len() == 64 && clean.chars().all(|c| c.is_ascii_hexdigit()) {
                return Some(clean.to_ascii_lowercase());
            }
        }
    }
    None
}

/// Thread-safe manager handling background update checks and downloads.
pub struct OtaManager {
    root: PathBuf,
    target: String,
    status: Arc<Mutex<OtaStatus>>,
    cancel: Arc<AtomicBool>,
}

impl OtaManager {
    pub fn new(root: PathBuf, target: String) -> Self {
        Self {
            root,
            target,
            status: Arc::new(Mutex::new(OtaStatus::Idle)),
            cancel: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn status(&self) -> OtaStatus {
        self.status.lock().unwrap().clone()
    }

    pub fn set_target(&mut self, target: String) {
        self.target = target;
    }

    pub fn check_for_updates(&self) {
        {
            let mut st = self.status.lock().unwrap();
            if st.is_busy() {
                return;
            }
            *st = OtaStatus::Checking;
        }

        self.cancel.store(false, Ordering::SeqCst);
        let status = Arc::clone(&self.status);
        let cancel = Arc::clone(&self.cancel);
        let target = self.target.clone();

        thread::spawn(move || {
            let result = Self::run_check(&target, &cancel);
            if cancel.load(Ordering::SeqCst) {
                return;
            }
            let mut st = status.lock().unwrap();
            match result {
                Ok(Some(info)) => {
                    let cur = Version::current();
                    let rem = Version::parse(&info.version).unwrap_or_default();
                    if rem > cur {
                        *st = OtaStatus::Available {
                            version: info.version,
                            url: info.download_url,
                            sha256: info.sha256,
                            size: info.size,
                        };
                    } else {
                        *st = OtaStatus::UpToDate(info.version);
                    }
                }
                Ok(None) => {
                    *st = OtaStatus::UpToDate(crate::build_info::SLOT_RELEASE.into());
                }
                Err(err) => {
                    *st = OtaStatus::Failed(err);
                }
            }
        });
    }

    fn run_check(target: &str, cancel: &AtomicBool) -> Result<Option<UpdateInfo>, String> {
        let url = if target.starts_with("http://") || target.starts_with("https://") {
            target.to_string()
        } else {
            format!("https://api.github.com/repos/{target}/releases/latest")
        };

        if cancel.load(Ordering::SeqCst) {
            return Ok(None);
        }

        let agent = ureq::AgentBuilder::new()
            .timeout_connect(Duration::from_secs(CONNECT_TIMEOUT_SECS))
            .timeout(Duration::from_secs(30))
            .user_agent("fragile")
            .build();

        let resp = agent
            .get(&url)
            .set("Accept", "application/vnd.github.v3+json, application/json")
            .call()
            .map_err(|e| format!("Check failed: {e}"))?;

        let body = resp
            .into_string()
            .map_err(|e| format!("Failed to read response: {e}"))?;

        if cancel.load(Ordering::SeqCst) {
            return Ok(None);
        }

        Ok(parse_release_manifest(&body))
    }

    pub fn start_download(&self) {
        let (version, url, expected_sha, total_size) = {
            let st = self.status.lock().unwrap();
            match &*st {
                OtaStatus::Available {
                    version,
                    url,
                    sha256,
                    size,
                } => (version.clone(), url.clone(), sha256.clone(), *size),
                _ => return,
            }
        };

        self.cancel.store(false, Ordering::SeqCst);
        let status = Arc::clone(&self.status);
        let cancel = Arc::clone(&self.cancel);
        let root = self.root.clone();

        thread::spawn(move || {
            let res = Self::run_download(
                &root,
                &version,
                &url,
                expected_sha.as_deref(),
                total_size,
                &status,
                &cancel,
            );
            if cancel.load(Ordering::SeqCst) {
                return;
            }
            let mut st = status.lock().unwrap();
            match res {
                Ok(()) => {
                    *st = OtaStatus::ReadyToRestart { version };
                }
                Err(err) => {
                    *st = OtaStatus::Failed(err);
                }
            }
        });
    }

    fn run_download(
        root: &Path,
        version: &str,
        url: &str,
        expected_sha: Option<&str>,
        total_hint: u64,
        status: &Arc<Mutex<OtaStatus>>,
        cancel: &AtomicBool,
    ) -> Result<(), String> {
        let sys_dir = root.join("System");
        if !sys_dir.exists() {
            fs::create_dir_all(&sys_dir)
                .map_err(|e| format!("Could not create System dir: {e}"))?;
        }

        let download_path = sys_dir.join("slot.download");
        let stage_path = sys_dir.join("slot.new");

        // Clean any leftover download file
        let _ = fs::remove_file(&download_path);

        let agent = ureq::AgentBuilder::new()
            .timeout_connect(Duration::from_secs(CONNECT_TIMEOUT_SECS))
            .timeout(Duration::from_secs(READ_TIMEOUT_SECS))
            .user_agent("fragile")
            .build();

        let resp = agent
            .get(url)
            .set("Accept", "application/octet-stream")
            .call()
            .map_err(|e| format!("Download request failed: {e}"))?;

        let total = resp
            .header("Content-Length")
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(total_hint);

        let mut reader = resp.into_reader();
        let mut file = File::create(&download_path)
            .map_err(|e| format!("Failed to create download file: {e}"))?;

        let mut hasher = Sha256::new();
        let mut buffer = [0u8; CHUNK_SIZE];
        let mut received = 0u64;

        loop {
            if cancel.load(Ordering::SeqCst) {
                let _ = fs::remove_file(&download_path);
                return Err("Download cancelled".into());
            }

            let bytes_read = reader
                .read(&mut buffer)
                .map_err(|e| format!("Read error: {e}"))?;
            if bytes_read == 0 {
                break;
            }

            let chunk = &buffer[..bytes_read];
            file.write_all(chunk)
                .map_err(|e| format!("Write error: {e}"))?;
            hasher.update(chunk);
            received += bytes_read as u64;

            let percent = (received * 100).checked_div(total).unwrap_or(0).min(99) as u8;

            let mut st = status.lock().unwrap();
            *st = OtaStatus::Downloading {
                version: version.to_string(),
                received,
                total,
                percent,
            };
        }

        file.flush().map_err(|e| format!("Flush error: {e}"))?;
        drop(file);

        // Verification phase
        {
            let mut st = status.lock().unwrap();
            *st = OtaStatus::Verifying;
        }

        if received < 1024 {
            let _ = fs::remove_file(&download_path);
            return Err("Downloaded binary is too small to be valid".into());
        }

        let computed_sha = to_hex(&hasher.finalize());
        if let Some(expected) = expected_sha {
            let exp_clean = expected.trim().to_ascii_lowercase();
            if computed_sha != exp_clean {
                let _ = fs::remove_file(&download_path);
                return Err(format!(
                    "SHA-256 checksum mismatch (expected {exp_clean}, got {computed_sha})"
                ));
            }
        }

        // Validate ELF header if raw binary
        if let Ok(mut check_file) = File::open(&download_path) {
            let mut magic = [0u8; 4];
            if check_file.read_exact(&mut magic).is_ok() && magic != [0x7f, b'E', b'L', b'F'] {
                let _ = fs::remove_file(&download_path);
                return Err("Downloaded asset does not have a valid ELF header".into());
            }
        }

        // Set executable permissions
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = fs::set_permissions(&download_path, fs::Permissions::from_mode(0o755));
        }

        // Atomically rename to slot.new
        fs::rename(&download_path, &stage_path)
            .map_err(|e| format!("Failed to stage slot.new: {e}"))?;

        // Sync to flash storage
        #[cfg(unix)]
        unsafe {
            libc::sync();
        }

        Ok(())
    }

    pub fn cancel(&self) {
        self.cancel.store(true, Ordering::SeqCst);
        let mut st = self.status.lock().unwrap();
        *st = OtaStatus::Idle;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_versions() {
        assert_eq!(
            Version::parse("1.8.0"),
            Some(Version {
                major: 1,
                minor: 8,
                patch: 0,
                rev: 0
            })
        );
        assert_eq!(
            Version::parse("v1.9.0"),
            Some(Version {
                major: 1,
                minor: 9,
                patch: 0,
                rev: 0
            })
        );
        assert_eq!(
            Version::parse("v1.9.1.3"),
            Some(Version {
                major: 1,
                minor: 9,
                patch: 1,
                rev: 3
            })
        );
        assert_eq!(
            Version::parse("v1.9.0-fragile.2"),
            Some(Version {
                major: 1,
                minor: 9,
                patch: 0,
                rev: 2
            })
        );
        assert!(Version::parse("v1.9.0") > Version::parse("v1.8.0"));
        assert!(Version::parse("v1.9.1") > Version::parse("v1.9.0"));
        assert_eq!(Version::parse("invalid"), None);
    }

    #[test]
    fn json_parser_basic() {
        let raw = r#"{
            "tag_name": "v1.9.0",
            "name": "Fragile 1.9.0",
            "assets": [
                {
                    "name": "slot",
                    "size": 10450200,
                    "browser_download_url": "https://github.com/fragkicul/fragile/releases/download/v1.9.0/slot"
                }
            ],
            "body": "Fragile 1.9.0 release with SHA256: e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        }"#;

        let info = parse_release_manifest(raw).expect("manifest should parse");
        assert_eq!(info.version, "1.9.0");
        assert_eq!(
            info.download_url,
            "https://github.com/fragkicul/fragile/releases/download/v1.9.0/slot"
        );
        assert_eq!(info.size, 10450200);
        assert_eq!(
            info.sha256,
            Some("e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855".into())
        );
    }

    #[test]
    fn json_direct_manifest() {
        let raw = r#"{
            "version": "1.9.2",
            "url": "https://example.com/slot",
            "sha256": "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
            "size": 9500000
        }"#;

        let info = parse_release_manifest(raw).expect("direct manifest should parse");
        assert_eq!(info.version, "1.9.2");
        assert_eq!(info.download_url, "https://example.com/slot");
        assert_eq!(info.size, 9500000);
        assert_eq!(
            info.sha256,
            Some("ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad".into())
        );
    }

    #[test]
    fn extract_sha256() {
        let text = "Release notes:\nCheck binary sha256:\n248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1  slot\nDone.";
        assert_eq!(
            extract_sha256_from_text(text),
            Some("248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1".into())
        );
    }

    #[test]
    fn ota_status_menu_texts() {
        assert_eq!(OtaStatus::Idle.menu_text(), "Check");
        assert_eq!(OtaStatus::Checking.menu_text(), "Checking...");
        assert_eq!(
            OtaStatus::UpToDate("1.8.0".into()).menu_text(),
            "Up to date"
        );
        assert_eq!(
            OtaStatus::Available {
                version: "1.9.0".into(),
                url: "".into(),
                sha256: None,
                size: 0
            }
            .menu_text(),
            "v1.9.0 ready"
        );
        assert_eq!(
            OtaStatus::Downloading {
                version: "1.9.0".into(),
                received: 50,
                total: 100,
                percent: 50
            }
            .menu_text(),
            "Downloading 50%"
        );
        assert_eq!(
            OtaStatus::ReadyToRestart {
                version: "1.9.0".into()
            }
            .menu_text(),
            "Restart to apply"
        );
    }
}
