//! Embedded local LAN Web Dashboard for Fragile.
//! Serves a retro-styled single page web application on port 1507
//! for cartridge shell editing, label uploading, collections management,
//! and save file backup/restore across all 6 supported platforms.

use std::collections::HashMap;
use std::fs;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

pub const DASHBOARD_PORT: u16 = 1507;

pub struct WebServer {
    shutdown: Arc<AtomicBool>,
    thread: Option<std::thread::JoinHandle<()>>,
    pub port: u16,
}

impl WebServer {
    pub fn start(root: PathBuf, port: u16) -> Result<Self, String> {
        let listener = TcpListener::bind(("0.0.0.0", port))
            .map_err(|e| format!("Failed to bind dashboard to port {port}: {e}"))?;
        listener
            .set_nonblocking(true)
            .map_err(|e| format!("Failed to set non-blocking: {e}"))?;

        let shutdown = Arc::new(AtomicBool::new(false));
        let stop_flag = shutdown.clone();

        let thread = std::thread::Builder::new()
            .name("slot-web-dashboard".into())
            .spawn(move || {
                run_listener(listener, root, stop_flag);
            })
            .map_err(|e| format!("Failed to spawn web thread: {e}"))?;

        Ok(WebServer {
            shutdown,
            thread: Some(thread),
            port,
        })
    }

    pub fn stop(&mut self) {
        self.shutdown.store(true, Ordering::Relaxed);
        if let Some(handle) = self.thread.take() {
            let _ = handle.join();
        }
    }

    pub fn is_running(&self) -> bool {
        !self.shutdown.load(Ordering::Relaxed)
    }
}

impl Drop for WebServer {
    fn drop(&mut self) {
        self.stop();
    }
}

fn run_listener(listener: TcpListener, root: PathBuf, shutdown: Arc<AtomicBool>) {
    while !shutdown.load(Ordering::Relaxed) {
        match listener.accept() {
            Ok((mut stream, _addr)) => {
                let _ = stream.set_read_timeout(Some(Duration::from_secs(5)));
                let _ = stream.set_write_timeout(Some(Duration::from_secs(5)));
                handle_client(&mut stream, &root);
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(Duration::from_millis(50));
            }
            Err(_) => {
                std::thread::sleep(Duration::from_millis(100));
            }
        }
    }
}

fn handle_client(stream: &mut TcpStream, root: &Path) {
    let mut buf = Vec::new();
    let mut chunk = [0u8; 4096];
    let mut header_end = None;

    // Read headers
    while header_end.is_none() {
        match stream.read(&mut chunk) {
            Ok(0) => return,
            Ok(n) => {
                buf.extend_from_slice(&chunk[..n]);
                if let Some(pos) = find_subsequence(&buf, b"\r\n\r\n") {
                    header_end = Some(pos);
                } else if buf.len() > 65536 {
                    return; // Headers too long
                }
            }
            Err(_) => return,
        }
    }

    let header_end_pos = header_end.unwrap();
    let header_str = String::from_utf8_lossy(&buf[..header_end_pos]);
    let mut lines = header_str.lines();
    let request_line = match lines.next() {
        Some(l) => l,
        None => return,
    };

    let mut parts = request_line.split_whitespace();
    let method = match parts.next() {
        Some(m) => m,
        None => return,
    };
    let uri = match parts.next() {
        Some(u) => u,
        None => return,
    };

    let mut headers = HashMap::new();
    for line in lines {
        if let Some((k, v)) = line.split_once(':') {
            headers.insert(k.trim().to_ascii_lowercase(), v.trim().to_string());
        }
    }

    // Check Authentication
    let expected_pw = crate::ssh::read_ssh_password(Some(root));
    if !check_auth(headers.get("authorization"), &expected_pw) {
        send_response(
            stream,
            401,
            "text/plain",
            b"Unauthorized",
            Some("Basic realm=\"Fragile Web Dashboard\""),
        );
        return;
    }

    // Read remaining body if Content-Length specified
    let content_len: usize = headers
        .get("content-length")
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);

    let body_start = header_end_pos + 4;
    let mut body = buf[body_start..].to_vec();
    while body.len() < content_len {
        let needed = content_len - body.len();
        let to_read = needed.min(chunk.len());
        match stream.read(&mut chunk[..to_read]) {
            Ok(0) => break,
            Ok(n) => body.extend_from_slice(&chunk[..n]),
            Err(_) => break,
        }
    }

    route_request(stream, method, uri, &headers, &body, root);
}

