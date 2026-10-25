// Ported from nickclyde/slot-plus@29b8a81, crates/slot-cheevos/src/memory.rs.

use std::cell::RefCell;
use std::ffi::{c_uint, c_void};

use slot_retro::MemoryDescriptor;

use crate::ffi;

struct Regions {
    map: Vec<MemoryDescriptor>,
    console: u32,
    handle: *mut c_void,
}

impl Drop for Regions {
    fn drop(&mut self) {
        unsafe { ffi::slot_regions_free(self.handle) };
    }
}

thread_local! {
    static REGIONS: RefCell<Option<Regions>> = const { RefCell::new(None) };
}

pub fn refresh(map: &[MemoryDescriptor], console: u32) -> bool {
    REGIONS.with_borrow_mut(|current| {
        let same = current
            .as_ref()
            .is_some_and(|r| r.console == console && r.map == map);
        if !same && !map.is_empty() {
            *current = Some(Regions {
                map: map.to_vec(),
                console,
                handle: build(map, console),
            });
        }
        current.as_ref().is_some_and(|r| !r.handle.is_null())
    })
}

pub fn ready() -> bool {
    REGIONS.with_borrow(|r| r.as_ref().is_some_and(|r| !r.handle.is_null()))
}

pub fn clear() {
    REGIONS.with_borrow_mut(|r| *r = None);
}

pub fn read(address: u32, buffer: &mut [u8]) -> u32 {
    REGIONS.with_borrow(|r| match r {
        Some(r) if !r.handle.is_null() => unsafe {
            ffi::slot_regions_read(r.handle, address, buffer.as_mut_ptr(), buffer.len() as u32)
        },
        _ => 0,
    })
}

fn build(map: &[MemoryDescriptor], console: u32) -> *mut c_void {
    let raw: Vec<ffi::RetroMemoryDescriptor> = map
        .iter()
        .map(|d| ffi::RetroMemoryDescriptor {
            flags: d.flags,
            ptr: d.ptr as *mut c_void,
            offset: d.offset,
            start: d.start,
            select: d.select,
            disconnect: d.disconnect,
            len: d.len,
            addrspace: std::ptr::null(),
        })
        .collect();
    unsafe { ffi::slot_regions_new(raw.as_ptr(), raw.len() as c_uint, console) }
}

pub(crate) unsafe extern "C" fn read_memory(
    address: u32,
    buffer: *mut u8,
    n: u32,
    _client: *mut ffi::rc_client_t,
) -> u32 {
    if buffer.is_null() {
        return 0;
    }
    read(address, std::slice::from_raw_parts_mut(buffer, n as usize))
}
