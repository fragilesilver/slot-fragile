// Ported from nickclyde/slot-plus@c218577, crates/slot/tests/cheevos_live.rs.

#![cfg(feature = "cheevos")]

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use slot::audio::{AudioSink, StubSink, OUTPUT_HZ};
use slot::emu::{EmuHandle, Speed};
use slot_retro::MockCore;

const LOGIN: &str = r#"{"Success":true,"User":"Player","Token":"ApiToken","Score":1,"SoftcoreScore":1,"Messages":0,"Permissions":1,"AccountType":"Registered"}"#;
const UNKNOWN: &str = r#"{"Success":true,"GameId":0,"Title":"","ConsoleId":0,"ImageIconUrl":"","RichPresenceGameId":0,"RichPresencePatch":"","Sets":[]}"#;

fn serve(seen: Arc<Mutex<Vec<String>>>) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    std::thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            let mut reader = BufReader::new(stream.try_clone().unwrap());
            let mut len = 0;
            loop {
                let mut line = String::new();
                if reader.read_line(&mut line).unwrap_or(0) == 0 || line == "\r\n" {
                    break;
                }
                if let Some(v) = line.to_ascii_lowercase().strip_prefix("content-length:") {
                    len = v.trim().parse().unwrap_or(0);
                }
            }
            let mut body = vec![0; len];
            let _ = reader.read_exact(&mut body);
            let body = String::from_utf8_lossy(&body).into_owned();
            let reply = if body.contains("r=login2") {
                LOGIN
            } else if body.contains("r=achievementsets") {
                UNKNOWN
            } else {
                r#"{"Success":true}"#
            };
            seen.lock().unwrap().push(body);
            let mut out = stream;
            let _ = write!(
                out,
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                reply.len(),
                reply
            );
        }
    });
    format!("http://{addr}")
}

fn wait_for(what: &str, f: impl Fn() -> bool) {
    let end = Instant::now() + Duration::from_secs(10);
    while !f() {
        assert!(Instant::now() < end, "timed out waiting for {what}");
        std::thread::sleep(Duration::from_millis(20));
    }
}

#[test]
fn signs_in_remembers_the_token_and_identifies_the_inserted_cart() {
    let seen = Arc::new(Mutex::new(Vec::new()));
    let host = serve(seen.clone());
    let root = tempfile::tempdir().unwrap();
    let config = root.path().join("Config/retroachievements.txt");
    std::fs::create_dir_all(config.parent().unwrap()).unwrap();
    std::fs::write(
        &config,
        format!("username = Player\npassword = hunter2\nhost = {host}\n"),
    )
    .unwrap();
    let rom: PathBuf = root.path().join("Games/GBA/Test.gba");
    std::fs::create_dir_all(rom.parent().unwrap()).unwrap();
    std::fs::write(&rom, b"slot cheevos test rom").unwrap();

    slot::cheevos::start(root.path());

    wait_for("the token to replace the password", || {
        std::fs::read_to_string(&config).is_ok_and(|t| t.contains("token = ApiToken"))
    });
    let text = std::fs::read_to_string(&config).unwrap();
    assert!(!text.contains("hunter2"), "{text}");
    assert!(seen.lock().unwrap()[0].contains("p=hunter2"));

    let mut sink = StubSink::new();
    sink.open(OUTPUT_HZ).unwrap();
    let emu = EmuHandle::spawn(Box::new(MockCore::new()), rom, sink.ring(), None, None);
    emu.set_speed(Speed::Normal);

    let md5 = "m=f22aaed72508c460a35c544a8e68d62f";
    wait_for("the cart to be identified", || {
        seen.lock()
            .unwrap()
            .iter()
            .any(|b| b.contains("r=achievementsets"))
    });
    let identify = seen
        .lock()
        .unwrap()
        .iter()
        .find(|b| b.contains("r=achievementsets"))
        .cloned()
        .unwrap();
    assert!(identify.contains("t=ApiToken"), "{identify}");
    assert!(identify.contains(md5), "{identify}");
    drop(emu);
}
