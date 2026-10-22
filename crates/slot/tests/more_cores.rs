// Fragile: the real NES, SNES and PlayStation cores, built for this computer by
// `task core:more:host` into target/host-cores (or SLOT_HOST_CORES), each running a program made
// here for its system: every option Fragile gives it is one it declares, it draws frames slot
// can show, plays sound, and its state saves and loads. Skipped without the builds.

use std::path::PathBuf;
use std::sync::Mutex;

use slot_retro::{ButtonMask, LibretroCore, RetroCore};
use slot_store::{Core, Platform};

/// One core can be open at a time in a process.
static ONE_CORE: Mutex<()> = Mutex::new(());

fn core_path(core: Core) -> Option<PathBuf> {
    let dir = std::env::var_os("SLOT_HOST_CORES")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/host-cores")
        });
    let p = dir.join(format!("{}_libretro.so", core.as_str()));
    slot_retro::runs_here(&p).then_some(p)
}

/// An NROM cart whose code turns interrupts off and loops on the spot.
fn nes_rom() -> Vec<u8> {
    let mut rom = b"NES\x1a\x01\x01\x00\x00".to_vec();
    rom.resize(16, 0);
    let mut prg = vec![0xeau8; 0x4000];
    prg[..5].copy_from_slice(&[0x78, 0xd8, 0x4c, 0x02, 0xc0]); // sei; cld; jmp $c002
    for v in [0x3ffa, 0x3ffc, 0x3ffe] {
        prg[v..v + 2].copy_from_slice(&[0x00, 0xc0]);
    }
    rom.extend(prg);
    rom.extend(vec![0u8; 0x2000]);
    rom
}

/// A 128 KiB LoROM cart, its header and checksum in order, whose code switches to native mode
/// and loops on the spot.
fn snes_rom() -> Vec<u8> {
    let mut rom = vec![0u8; 0x20000];
    rom[..5].copy_from_slice(&[0x78, 0x18, 0xfb, 0x80, 0xfe]); // sei; clc; xce; bra -2
    let header = 0x7fc0;
    rom[header..header + 21].copy_from_slice(b"FRAGILE              ");
    rom[header + 0x15] = 0x20; // LoROM
    rom[header + 0x17] = 0x07; // 128 KiB
    rom[header + 0x19] = 0x01; // North America
    rom[0x7ffc..0x7ffe].copy_from_slice(&[0x00, 0x80]); // reset
    rom[0x7fea..0x7fec].copy_from_slice(&[0x00, 0x80]); // nmi
    rom[header + 0x1c..header + 0x20].copy_from_slice(&[0xff, 0xff, 0x00, 0x00]);
    let sum = rom.iter().fold(0u16, |s, b| s.wrapping_add(u16::from(*b)));
    rom[header + 0x1c..header + 0x1e].copy_from_slice(&(!sum).to_le_bytes());
    rom[header + 0x1e..header + 0x20].copy_from_slice(&sum.to_le_bytes());
    rom
}

/// A PlayStation executable that loops on the spot, which PCSX ReARMed boots on its own BIOS.
fn ps_exe() -> Vec<u8> {
    let load = 0x8001_0000u32;
    let mut exe = vec![0u8; 0x800];
    exe[..8].copy_from_slice(b"PS-X EXE");
    exe[0x10..0x14].copy_from_slice(&load.to_le_bytes()); // pc
    exe[0x18..0x1c].copy_from_slice(&load.to_le_bytes()); // where it loads
    exe[0x1c..0x20].copy_from_slice(&0x800u32.to_le_bytes()); // how much
    exe[0x30..0x34].copy_from_slice(&0x801f_fff0u32.to_le_bytes()); // stack
    let region = b"Sony Computer Entertainment Inc. for North America area";
    exe[0x4c..0x4c + region.len()].copy_from_slice(region);
    let mut code = vec![0u8; 0x800];
    let jump = 0x0800_0000 | ((load >> 2) & 0x03ff_ffff); // j <itself>; nop
    code[..4].copy_from_slice(&jump.to_le_bytes());
    exe.extend(code);
    exe
}

