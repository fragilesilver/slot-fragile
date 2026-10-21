//! Whether a core library was built for this computer. Upstream's host is a Mac, whose cores
//! are .dylib and never meet the device's .so; on a Linux computer both are .so in vendor/, and
//! an aarch64 device core there is not one the tests can open. Fragile's.

use std::io::Read;
use std::path::Path;

/// The file is there and, when it is an ELF library, built for this machine's architecture.
pub fn runs_here(path: &Path) -> bool {
    let Ok(mut f) = std::fs::File::open(path) else {
        return false;
    };
    let mut head = [0u8; 20];
    if f.read_exact(&mut head).is_err() || &head[..4] != b"\x7fELF" {
        return path.exists();
    }
    let machine = u16::from_le_bytes([head[18], head[19]]);
    match std::env::consts::ARCH {
        "x86_64" => machine == 0x3e,
        "aarch64" => machine == 0xb7,
        "x86" => machine == 0x03,
        "arm" => machine == 0x28,
        _ => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn elf(machine: u16) -> std::path::PathBuf {
        let mut head = vec![0u8; 64];
        head[..4].copy_from_slice(b"\x7fELF");
        head[18..20].copy_from_slice(&machine.to_le_bytes());
        let path = std::env::temp_dir().join(format!(
            "slot-runs-here-{}-{machine:x}.so",
            std::process::id()
        ));
        std::fs::write(&path, head).unwrap();
        path
    }

    #[test]
    fn a_library_for_another_machine_is_not_one_to_open() {
        let here = match std::env::consts::ARCH {
            "aarch64" => 0xb7,
            _ => 0x3e,
        };
        let other = if here == 0xb7 { 0x3e } else { 0xb7 };
        let (mine, theirs) = (elf(here), elf(other));
        assert!(runs_here(&mine));
        assert!(!runs_here(&theirs));
        let _ = std::fs::remove_file(mine);
        let _ = std::fs::remove_file(theirs);
        assert!(!runs_here(Path::new("/nowhere/at/all.so")));
    }
}
