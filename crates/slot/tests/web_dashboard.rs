mod common;

use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::Duration;

use common::tmp_root_with_carts;
use slot::web::WebServer;
use slot_ui::QuickRow;

fn base64_encode(input: &[u8]) -> String {
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    let mut i = 0;
    while i < input.len() {
        let b0 = input[i];
        let b1 = if i + 1 < input.len() { input[i + 1] } else { 0 };
        let b2 = if i + 2 < input.len() { input[i + 2] } else { 0 };

        let n = ((b0 as u32) << 16) | ((b1 as u32) << 8) | (b2 as u32);
        out.push(CHARS[((n >> 18) & 63) as usize] as char);
        out.push(CHARS[((n >> 12) & 63) as usize] as char);
        if i + 1 < input.len() {
            out.push(CHARS[((n >> 6) & 63) as usize] as char);
        } else {
            out.push('=');
        }
        if i + 2 < input.len() {
            out.push(CHARS[(n & 63) as usize] as char);
        } else {
            out.push('=');
        }
        i += 3;
    }
    out
}

fn request(
    port: u16,
    method: &str,
    path: &str,
    auth: Option<(&str, &str)>,
    body: Option<&[u8]>,
) -> (u16, HashMap<String, String>, Vec<u8>) {
    let mut stream = TcpStream::connect(("127.0.0.1", port)).expect("connect");
    stream
        .set_read_timeout(Some(Duration::from_secs(3)))
        .expect("read timeout");
    stream
        .set_write_timeout(Some(Duration::from_secs(3)))
        .expect("write timeout");

    let mut req =
        format!("{method} {path} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nConnection: close\r\n");
    if let Some((user, pass)) = auth {
        let creds = format!("{user}:{pass}");
        req.push_str(&format!(
            "Authorization: Basic {}\r\n",
            base64_encode(creds.as_bytes())
        ));
    }
    if let Some(b) = body {
        req.push_str(&format!("Content-Length: {}\r\n", b.len()));
        req.push_str("Content-Type: application/json\r\n");
    }
    req.push_str("\r\n");

    stream.write_all(req.as_bytes()).expect("write req");
    if let Some(b) = body {
        stream.write_all(b).expect("write body");
    }
    stream.flush().expect("flush");

    let mut resp_buf = Vec::new();
    stream.read_to_end(&mut resp_buf).expect("read resp");

    let header_end = resp_buf
        .windows(4)
        .position(|w| w == b"\r\n\r\n")
        .expect("header end");
    let header_str = String::from_utf8_lossy(&resp_buf[..header_end]);
    let mut lines = header_str.lines();
    let status_line = lines.next().expect("status line");
    let status_code: u16 = status_line
        .split_whitespace()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .expect("status code");

    let mut headers = HashMap::new();
    for l in lines {
        if let Some((k, v)) = l.split_once(':') {
            headers.insert(k.trim().to_ascii_lowercase(), v.trim().to_string());
        }
    }

    let body = resp_buf[header_end + 4..].to_vec();
    (status_code, headers, body)
}

#[test]
fn dashboard_auth_enforcement_and_spa() {
    let d = tmp_root_with_carts(&["Emerald", "Fusion"]);
    let server = WebServer::start(d.path().to_path_buf(), 15071).expect("start server");
    assert!(server.is_running());

    // 1. Without auth -> 401
    let (status, headers, _) = request(15071, "GET", "/", None, None);
    assert_eq!(status, 401);
    assert!(headers.contains_key("www-authenticate"));

    // 2. With wrong auth -> 401
    let (status, _, _) = request(15071, "GET", "/", Some(("root", "wrongpassword")), None);
    assert_eq!(status, 401);

    // 3. With correct auth (default "root") -> 200 OK HTML
    let (status, headers, body) = request(15071, "GET", "/", Some(("root", "root")), None);
    assert_eq!(status, 200);
    assert_eq!(
        headers.get("content-type").map(|s| s.as_str()),
        Some("text/html; charset=utf-8")
    );
    let html = String::from_utf8_lossy(&body);
    assert!(html.contains("FRAGILE"));
    assert!(html.contains("DASHBOARD"));
}

#[test]
fn dashboard_api_games_lists_inventory() {
    let d = tmp_root_with_carts(&["Emerald", "Fusion"]);
    let _server = WebServer::start(d.path().to_path_buf(), 15072).expect("start server");

    let (status, _, body) = request(15072, "GET", "/api/games", Some(("root", "root")), None);
    assert_eq!(status, 200);

    let json = String::from_utf8_lossy(&body);
    assert!(json.contains("Emerald"));
    assert!(json.contains("Fusion"));
}

#[test]
fn dashboard_api_cart_shell_persists() {
    let d = tmp_root_with_carts(&["Emerald", "Fusion"]);
    let _server = WebServer::start(d.path().to_path_buf(), 15073).expect("start server");

    let payload = br#"{"stem":"Emerald","outline":"rounded","colour":"00aa44","finish":"clear"}"#;
    let (status, _, body) = request(
        15073,
        "POST",
        "/api/cart_shell",
        Some(("root", "root")),
        Some(payload),
    );
    assert_eq!(status, 200);
    assert!(String::from_utf8_lossy(&body).contains("\"ok\":true"));

    let shell_file = d.path().join("Labels/cart_shell.txt");
    assert!(shell_file.exists());
    let content = std::fs::read_to_string(&shell_file).unwrap();
    assert!(content.contains("Emerald = rounded 00aa44 clear"));
}