fn route_request(
    stream: &mut TcpStream,
    method: &str,
    uri: &str,
    headers: &HashMap<String, String>,
    body: &[u8],
    root: &Path,
) {
    let (path, query) = match uri.split_once('?') {
        Some((p, q)) => (p, q),
        None => (uri, ""),
    };
    let query_map = parse_query(query);

    match (method, path) {
        ("GET", "/") => {
            let html = include_str!("web/index.html");
            send_response(
                stream,
                200,
                "text/html; charset=utf-8",
                html.as_bytes(),
                None,
            );
        }
        ("GET", "/favicon.ico") => {
            send_response(stream, 204, "text/plain", b"", None);
        }
        ("GET", "/api/status") => {
            let json = handle_api_status(root);
            send_response(stream, 200, "application/json", json.as_bytes(), None);
        }
        ("GET", "/api/games") => {
            let json = handle_api_games(root);
            send_response(stream, 200, "application/json", json.as_bytes(), None);
        }
        ("POST", "/api/cart_shell") => {
            let res = handle_api_cart_shell(body, root);
            send_response(stream, 200, "application/json", res.as_bytes(), None);
        }
        ("POST", "/api/upload_label") => {
            let platform = query_map
                .get("platform")
                .cloned()
                .or_else(|| headers.get("x-platform").cloned())
                .unwrap_or_default();
            let stem = query_map
                .get("stem")
                .cloned()
                .or_else(|| headers.get("x-stem").cloned())
                .unwrap_or_default();
            let res = handle_api_upload_label(&platform, &stem, body, root);
            send_response(stream, 200, "application/json", res.as_bytes(), None);
        }
        ("GET", "/api/label") => {
            let platform = query_map.get("platform").cloned().unwrap_or_default();
            let stem = query_map.get("stem").cloned().unwrap_or_default();
            if let Some(bytes) = handle_api_get_label(&platform, &stem, root) {
                send_response(stream, 200, "image/png", &bytes, None);
            } else {
                send_response(stream, 404, "text/plain", b"Label Not Found", None);
            }
        }
        ("GET", "/api/lists") => {
            let json = handle_api_get_lists(root);
            send_response(stream, 200, "application/json", json.as_bytes(), None);
        }
        ("POST", "/api/lists") => {
            let res = handle_api_post_lists(body, root);
            send_response(stream, 200, "application/json", res.as_bytes(), None);
        }
        ("POST", "/api/favorite") => {
            let res = handle_api_post_favorite(body, root);
            send_response(stream, 200, "application/json", res.as_bytes(), None);
        }
        ("GET", "/api/saves") => {
            let json = handle_api_get_saves(root);
            send_response(stream, 200, "application/json", json.as_bytes(), None);
        }
        ("GET", "/api/save") => {
            let platform = query_map.get("platform").cloned().unwrap_or_default();
            let stem = query_map.get("stem").cloned().unwrap_or_default();
            if let Some(bytes) = handle_api_download_save(&platform, &stem, root) {
                send_response(stream, 200, "application/octet-stream", &bytes, None);
            } else {
                send_response(stream, 404, "text/plain", b"Save Not Found", None);
            }
        }
        ("POST", "/api/save") => {
            let platform = query_map.get("platform").cloned().unwrap_or_default();
            let stem = query_map.get("stem").cloned().unwrap_or_default();
            let res = handle_api_upload_save(&platform, &stem, body, root);
            send_response(stream, 200, "application/json", res.as_bytes(), None);
        }
        ("POST", "/api/upload_rom") => {
            let platform = query_map.get("platform").cloned().unwrap_or_default();
            let filename = query_map.get("filename").cloned().unwrap_or_default();
            let res = handle_api_upload_rom(&platform, &filename, body, root);
            send_response(stream, 200, "application/json", res.as_bytes(), None);
        }
        ("POST", "/api/restart") => {
            let _ = fs::write("/tmp/slot_restart", b"restart\n");
            send_response(stream, 200, "application/json", b"{\"ok\":true}", None);
        }
        _ => {
            send_response(stream, 404, "text/plain", b"Not Found", None);
        }
    }
}

