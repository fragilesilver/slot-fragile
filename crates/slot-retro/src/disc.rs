// Fragile: libretro's disk control interface, so a game on more than one disc (a PlayStation .m3u)
// can have its discs swapped. Only the calls slot makes are kept: the tray, the disc in the drive
// and how many there are.

use std::ffi::{c_uint, c_void};

pub const SET_DISK_CONTROL_INTERFACE: c_uint = 13;
pub const GET_DISK_CONTROL_INTERFACE_VERSION: c_uint = 57;
pub const SET_DISK_CONTROL_EXT_INTERFACE: c_uint = 58;

type SetEject = unsafe extern "C" fn(bool) -> bool;
type GetIndex = unsafe extern "C" fn() -> c_uint;
type SetIndex = unsafe extern "C" fn(c_uint) -> bool;
type GetCount = unsafe extern "C" fn() -> c_uint;

/// The head of `retro_disk_control_callback`, which `retro_disk_control_ext_callback` begins
/// with too.
#[repr(C)]
struct RawDiscs {
    set_eject_state: Option<SetEject>,
    get_eject_state: Option<unsafe extern "C" fn() -> bool>,
    get_image_index: Option<GetIndex>,
    set_image_index: Option<SetIndex>,
    get_num_images: Option<GetCount>,
}

#[derive(Clone, Copy)]
pub(crate) struct Discs {
    set_eject: SetEject,
    get_index: GetIndex,
    set_index: SetIndex,
    count: GetCount,
}

/// The interface a core hands over, or None when it leaves out a call slot needs.
///
/// # Safety
/// `data` is null or points at a `retro_disk_control_callback` or its `_ext` form.
pub(crate) unsafe fn read(data: *const c_void) -> Option<Discs> {
    let raw = (data as *const RawDiscs).as_ref()?;
    Some(Discs {
        set_eject: raw.set_eject_state?,
        get_index: raw.get_image_index?,
        set_index: raw.set_image_index?,
        count: raw.get_num_images?,
    })
}

impl Discs {
    /// The disc in the drive, from 0, and how many the game has.
    pub(crate) fn now(&self) -> (u32, u32) {
        unsafe { ((self.get_index)(), (self.count)()) }
    }

    /// Opens the tray and leaves it open: PCSX ReARMed reads the lid as open until it is shut.
    pub(crate) fn open(&self) -> bool {
        unsafe { (self.set_eject)(true) }
    }

    /// Opens the tray, puts in disc `index` and closes it again. A core that keeps the lid open
    /// a while after it shuts (PCSX ReARMed does, two seconds) lets the game see the swap.
    pub(crate) fn swap(&self, index: u32) -> bool {
        let (_, count) = self.now();
        if index >= count {
            return false;
        }
        unsafe {
            (self.set_eject)(true);
            let put = (self.set_index)(index);
            (self.set_eject)(false);
            put
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

    static OPEN: AtomicBool = AtomicBool::new(false);
    static INDEX: AtomicU32 = AtomicU32::new(0);
    static SWAPPED_OPEN: AtomicBool = AtomicBool::new(false);

    unsafe extern "C" fn set_eject(open: bool) -> bool {
        OPEN.store(open, Ordering::SeqCst);
        true
    }
    unsafe extern "C" fn get_index() -> c_uint {
        INDEX.load(Ordering::SeqCst)
    }
    unsafe extern "C" fn set_index(i: c_uint) -> bool {
        SWAPPED_OPEN.store(OPEN.load(Ordering::SeqCst), Ordering::SeqCst);
        INDEX.store(i, Ordering::SeqCst);
        true
    }
    unsafe extern "C" fn count() -> c_uint {
        3
    }

    #[test]
    fn a_swap_opens_the_tray_changes_the_disc_and_shuts_it() {
        let raw = RawDiscs {
            set_eject_state: Some(set_eject),
            get_eject_state: None,
            get_image_index: Some(get_index),
            set_image_index: Some(set_index),
            get_num_images: Some(count),
        };
        let discs = unsafe { read(&raw as *const RawDiscs as *const c_void) }.expect("complete");
        assert_eq!(discs.now(), (0, 3));
        assert!(discs.swap(2));
        assert!(
            SWAPPED_OPEN.load(Ordering::SeqCst),
            "the disc went in with the tray shut"
        );
        assert!(!OPEN.load(Ordering::SeqCst), "the tray was left open");
        assert_eq!(discs.now(), (2, 3));
        assert!(!discs.swap(3), "a fourth disc of three went in");
    }

    #[test]
    fn an_interface_missing_a_call_slot_needs_is_refused() {
        let raw = RawDiscs {
            set_eject_state: Some(set_eject),
            get_eject_state: None,
            get_image_index: None,
            set_image_index: Some(set_index),
            get_num_images: Some(count),
        };
        assert!(unsafe { read(&raw as *const RawDiscs as *const c_void) }.is_none());
        assert!(unsafe { read(std::ptr::null()) }.is_none());
    }
}
