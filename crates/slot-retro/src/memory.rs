// Ported from nickclyde/slot-plus@29b8a81, crates/slot-retro/src/memory.rs: the core's memory map,
// which RetroAchievements reads the game's memory through.

use std::ffi::{c_uint, c_void};

pub const SET_MEMORY_MAPS: c_uint = 36 | 0x10000;
pub const SET_MEMORY_MAPS_PLAIN: c_uint = 36;

const MAX_DESCRIPTORS: usize = 256;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MemoryDescriptor {
    pub flags: u64,
    pub ptr: usize,
    pub offset: usize,
    pub start: usize,
    pub select: usize,
    pub disconnect: usize,
    pub len: usize,
}

#[repr(C)]
struct RawDescriptor {
    flags: u64,
    ptr: *mut c_void,
    offset: usize,
    start: usize,
    select: usize,
    disconnect: usize,
    len: usize,
    addrspace: *const std::ffi::c_char,
}

#[repr(C)]
struct RawMap {
    descriptors: *const RawDescriptor,
    num_descriptors: c_uint,
}

pub(crate) unsafe fn descriptors(data: *mut c_void) -> Vec<MemoryDescriptor> {
    if data.is_null() {
        return Vec::new();
    }
    let map = &*(data as *const RawMap);
    if map.descriptors.is_null() {
        return Vec::new();
    }
    let n = (map.num_descriptors as usize).min(MAX_DESCRIPTORS);
    std::slice::from_raw_parts(map.descriptors, n)
        .iter()
        .map(|d| MemoryDescriptor {
            flags: d.flags,
            ptr: d.ptr as usize,
            offset: d.offset,
            start: d.start,
            select: d.select,
            disconnect: d.disconnect,
            len: d.len,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn raw(start: usize, ram: &mut [u8]) -> RawDescriptor {
        RawDescriptor {
            flags: 0,
            ptr: ram.as_mut_ptr().cast(),
            offset: 0,
            start,
            select: 0,
            disconnect: 0,
            len: ram.len(),
            addrspace: std::ptr::null(),
        }
    }

    #[test]
    fn copies_every_descriptor_the_core_declares() {
        let mut iwram = vec![0u8; 0x8000];
        let mut ewram = vec![0u8; 0x40000];
        let list = [raw(0x3000000, &mut iwram), raw(0x2000000, &mut ewram)];
        let mut map = RawMap {
            descriptors: list.as_ptr(),
            num_descriptors: 2,
        };
        let got = unsafe { descriptors((&mut map as *mut RawMap).cast()) };
        assert_eq!(got.len(), 2);
        assert_eq!(got[0].start, 0x3000000);
        assert_eq!(got[0].ptr, iwram.as_ptr() as usize);
        assert_eq!(got[1].len, 0x40000);
    }

    #[test]
    fn a_null_map_or_list_gives_nothing() {
        assert!(unsafe { descriptors(std::ptr::null_mut()) }.is_empty());
        let mut map = RawMap {
            descriptors: std::ptr::null(),
            num_descriptors: 4,
        };
        assert!(unsafe { descriptors((&mut map as *mut RawMap).cast()) }.is_empty());
    }

    #[test]
    fn caps_an_absurd_descriptor_count() {
        let mut ram = vec![0u8; 4];
        let list: Vec<RawDescriptor> = (0..MAX_DESCRIPTORS + 10)
            .map(|i| raw(i * 4, &mut ram))
            .collect();
        let mut map = RawMap {
            descriptors: list.as_ptr(),
            num_descriptors: list.len() as c_uint,
        };
        let got = unsafe { descriptors((&mut map as *mut RawMap).cast()) };
        assert_eq!(got.len(), MAX_DESCRIPTORS);
    }
}