#[test]
fn dashboard_api_upload_label_and_retrieve() {
    let d = tmp_root_with_carts(&["Emerald", "Fusion"]);
    let _server = WebServer::start(d.path().to_path_buf(), 15074).expect("start server");

    let dummy_png = b"\x89PNG\r\n\x1a\nfake png data for label test";
    let (status, _, _) = request(
        15074,
        "POST",
        "/api/upload_label?platform=GBA&stem=Emerald",
        Some(("root", "root")),
        Some(dummy_png),
    );
    assert_eq!(status, 200);

    let label_file = d.path().join("Labels/GBA/Emerald.png");
    assert!(label_file.exists());
    assert_eq!(std::fs::read(&label_file).unwrap(), dummy_png);

    // Retrieve via GET /api/label with canonical dir
    let (status, headers, body) = request(
        15074,
        "GET",
        "/api/label?platform=GBA&stem=Emerald",
        Some(("root", "root")),
        None,
    );
    assert_eq!(status, 200);
    assert_eq!(
        headers.get("content-type").map(|s| s.as_str()),
        Some("image/png")
    );
    assert_eq!(&body, dummy_png);

    // Retrieve via GET /api/label with full platform name ("Game Boy Advance")
    let (status, _, body2) = request(
        15074,
        "GET",
        "/api/label?platform=Game%20Boy%20Advance&stem=Emerald",
        Some(("root", "root")),
        None,
    );
    assert_eq!(status, 200);
    assert_eq!(&body2, dummy_png);

    // Retrieve scraped label from System/Cache/Scraper/Labels
    let scraped_dir = d.path().join(slot_store::SCRAPED_LABELS).join("GBA");
    std::fs::create_dir_all(&scraped_dir).unwrap();
    let scraped_png = b"\x89PNG\r\n\x1a\nscraped label data";
    std::fs::write(scraped_dir.join("Fusion.png"), scraped_png).unwrap();

    let (status, _, body_scraped) = request(
        15074,
        "GET",
        "/api/label?platform=GBA&stem=Fusion",
        Some(("root", "root")),
        None,
    );
    assert_eq!(status, 200);
    assert_eq!(&body_scraped, scraped_png);
}

#[test]
fn dashboard_api_lists_and_favorites() {
    let d = tmp_root_with_carts(&["Emerald", "Fusion"]);
    let _server = WebServer::start(d.path().to_path_buf(), 15075).expect("start server");

    // 1. Toggle favorite
    let payload = br#"{"platform":"GBA","stem":"Emerald"}"#;
    let (status, _, _) = request(
        15075,
        "POST",
        "/api/favorite",
        Some(("root", "root")),
        Some(payload),
    );
    assert_eq!(status, 200);

    let fav_file = d.path().join("Config/favorites.txt");
    assert!(fav_file.exists());
    let fav_content = std::fs::read_to_string(&fav_file).unwrap();
    assert!(fav_content.contains("GBA/Emerald"));

    // 2. Create list
    let list_payload = br#"{"name":"Classics","action":"create"}"#;
    let (status, _, _) = request(
        15075,
        "POST",
        "/api/lists",
        Some(("root", "root")),
        Some(list_payload),
    );
    assert_eq!(status, 200);

    let list_file = d.path().join("Config/Lists/Classics.txt");
    assert!(list_file.exists());

    // 3. Add to list
    let add_payload = br#"{"name":"Classics","action":"add","platform":"GBA","stem":"Emerald"}"#;
    let (status, _, _) = request(
        15075,
        "POST",
        "/api/lists",
        Some(("root", "root")),
        Some(add_payload),
    );
    assert_eq!(status, 200);

    let list_content = std::fs::read_to_string(&list_file).unwrap();
    assert!(list_content.contains("GBA/Emerald"));

    // 4. Query lists
    let (status, _, body) = request(15075, "GET", "/api/lists", Some(("root", "root")), None);
    assert_eq!(status, 200);
    let json = String::from_utf8_lossy(&body);
    assert!(json.contains("Classics"));
    assert!(json.contains("GBA/Emerald"));
}

#[test]
fn dashboard_quick_menu_toggle_and_connection_string() {
    let d = tmp_root_with_carts(&["Emerald", "Fusion"]);
    let mut a = common::boot(d.path());

    assert_eq!(
        a.quick_value(QuickRow::WebDashboard),
        Some(slot_ui::QuickValue::Off)
    );
    assert_eq!(a.web_connection_string(), None);

    // Toggle on
    a.set_web_dashboard(true);
    assert_eq!(
        a.quick_value(QuickRow::WebDashboard),
        Some(slot_ui::QuickValue::On)
    );
    assert!(slot_store::read_extras(d.path()).flag("web_dashboard", false));
}