fn check_auth(header: Option<&String>, expected_pw: &str) -> bool {
    let Some(auth) = header else {
        return false;
    };
    let Some(b64) = auth.strip_prefix("Basic ") else {
        return false;
    };
    let Some(decoded_bytes) = decode_base64(b64.trim()) else {
        return false;
    };
    let Ok(credentials) = std::str::from_utf8(&decoded_bytes) else {
        return false;
    };
    let password = match credentials.split_once(':') {
        Some((_, p)) => p,
        None => credentials,
    };
    password == expected_pw
}

fn send_response(
    stream: &mut TcpStream,
    status: u16,
    content_type: &str,
    body: &[u8],
    auth_header: Option<&str>,
) {
    let status_line = match status {
        200 => "HTTP/1.1 200 OK",
        204 => "HTTP/1.1 204 No Content",
        401 => "HTTP/1.1 401 Unauthorized",
        404 => "HTTP/1.1 404 Not Found",
        _ => "HTTP/1.1 500 Internal Server Error",
    };
    let mut headers = format!(
        "{status_line}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n",
        body.len()
    );
    if let Some(auth) = auth_header {
        headers.push_str(&format!("WWW-Authenticate: {auth}\r\n"));
    }
    headers.push_str("\r\n");

    let _ = stream.write_all(headers.as_bytes());
    if !body.is_empty() {
        let _ = stream.write_all(body);
    }
    let _ = stream.flush();
}

fn handle_api_status(root: &Path) -> String {
    let model = std::env::var("BASEOS_MODEL").unwrap_or_else(|_| "RG35XX Pro".into());
    let target = std::env::var("BASEOS_TARGET").unwrap_or_else(|_| "H700".into());

    let (free_bytes, total_bytes) = get_disk_space(root);
    let (battery, charging) = get_battery_info();

    format!(
        "{{\"model\":\"{}\",\"target\":\"{}\",\"battery\":{},\"charging\":{},\"storage\":{{\"free_bytes\":{},\"total_bytes\":{}}}}}",
        escape_json(&model),
        escape_json(&target),
        battery.map_or("null".to_string(), |b| b.to_string()),
        charging,
        free_bytes,
        total_bytes
    )
}

fn handle_api_games(root: &Path) -> String {
    let carts = slot_store::scan(root).unwrap_or_default();
    let favorites = slot_store::read_slot_state(root).favorites;
    let mut items = Vec::new();

    for c in carts {
        let platform_dir = c.platform.dir_name();
        let platform_name = c.platform.name();
        let stem = &c.stem;
        let is_fav = favorites
            .iter()
            .any(|f| f.platform == c.platform && f.stem == *stem);
        let has_label = c.label.is_some();
        let shell_val = c.shell.map(|s| s.to_value()).unwrap_or_default();

        items.push(format!(
            "{{\"platform\":\"{}\",\"platform_name\":\"{}\",\"stem\":\"{}\",\"has_label\":{},\"favorite\":{},\"shell\":\"{}\"}}",
            escape_json(platform_dir),
            escape_json(platform_name),
            escape_json(stem),
            has_label,
            is_fav,
            escape_json(&shell_val)
        ));
    }

    format!("[{}]", items.join(","))
}

