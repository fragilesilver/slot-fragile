use std::ffi::{c_char, c_void, CStr};
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;
use std::sync::OnceLock;

use chd::read::ChdReader;
use chd::Chd;

use crate::ffi;

const CHD_MAGIC: u64 = 0x4348445F534C4F54; // "CHD_SLOT"

struct ChdTrack {
    #[allow(dead_code)]
    magic: u64,
    reader: ChdReader<File>,
    unit_bytes: u32,
    header_size: u32,
}

static DEFAULT_CDREADER: OnceLock<ffi::rc_hash_cdreader_t> = OnceLock::new();

pub fn init() {
    static ONCE: OnceLock<()> = OnceLock::new();
    ONCE.get_or_init(|| unsafe {
        let mut default_reader = std::mem::zeroed();
        ffi::rc_hash_get_default_cdreader(&mut default_reader);
        let _ = DEFAULT_CDREADER.set(default_reader);

        let mut custom_reader = ffi::rc_hash_cdreader_t {
            open_track: None,
            open_track_iterator: Some(chd_open_track_iterator),
            read_sector: Some(chd_read_sector),
            close_track: Some(chd_close_track),
            first_track_sector: Some(chd_first_track_sector),
        };
        ffi::rc_hash_init_custom_cdreader(&mut custom_reader);
    });
}

unsafe extern "C" fn chd_open_track_iterator(
    path: *const c_char,
    track: u32,
    iterator: *const c_void,
) -> *mut c_void {
    if path.is_null() {
        return std::ptr::null_mut();
    }
    let cstr = CStr::from_ptr(path);
    let Ok(path_str) = cstr.to_str() else {
        return std::ptr::null_mut();
    };

    let p = Path::new(path_str);
    let is_chd = p
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case("chd"));

    if is_chd {
        if let Ok(file) = File::open(p) {
            if let Ok(chd_file) = Chd::open(file, None) {
                let unit_bytes = match chd_file.header().unit_bytes() {
                    0 => 2448,
                    u => u,
                };
                let mut reader = ChdReader::new(chd_file);

                // Determine sector header size by probing sector 16 (ISO Primary Volume Descriptor)
                let mut header_size = 24; // Default to Mode 2 Form 1 (PlayStation standard)
                let pvd_offset = 16 * unit_bytes as u64;
                let mut probe = [0u8; 32];
                if reader.seek(SeekFrom::Start(pvd_offset)).is_ok()
                    && reader.read_exact(&mut probe).is_ok()
                {
                    const SYNC: [u8; 12] = [
                        0x00, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x00,
                    ];
                    if probe[0..12] == SYNC {
                        if &probe[25..30] == b"CD001" {
                            header_size = 24; // Mode 2 Form 1
                        } else {
                            header_size = 16; // Mode 1 raw
                        }
                    } else if &probe[1..6] == b"CD001" {
                        header_size = 0; // Mode 1 cooked 2048
                    }
                }

                let chd_track = Box::new(ChdTrack {
                    magic: CHD_MAGIC,
                    reader,
                    unit_bytes,
                    header_size,
                });
                return Box::into_raw(chd_track) as *mut c_void;
            }
        }
    }

    // Fall back to default cdreader
    if let Some(default_reader) = DEFAULT_CDREADER.get() {
        if let Some(open_it) = default_reader.open_track_iterator {
            return open_it(path, track, iterator);
        }
        if let Some(open) = default_reader.open_track {
            return open(path, track);
        }
    }

    std::ptr::null_mut()
}

unsafe extern "C" fn chd_read_sector(
    track_handle: *mut c_void,
    sector: u32,
    buffer: *mut c_void,
    requested_bytes: usize,
) -> usize {
    if track_handle.is_null() || buffer.is_null() || requested_bytes == 0 {
        return 0;
    }

    // Check if this is our CHD handle
    if *(track_handle as *const u64) == CHD_MAGIC {
        let track = &mut *(track_handle as *mut ChdTrack);
        let mut total_read = 0;
        let mut cur_sector = sector;
        let mut remaining = requested_bytes;
        let mut out_ptr = buffer as *mut u8;

        while remaining > 0 {
            let chunk = remaining.min(2048);
            let sector_offset =
                cur_sector as u64 * track.unit_bytes as u64 + track.header_size as u64;
            if track.reader.seek(SeekFrom::Start(sector_offset)).is_err() {
                break;
            }
            let slice = std::slice::from_raw_parts_mut(out_ptr, chunk);
            match track.reader.read_exact(slice) {
                Ok(_) => {
                    total_read += chunk;
                    remaining -= chunk;
                    cur_sector += 1;
                    out_ptr = out_ptr.add(chunk);
                }
                Err(_) => break,
            }
        }
        return total_read;
    }

    // Delegate to default cdreader
    if let Some(default_reader) = DEFAULT_CDREADER.get() {
        if let Some(read) = default_reader.read_sector {
            return read(track_handle, sector, buffer, requested_bytes);
        }
    }

    0
}

unsafe extern "C" fn chd_close_track(track_handle: *mut c_void) {
    if track_handle.is_null() {
        return;
    }

    if *(track_handle as *const u64) == CHD_MAGIC {
        let _ = Box::from_raw(track_handle as *mut ChdTrack);
        return;
    }

    if let Some(default_reader) = DEFAULT_CDREADER.get() {
        if let Some(close) = default_reader.close_track {
            close(track_handle);
        }
    }
}

unsafe extern "C" fn chd_first_track_sector(track_handle: *mut c_void) -> u32 {
    if track_handle.is_null() {
        return 0;
    }

    if *(track_handle as *const u64) == CHD_MAGIC {
        return 0;
    }

    if let Some(default_reader) = DEFAULT_CDREADER.get() {
        if let Some(first) = default_reader.first_track_sector {
            return first(track_handle);
        }
    }

    0
}
