//! Fragile: offline Artwork.pack reader (Phase 13). Ported from pvaibhav/slot `pack.rs`.
//! Allows placing a single `Labels/Artwork.pack` file on the SD card to label carts
//! instantly without network access.
//!
//! Format:
//! ```text
//! b"SLOTART1", entry count (u32 LE)
//! per entry (sorted by CRC32): CRC32 (u32 LE), offset (u32 LE), length (u32 LE)
//! raw image data (JPEG/PNG)
//! ```

use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom};
use std::path::Path;

use slot_store::Platform;

use super::crop;

pub const FILE: &str = "Artwork.pack";
const MAGIC: &[u8; 8] = b"SLOTART1";
const HEAD: u64 = 12;
const ENTRY: u64 = 12;
const MAX_IMAGE: u32 = 4 << 20;

fn invalid(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message.to_owned())
}

/// Reads a cart label from `Labels/Artwork.pack` if present on the root.
pub fn find_in_pack(root: &Path, crc: u32, platform: Platform) -> Option<Vec<u8>> {
    let path = root.join("Labels").join(FILE);
    let bytes = match read_entry(&path, crc) {
        Ok(Some(bytes)) => bytes,
        Ok(None) => return None,
        Err(e) => {
            eprintln!("slot: artwork pack: {}: {e}", path.display());
            return None;
        }
    };
    crop::prepare_cart_label(&bytes, platform, 2)
}

fn read_entry(path: &Path, crc: u32) -> io::Result<Option<Vec<u8>>> {
    let mut file = match File::open(path) {
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
        file => file?,
    };
    let size = file.metadata()?.len();
    let mut head = [0u8; HEAD as usize];
    file.read_exact(&mut head)
        .map_err(|_| invalid("artwork pack too short"))?;
    if &head[..8] != MAGIC {
        return Err(invalid("not an artwork pack"));
    }
    let count = u64::from(u32::from_le_bytes(head[8..12].try_into().unwrap()));
    if HEAD + ENTRY * count > size {
        return Err(invalid("artwork pack table runs past the end"));
    }
    let mut table = vec![0u8; (ENTRY * count) as usize];
    file.read_exact(&mut table)?;
    let field =
        |entry: &[u8], i: usize| u32::from_le_bytes(entry[i * 4..i * 4 + 4].try_into().unwrap());
    let entries: Vec<&[u8]> = table.chunks_exact(ENTRY as usize).collect();
    let Ok(at) = entries.binary_search_by_key(&crc, |entry| field(entry, 0)) else {
        return Ok(None);
    };
    let (offset, len) = (field(entries[at], 1), field(entries[at], 2));
    if len > MAX_IMAGE || u64::from(offset) + u64::from(len) > size {
        return Err(invalid("artwork pack entry runs past the end"));
    }
    let mut image = vec![0u8; len as usize];
    file.seek(SeekFrom::Start(offset.into()))?;
    file.read_exact(&mut image)?;
    Ok(Some(image))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn write_test_pack(path: &Path, entries: &[(u32, &[u8])]) {
        let mut f = File::create(path).unwrap();
        f.write_all(MAGIC).unwrap();
        f.write_all(&(entries.len() as u32).to_le_bytes()).unwrap();
        let mut offset = HEAD + ENTRY * (entries.len() as u64);
        for (crc, data) in entries {
            f.write_all(&crc.to_le_bytes()).unwrap();
            f.write_all(&(offset as u32).to_le_bytes()).unwrap();
            f.write_all(&(data.len() as u32).to_le_bytes()).unwrap();
            offset += data.len() as u64;
        }
        for (_, data) in entries {
            f.write_all(data).unwrap();
        }
    }

    #[test]
    fn reading_pack_finds_by_crc() {
        let dir = tempfile::tempdir().unwrap();
        let pack_path = dir.path().join("Labels").join(FILE);
        std::fs::create_dir_all(pack_path.parent().unwrap()).unwrap();
        write_test_pack(&pack_path, &[(0x1234, b"test1"), (0x5678, b"test2")]);

        assert_eq!(
            read_entry(&pack_path, 0x1234).unwrap(),
            Some(b"test1".to_vec())
        );
        assert_eq!(
            read_entry(&pack_path, 0x5678).unwrap(),
            Some(b"test2".to_vec())
        );
        assert_eq!(read_entry(&pack_path, 0x9999).unwrap(), None);
    }
}