fn resolve_platform(platform_str: &str) -> (Option<slot_store::Platform>, String) {
    let plat = slot_store::Platform::ALL.into_iter().find(|p| {
        p.dir_name().eq_ignore_ascii_case(platform_str)
            || p.key().eq_ignore_ascii_case(platform_str)
            || p.name().eq_ignore_ascii_case(platform_str)
            || p.short_name().eq_ignore_ascii_case(platform_str)
    });
    let dir_name = plat
        .map(|p| p.dir_name().to_string())
        .unwrap_or_else(|| platform_str.to_string());
    (plat, dir_name)
}

fn handle_api_cart_shell(body: &[u8], root: &Path) -> String {
    let body_str = String::from_utf8_lossy(body);
    let parsed: HashMap<String, String> = parse_json_or_form(&body_str);

    let stem = parsed.get("stem").cloned().unwrap_or_default();
    let outline = parsed
        .get("outline")
        .cloned()
        .unwrap_or_else(|| "auto".into());
    let colour = parsed.get("colour").cloned().unwrap_or_default();
    let finish = parsed
        .get("finish")
        .cloned()
        .unwrap_or_else(|| "solid".into());

    if stem.is_empty() {
        return "{\"ok\":false,\"error\":\"Missing stem\"}".into();
    }

    let file_path = root.join(slot_store::LABELS_SHELL_FILE);
    let existing = fs::read_to_string(&file_path).unwrap_or_default();
    let mut lines: Vec<String> = existing
        .lines()
        .filter(|l| {
            if let Some((k, _)) = l.split_once('=') {
                k.trim() != stem
            } else {
                true
            }
        })
        .map(|s| s.to_string())
        .collect();

    if outline != "auto" || !colour.is_empty() {
        let hex = colour.trim_start_matches('#');
        lines.push(format!("{stem} = {outline} {hex} {finish}"));
    }

    if let Some(parent) = file_path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let _ = fs::write(&file_path, lines.join("\n") + "\n");

    "{\"ok\":true}".into()
}

fn handle_api_upload_label(platform_str: &str, stem: &str, body: &[u8], root: &Path) -> String {
    if platform_str.is_empty() || stem.is_empty() || body.is_empty() {
        return "{\"ok\":false,\"error\":\"Invalid parameters\"}".into();
    }
    let (_, dir_name) = resolve_platform(platform_str);
    let target_dir = root.join("Labels").join(&dir_name);
    let _ = fs::create_dir_all(&target_dir);
    let target_file = target_dir.join(format!("{stem}.png"));

    match fs::write(&target_file, body) {
        Ok(()) => "{\"ok\":true}".into(),
        Err(e) => format!(
            "{{\"ok\":false,\"error\":\"{}\"}}",
            escape_json(&e.to_string())
        ),
    }
}

fn handle_api_get_label(platform_str: &str, stem: &str, root: &Path) -> Option<Vec<u8>> {
    let (plat, dir_name) = resolve_platform(platform_str);

    let candidates = [
        root.join("Labels").join(&dir_name),
        root.join(slot_store::SCRAPED_LABELS).join(&dir_name),
    ];

    let target_png = format!("{stem}.png").to_lowercase();

    for dir in &candidates {
        let direct = dir.join(format!("{stem}.png"));
        if direct.is_file() {
            if let Ok(bytes) = fs::read(direct) {
                return Some(bytes);
            }
        }
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                if let Ok(name) = entry.file_name().into_string() {
                    if name.to_lowercase() == target_png {
                        if let Ok(bytes) = fs::read(entry.path()) {
                            return Some(bytes);
                        }
                    }
                }
            }
        }
    }

    if let Ok(carts) = slot_store::scan(root) {
        if let Some(c) = carts.iter().find(|c| {
            c.stem.eq_ignore_ascii_case(stem)
                && plat.map_or_else(
                    || c.platform.dir_name().eq_ignore_ascii_case(&dir_name),
                    |p| c.platform == p,
                )
        }) {
            if let Some(label_path) = &c.label {
                if let Ok(bytes) = fs::read(label_path) {
                    return Some(bytes);
                }
            }
        }
    }

    let art_kinds = ["box.png", "cart.png", "logo.png"];
    let art_dir = root.join("System/Cache/Scraper/Art").join(&dir_name).join(stem);
    for kind in art_kinds {
        let p = art_dir.join(kind);
        if p.is_file() {
            if let Ok(bytes) = fs::read(p) {
                return Some(bytes);
            }
        }
    }

    None
}

