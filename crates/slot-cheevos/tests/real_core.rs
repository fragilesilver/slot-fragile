// Ported from nickclyde/slot-plus@29b8a81, crates/slot-cheevos/tests/real_core.rs.

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use slot_cheevos::{game, memory};
use slot_retro::{ButtonMask, LibretroCore, MemoryDescriptor, RetroCore};

static CORE_LOCK: Mutex<()> = Mutex::new(());

fn vendored(name: &str) -> Option<LibretroCore> {
    let p = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../vendor")
        .join(format!(
            "{name}_libretro.{}",
            std::env::consts::DLL_EXTENSION
        ));
    if !p.exists() {
        eprintln!("skipping: no {}", p.display());
        return None;
    }
    Some(LibretroCore::open(&p).expect("vendored core is present but would not open"))
}

fn write_rom(name: &str, bytes: Vec<u8>) -> PathBuf {
    let p = Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);
    std::fs::write(&p, bytes).unwrap();
    p
}

fn gba_rom() -> PathBuf {
    let mut rom = vec![0u8; 0x8000];
    rom[0..4].copy_from_slice(&0xea00002eu32.to_le_bytes());
    rom[0xb2] = 0x96;
    let sum = rom[0xa0..0xbd].iter().fold(0u8, |a, b| a.wrapping_add(*b));
    rom[0xbd] = 0u8.wrapping_sub(sum).wrapping_sub(0x19);
    rom[0xc0..0xc4].copy_from_slice(&0xeafffffeu32.to_le_bytes());
    write_rom("cheevos-real.gba", rom)
}

const GB_LOGO: [u8; 48] = [
    0xce, 0xed, 0x66, 0x66, 0xcc, 0x0d, 0x00, 0x0b, 0x03, 0x73, 0x00, 0x83, 0x00, 0x0c, 0x00, 0x0d,
    0x00, 0x08, 0x11, 0x1f, 0x88, 0x89, 0x00, 0x0e, 0xdc, 0xcc, 0x6e, 0xe6, 0xdd, 0xdd, 0xd9, 0x99,
    0xbb, 0xbb, 0x67, 0x63, 0x6e, 0x0e, 0xec, 0xcc, 0xdd, 0xdc, 0x99, 0x9f, 0xbb, 0xb9, 0x33, 0x3e,
];

fn gb_rom(cgb: u8) -> PathBuf {
    let mut rom = vec![0u8; 0x8000];
    rom[0x100..0x104].copy_from_slice(&[0x00, 0xc3, 0x50, 0x01]);
    rom[0x104..0x134].copy_from_slice(&GB_LOGO);
    rom[0x143] = cgb;
    rom[0x150..0x153].copy_from_slice(&[0xc3, 0x50, 0x01]);
    let sum = rom[0x134..0x14d]
        .iter()
        .fold(0u8, |a, b| a.wrapping_sub(*b).wrapping_sub(1));
    rom[0x14d] = sum;
    write_rom(&format!("cheevos-real-{cgb:02x}.gb"), rom)
}

fn byte_at(map: &[MemoryDescriptor], addr: usize) -> Option<u8> {
    map.iter()
        .find(|d| d.ptr != 0 && addr >= d.start && addr < d.start + d.len)
        .map(|d| unsafe { *((d.ptr + d.offset + addr - d.start) as *const u8) })
}

fn check(core: &mut LibretroCore, console: u32, ra: u32, native: usize) {
    core.run_frame(ButtonMask(0));
    let map = core.memory_map().to_vec();
    assert!(memory::refresh(&map, console), "{map:x?}");
    let want = byte_at(&map, native).expect("native address mapped");
    unsafe {
        let d = map
            .iter()
            .find(|d| native >= d.start && native < d.start + d.len)
            .unwrap();
        *((d.ptr + d.offset + native - d.start) as *mut u8) = want ^ 0x5a;
    }
    let mut b = [0u8; 1];
    assert_eq!(memory::read(ra, &mut b), 1);
    assert_eq!(b[0], want ^ 0x5a);
    memory::clear();
}

#[test]
fn mgba_gba_ram_is_where_retroachievements_expects_it() {
    let _l = CORE_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let Some(mut core) = vendored("mgba") else {
        return;
    };
    core.load(&gba_rom()).unwrap();
    check(&mut core, game::GBA, 0x0010, 0x3000010);
    check(&mut core, game::GBA, 0x8010, 0x2000010);
}

#[test]
fn gpsp_gba_ram_is_where_retroachievements_expects_it() {
    let _l = CORE_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let Some(mut core) = vendored("gpsp") else {
        return;
    };
    core.load(&gba_rom()).unwrap();
    check(&mut core, game::GBA, 0x0010, 0x3000010);
    check(&mut core, game::GBA, 0x8010, 0x2000010);
}

#[test]
fn mgba_game_boy_work_ram_is_where_retroachievements_expects_it() {
    let _l = CORE_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let Some(mut core) = vendored("mgba") else {
        return;
    };
    core.load(&gb_rom(0x00)).unwrap();
    check(&mut core, game::GB, 0xc010, 0xc010);
    check(&mut core, game::GB, 0xff90, 0xff90);
}

#[test]
fn mgba_game_boy_color_work_ram_is_where_retroachievements_expects_it() {
    let _l = CORE_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let Some(mut core) = vendored("mgba") else {
        return;
    };
    core.load(&gb_rom(0x80)).unwrap();
    check(&mut core, game::GBC, 0xc010, 0xc010);
}