fn runs(core: Core, platform: Platform, file: &str, bytes: Vec<u8>) {
    runs_and(core, platform, file, bytes, |_| {});
}

fn runs_and(
    core: Core,
    platform: Platform,
    file: &str,
    bytes: Vec<u8>,
    also: impl Fn(&mut LibretroCore),
) {
    let Some(path) = core_path(core) else {
        eprintln!(
            "skipping: no {} built for this computer (task core:more:host)",
            core.as_str()
        );
        return;
    };
    let _one = ONE_CORE.lock().unwrap_or_else(|e| e.into_inner());
    let d = tempfile::tempdir().unwrap();
    let rom = d.path().join(file);
    std::fs::write(&rom, bytes).unwrap();
    let mut opened =
        LibretroCore::open_with_options(&path, d.path(), d.path(), &[]).expect("open the core");
    slot::core::apply_core_options(&mut opened, core, platform, "", false, false, None);
    opened.set_native(platform.spec().native);
    opened.load(&rom).expect("load the program");
    // FCEUmm declares its options as it loads a game, not before.
    let declared = opened.declared_options().clone();
    for (key, value) in opened.options() {
        let values = declared
            .get(&key)
            .unwrap_or_else(|| panic!("{} has no option {key}", core.as_str()));
        assert!(
            values.contains(&value),
            "{}'s {key} has no {value:?} (it has {values:?})",
            core.as_str()
        );
    }
    let mut audio = 0;
    for frame in 0..180 {
        let press = if frame % 20 < 10 {
            ButtonMask(ButtonMask::A | ButtonMask::X | ButtonMask::L2)
        } else {
            ButtonMask::default()
        };
        opened.run_frame(press);
        audio += opened.take_audio().len();
    }
    let (w, h) = opened.video_size();
    eprintln!("{}: {w}x{h}, {audio} samples", core.as_str());
    assert!(
        (128..=1024).contains(&w) && (112..=576).contains(&h),
        "{} drew a {w}x{h} picture",
        core.as_str()
    );
    assert_eq!(
        opened.video_xrgb8888().len(),
        (w * h * 4) as usize,
        "{}'s picture is not the size it says",
        core.as_str()
    );
    let av = opened.av_info();
    assert!(
        av.sample_rate > 8_000.0 && av.fps > 45.0,
        "{} runs at {} Hz and {} fps",
        core.as_str(),
        av.sample_rate,
        av.fps
    );
    assert!(audio > 0, "{} played no sound", core.as_str());
    let state = opened.serialize().expect("save a state");
    assert!(!state.is_empty(), "{}'s state is empty", core.as_str());
    opened.run_frame(ButtonMask::default());
    opened.unserialize(&state).expect("load the state back");
    also(&mut opened);
}

#[test]
fn fceumm_runs_an_nes_cart() {
    runs(Core::Fceumm, Platform::Nes, "Fragile.nes", nes_rom());
}

#[test]
fn snes9x_runs_a_snes_cart() {
    runs(Core::Snes9x, Platform::Snes, "Fragile.sfc", snes_rom());
}

#[test]
fn supafaust_runs_a_snes_cart() {
    runs(Core::Supafaust, Platform::Snes, "Fragile.sfc", snes_rom());
}

#[test]
fn pcsx_rearmed_runs_a_playstation_program_on_its_own_bios() {
    runs_and(
        Core::PcsxRearmed,
        Platform::PlayStation,
        "Fragile.exe",
        ps_exe(),
        |core| {
            let discs = core.discs();
            eprintln!("pcsx_rearmed: discs {discs:?}");
            assert!(
                discs.is_some(),
                "PCSX ReARMed gave slot no way to swap discs"
            );
        },
    );
}