fn handle_api_get_lists(root: &Path) -> String {
    let lists_dir = root.join(slot_store::collections::LISTS_DIR);
    let mut out = HashMap::new();

    if let Ok(entries) = fs::read_dir(lists_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("txt") {
                if let Some(name) = path.file_stem().and_then(|s| s.to_str()) {
                    let content = fs::read_to_string(&path).unwrap_or_default();
                    let games: Vec<String> = content
                        .lines()
                        .map(|l| l.trim().to_string())
                        .filter(|l| !l.is_empty() && !l.starts_with('#'))
                        .collect();
                    out.insert(name.to_string(), games);
                }
            }
        }
    }

    let mut json_parts = Vec::new();
    for (k, v) in out {
        let items: Vec<String> = v
            .iter()
            .map(|s| format!("\"{}\"", escape_json(s)))
            .collect();
        json_parts.push(format!("\"{}\":[ {} ]", escape_json(&k), items.join(",")));
    }
    format!("{{{}}}", json_parts.join(","))
}

fn handle_api_post_lists(body: &[u8], root: &Path) -> String {
    let body_str = String::from_utf8_lossy(body);
    let parsed = parse_json_or_form(&body_str);

    let name = parsed.get("name").cloned().unwrap_or_default();
    let action = parsed.get("action").cloned().unwrap_or_default();
    let platform = parsed.get("platform").cloned().unwrap_or_default();
    let stem = parsed.get("stem").cloned().unwrap_or_default();

    if name.is_empty() {
        return "{\"ok\":false,\"error\":\"Missing list name\"}".into();
    }

    let lists_dir = root.join(slot_store::collections::LISTS_DIR);
    let _ = fs::create_dir_all(&lists_dir);
    let list_file = lists_dir.join(format!("{name}.txt"));

    match action.as_str() {
        "create" => {
            if !list_file.exists() {
                let _ = fs::write(&list_file, b"");
            }
            "{\"ok\":true}".into()
        }
        "delete" => {
            let _ = fs::remove_file(&list_file);
            "{\"ok\":true}".into()
        }
        "add" => {
            let entry = if !platform.is_empty() {
                format!("{platform}/{stem}")
            } else {
                stem
            };
            let mut content = fs::read_to_string(&list_file).unwrap_or_default();
            if !content.lines().any(|l| l.trim() == entry) {
                if !content.is_empty() && !content.ends_with('\n') {
                    content.push('\n');
                }
                content.push_str(&entry);
                content.push('\n');
                let _ = fs::write(&list_file, content);
            }
            "{\"ok\":true}".into()
        }
        "remove" => {
            let entry = if !platform.is_empty() {
                format!("{platform}/{stem}")
            } else {
                stem
            };
            let content = fs::read_to_string(&list_file).unwrap_or_default();
            let lines: Vec<&str> = content
                .lines()
                .filter(|l| {
                    l.trim() != entry && l.trim() != entry.split('/').next_back().unwrap_or("")
                })
                .collect();
            let _ = fs::write(&list_file, lines.join("\n") + "\n");
            "{\"ok\":true}".into()
        }
        _ => "{\"ok\":false,\"error\":\"Invalid action\"}".into(),
    }
}

