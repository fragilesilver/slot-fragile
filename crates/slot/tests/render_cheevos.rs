// Ported from nickclyde/slot-plus@c218577, crates/slot/tests/render_cheevos.rs.

#![cfg(all(any(target_os = "macos", target_os = "linux"), feature = "cheevos"))]

mod common;

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::time::{Duration, Instant};

use common::{clocked, tmp_root_with_carts};
use slot::cheevos::overlay::{rect, FACE_H, FACE_W};
use slot::frontend::Frontend;
use slot_gfx::{Compositor, HeadlessSurface, OUT_H, OUT_W};
use slot_power::SimPlatform;

const LOGIN: &str = r#"{"Success":true,"User":"Player","Token":"ApiToken","Score":1,"SoftcoreScore":1,"Messages":0,"Permissions":1,"AccountType":"Registered"}"#;

fn serve() -> String {
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
            let mut out = stream;
            let _ = write!(
                out,
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                LOGIN.len(),
                LOGIN
            );
        }
    });
    format!("http://{addr}")
}

fn save(name: &str, px: &[u8]) {
    let Ok(dir) = std::env::var("SCRATCH_PNG_DIR") else {
        return;
    };
    let path = format!("{dir}/{name}.png");
    let file = std::fs::File::create(&path).expect("create png");
    let mut e = png::Encoder::new(std::io::BufWriter::new(file), OUT_W, OUT_H);
    e.set_color(png::ColorType::Rgba);
    e.set_depth(png::BitDepth::Eight);
    e.write_header()
        .expect("png header")
        .write_image_data(px)
        .expect("png data");
    println!("wrote {path}");
}

#[test]
fn the_signed_in_banner_draws_over_the_shelf() {
    let Ok(surface) = HeadlessSurface::new() else {
        return;
    };
    let Ok(mut c) = Compositor::new(&surface) else {
        return;
    };
    let d = tmp_root_with_carts(&["Emerald"]);
    clocked(d.path());
    std::fs::write(
        d.path().join("Config/retroachievements.txt"),
        format!("username = Player\ntoken = ApiToken\nhost = {}\n", serve()),
    )
    .unwrap();
    let mut f = Frontend::boot(Box::new(SimPlatform::at(d.path().to_path_buf())));
    f.upload_faces(&mut c);

    f.compose(&mut c);
    let before = c.read_frame();

    let (x, y, _, _) = rect(OUT_W as f32);
    assert_eq!(
        x as u32 * 2 + FACE_W,
        OUT_W,
        "the toast is not centred across the top"
    );
    let (x, y) = (x as usize, y as usize);
    let inside = |px: &[u8]| -> Vec<u8> {
        (0..FACE_H as usize)
            .flat_map(|r| {
                let at = ((y + r) * OUT_W as usize + x) * 4;
                px[at..at + FACE_W as usize * 4].to_vec()
            })
            .collect()
    };

    let end = Instant::now() + Duration::from_secs(10);
    let shown = loop {
        assert!(Instant::now() < end, "the banner never appeared");
        f.compose(&mut c);
        let px = c.read_frame();
        let changed = inside(&px)
            .chunks(4)
            .zip(inside(&before).chunks(4))
            .filter(|(a, b)| a != b)
            .count();
        if changed > (FACE_W * FACE_H / 2) as usize {
            std::thread::sleep(Duration::from_millis(400));
            f.compose(&mut c);
            break c.read_frame();
        }
        std::thread::sleep(Duration::from_millis(30));
    };
    save("cheevos-banner", &shown);

    let banner = inside(&shown);
    let bright = banner
        .chunks(4)
        .filter(|p| p[0] > 0xe0 && p[1] > 0xe0 && p[2] > 0xe0)
        .count();
    assert!(bright > 200, "no light text in the banner ({bright} px)");
    let dark = banner
        .chunks(4)
        .filter(|p| p[0] < 0x30 && p[1] < 0x30 && p[2] < 0x30)
        .count();
    assert!(
        dark > (FACE_W * FACE_H / 2) as usize,
        "no dark panel ({dark} px)"
    );
    let above = ((y / 2) * OUT_W as usize + x) * 4;
    assert_eq!(shown[above..above + 4], before[above..above + 4]);
}
