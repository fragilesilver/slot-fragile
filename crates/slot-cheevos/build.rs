use std::path::Path;

const SOURCES: &[&str] = &[
    "src/rc_client.c",
    "src/rc_client_external.c",
    "src/rc_compat.c",
    "src/rc_util.c",
    "src/rc_version.c",
    "src/rc_libretro.c",
    "src/rapi/rc_api_common.c",
    "src/rapi/rc_api_info.c",
    "src/rapi/rc_api_runtime.c",
    "src/rapi/rc_api_user.c",
    "src/rhash/hash.c",
    "src/rhash/hash_rom.c",
    "src/rhash/md5.c",
];

fn main() {
    let rc = Path::new("rcheevos");
    let mut build = cc::Build::new();
    build
        .include(rc.join("include"))
        .include(rc.join("src"))
        .include(rc.join("libretro"))
        .define("RC_HASH_NO_DISC", None)
        .define("RC_HASH_NO_ENCRYPTED", None)
        .define("RC_HASH_NO_ZIP", None)
        .warnings(false);
    for src in SOURCES {
        build.file(rc.join(src));
    }
    let engine = std::fs::read_dir(rc.join("src/rcheevos")).expect("vendored rcheevos is missing");
    let mut engine: Vec<_> = engine
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "c"))
        .collect();
    engine.sort();
    build.files(engine);
    build.file("csrc/slot_shim.c");
    build.compile("rcheevos");
    println!("cargo:rerun-if-changed=rcheevos");
    println!("cargo:rerun-if-changed=csrc");
}