fn handle_api_post_favorite(body: &[u8], root: &Path) -> String {
    let body_str = String::from_utf8_lossy(body);
    let parsed = parse_json_or_form(&body_str);

    let platform = parsed.get("platform").cloned().unwrap_or_default();
    let stem = parsed.get("stem").cloned().unwrap_or_default();

    if platform.is_empty() || stem.is_empty() {
        return "{\"ok\":false,\"error\":\"Missing platform or stem\"}".into();
    }

    let (_, dir_name) = resolve_platform(&platform);

    let fav_file = root.join("Config/favorites.txt");
    let entry = format!("{dir_name}/{stem}");

    let content = fs::read_to_string(&fav_file).unwrap_or_default();
    let mut lines: Vec<String> = content.lines().map(|s| s.trim().to_string()).collect();

    if let Some(pos) = lines.iter().position(|l| {
        l.eq_ignore_ascii_case(&entry)
            || l.eq_ignore_ascii_case(&format!("{dir_name}/{stem}"))
    }) {
        lines.remove(pos);
    } else {
        lines.push(entry);
    }

    let _ = fs::create_dir_all(root.join("Config"));
    let _ = fs::write(&fav_file, lines.join("\n") + "\n");

    "{\"ok\":true}".into()
}

fn handle_api_get_saves(root: &Path) -> String {
    let saves_dir = root.join("Saves");
    let mut out = Vec::new();

    if let Ok(platforms) = fs::read_dir(saves_dir) {
        for p_entry in platforms.flatten() {
            if let Ok(file_type) = p_entry.file_type() {
                if file_type.is_dir() {
                    let platform_name = p_entry.file_name().to_string_lossy().into_owned();
                    if let Ok(saves) = fs::read_dir(p_entry.path()) {
                        for s_entry in saves.flatten() {
                            let path = s_entry.path();
                            let ext = path.extension().and_then(|s| s.to_str());
                            if ext == Some("sav") || ext == Some("srm") {
                                if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                                    let size = s_entry.metadata().map(|m| m.len()).unwrap_or(0);
                                    out.push(format!(
                                        "{{\"platform\":\"{}\",\"stem\":\"{}\",\"size\":{}}}",
                                        escape_json(&platform_name),
                                        escape_json(stem),
                                        size
                                    ));
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    format!("[{}]", out.join(","))
}

fn handle_api_download_save(platform_str: &str, stem: &str, root: &Path) -> Option<Vec<u8>> {
    let (_, dir_name) = resolve_platform(platform_str);
    let p1 = root
        .join("Saves")
        .join(&dir_name)
        .join(format!("{stem}.sav"));
    let p2 = root
        .join("Saves")
        .join(&dir_name)
        .join(format!("{stem}.srm"));
    fs::read(&p1).or_else(|_| fs::read(&p2)).ok()
}

fn handle_api_upload_save(platform_str: &str, stem: &str, body: &[u8], root: &Path) -> String {
    if platform_str.is_empty() || stem.is_empty() || body.is_empty() {
        return "{\"ok\":false,\"error\":\"Invalid parameters\"}".into();
    }
    let (_, dir_name) = resolve_platform(platform_str);
    let target_dir = root.join("Saves").join(&dir_name);
    let _ = fs::create_dir_all(&target_dir);
    let target_file = target_dir.join(format!("{stem}.sav"));

    match fs::write(&target_file, body) {
        Ok(()) => "{\"ok\":true}".into(),
        Err(e) => format!(
            "{{\"ok\":false,\"error\":\"{}\"}}",
            escape_json(&e.to_string())
        ),
    }
}

fn handle_api_upload_rom(platform_str: &str, filename: &str, body: &[u8], root: &Path) -> String {
    if platform_str.is_empty() || filename.is_empty() || body.is_empty() {
        return "{\"ok\":false,\"error\":\"Invalid parameters\"}".into();
    }
    let (_, dir_name) = resolve_platform(platform_str);
    let target_dir = root.join("Games").join(&dir_name);
    let _ = fs::create_dir_all(&target_dir);
    let target_file = target_dir.join(filename);

    match fs::write(&target_file, body) {
        Ok(()) => "{\"ok\":true}".into(),
        Err(e) => format!(
            "{{\"ok\":false,\"error\":\"{}\"}}",
            escape_json(&e.to_string())
        ),
    }
}

// Helpers
fn find_subsequence(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).position(|w| w == needle)
}

fn parse_query(q: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for pair in q.split('&') {
        if let Some((k, v)) = pair.split_once('=') {
            map.insert(url_decode(k), url_decode(v));
        }
    }
    map
}

fn url_decode(s: &str) -> String {
    let mut out = Vec::new();
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(b) =
                u8::from_str_radix(std::str::from_utf8(&bytes[i + 1..i + 3]).unwrap_or(""), 16)
            {
                out.push(b);
                i += 3;
                continue;
            }
        }
        if bytes[i] == b'+' {
            out.push(b' ');
        } else {
            out.push(bytes[i]);
        }
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn decode_base64(s: &str) -> Option<Vec<u8>> {
    let mut out = Vec::new();
    let mut val = 0u32;
    let mut bits = 0;
    for &b in s.as_bytes() {
        if b == b'=' {
            break;
        }
        let v = match b {
            b'A'..=b'Z' => b - b'A',
            b'a'..=b'z' => b - b'a' + 26,
            b'0'..=b'9' => b - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            b' ' | b'\r' | b'\n' | b'\t' => continue,
            _ => return None,
        } as u32;
        val = (val << 6) | v;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((val >> bits) as u8);
        }
    }
    Some(out)
}

fn escape_json(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}

fn parse_json_or_form(body: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    let trimmed = body.trim();
    if trimmed.starts_with('{') && trimmed.ends_with('}') {
        let inner = &trimmed[1..trimmed.len() - 1];
        for pair in inner.split(',') {
            if let Some((k, v)) = pair.split_once(':') {
                let k = k.trim().trim_matches('"');
                let v = v.trim().trim_matches('"');
                map.insert(k.to_string(), v.to_string());
            }
        }
    } else {
        for pair in trimmed.split('&') {
            if let Some((k, v)) = pair.split_once('=') {
                map.insert(url_decode(k), url_decode(v));
            }
        }
    }
    map
}

fn get_disk_space(root: &Path) -> (u64, u64) {
    #[cfg(unix)]
    {
        use std::ffi::CString;
        use std::mem::MaybeUninit;
        let c_path = CString::new(root.to_str().unwrap_or("/")).unwrap_or_default();
        let mut stat: MaybeUninit<libc::statvfs> = MaybeUninit::uninit();
        if unsafe { libc::statvfs(c_path.as_ptr(), stat.as_mut_ptr()) } == 0 {
            let stat = unsafe { stat.assume_init() };
            let free = stat.f_bavail * stat.f_frsize;
            let total = stat.f_blocks * stat.f_frsize;
            return (free, total);
        }
    }
    (16 * 1024 * 1024 * 1024, 64 * 1024 * 1024 * 1024)
}

fn get_battery_info() -> (Option<u8>, bool) {
    let capacity_path = Path::new("/sys/class/power_supply/battery/capacity");
    let status_path = Path::new("/sys/class/power_supply/battery/status");

    let capacity = fs::read_to_string(capacity_path)
        .ok()
        .and_then(|s| s.trim().parse::<u8>().ok());

    let charging = fs::read_to_string(status_path)
        .map(|s| s.trim().eq_ignore_ascii_case("charging"))
        .unwrap_or(false);

    (capacity, charging)
}
