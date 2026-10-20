//! Free and total space on the card a path lives on, and which card that is, for About's
//! Details page. `card_space` is ported from suprsomething/slot.advance@681fc1d,
//! crates/slot/src/card_space.rs.

use std::ffi::CString;
use std::os::unix::ffi::OsStrExt;
use std::path::Path;

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct CardSpace {
    pub free: u64,
    pub total: u64,
}

impl CardSpace {
    /// "41.2 of 59.4 GB free", in the binary gigabytes `df -h` prints.
    pub fn text(self) -> String {
        const GB: f64 = (1u64 << 30) as f64;
        format!(
            "{:.1} of {:.1} GB free",
            self.free as f64 / GB,
            self.total as f64 / GB
        )
    }
}

/// Space on the filesystem holding `path`. Free space is what an ordinary user can write
/// (`f_bavail`), as `df` reports it. `None` if the path can't be queried.
// The statvfs fields are u64 on 64-bit Linux but narrower elsewhere, so the casts stay.
#[allow(clippy::unnecessary_cast)]
pub fn card_space(path: &Path) -> Option<CardSpace> {
    let c = CString::new(path.as_os_str().as_bytes()).ok()?;
    let mut st = std::mem::MaybeUninit::<libc::statvfs>::zeroed();
    // SAFETY: `c` is a valid NUL-terminated path and `st` points to a statvfs-sized buffer
    // the call fills in. It is only read after the call reports success.
    let rc = unsafe { libc::statvfs(c.as_ptr(), st.as_mut_ptr()) };
    if rc != 0 {
        return None;
    }
    // SAFETY: statvfs returned 0, so it filled `st` in.
    let st = unsafe { st.assume_init() };
    let unit = if st.f_frsize > 0 {
        st.f_frsize as u64
    } else {
        st.f_bsize as u64
    };
    let total = (st.f_blocks as u64).checked_mul(unit)?;
    let free = (st.f_bavail as u64).checked_mul(unit)?;
    (total > 0).then_some(CardSpace {
        free: free.min(total),
        total,
    })
}

/// The device and mount point holding `path`, from the text of `/proc/mounts`: the longest
/// mount point the path sits under.
pub fn mount_of(mounts: &str, path: &Path) -> Option<(String, String)> {
    mounts
        .lines()
        .filter_map(|line| {
            let mut fields = line.split_whitespace();
            Some((fields.next()?, unescape(fields.next()?)))
        })
        .filter(|(_, at)| path.starts_with(at))
        .max_by_key(|(_, at)| at.len())
        .map(|(dev, at)| (dev.to_string(), at))
}

/// `/proc/mounts` writes a space in a mount point as `\040`.
fn unescape(field: &str) -> String {
    field.replace("\\040", " ")
}

/// The card slot a block device is in on the H700 handhelds: the boot card is `mmcblk0`,
/// the second slot `mmcblk1`.
pub fn card_slot(device: &str) -> Option<&'static str> {
    let name = device.strip_prefix("/dev/")?;
    match () {
        _ if name.starts_with("mmcblk0") => Some("TF1"),
        _ if name.starts_with("mmcblk1") => Some("TF2"),
        _ => None,
    }
}
