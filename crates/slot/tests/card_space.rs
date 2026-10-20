// card_space tests ported from suprsomething/slot.advance@681fc1d, crates/slot/tests/card_space.rs.

use std::path::Path;

use slot::card_space::{card_slot, card_space, mount_of, CardSpace};

#[test]
fn a_real_folder_reports_its_filesystems_space() {
    let dir = tempfile::tempdir().unwrap();
    let space = card_space(dir.path()).expect("a temp folder can be queried");
    assert!(space.total > 0);
    assert!(space.free <= space.total);
}

#[test]
fn a_missing_folder_reports_nothing_rather_than_zero() {
    let dir = tempfile::tempdir().unwrap();
    assert_eq!(card_space(&dir.path().join("not-there")), None);
}

const MOUNTS: &str = "\
/dev/root / ext4 rw,noatime 0 0
tmpfs /run tmpfs rw 0 0
/dev/mmcblk1p1 /mnt/sdcard vfat rw,noatime 0 0
/dev/mmcblk0p7 /mnt/system vfat ro 0 0
/dev/sda1 /mnt/usb\\040stick vfat rw 0 0
";

#[test]
fn a_path_belongs_to_the_deepest_mount_it_sits_under() {
    let at = |p: &str| mount_of(MOUNTS, Path::new(p));
    assert_eq!(
        at("/mnt/sdcard/Games/GBA"),
        Some(("/dev/mmcblk1p1".into(), "/mnt/sdcard".into()))
    );
    assert_eq!(at("/mnt/sdcardx"), Some(("/dev/root".into(), "/".into())));
    assert_eq!(
        at("/mnt/usb stick/Games"),
        Some(("/dev/sda1".into(), "/mnt/usb stick".into()))
    );
}

#[test]
fn the_boot_card_is_tf1_and_the_second_slot_tf2() {
    assert_eq!(card_slot("/dev/mmcblk0p7"), Some("TF1"));
    assert_eq!(card_slot("/dev/mmcblk1p1"), Some("TF2"));
    assert_eq!(card_slot("/dev/sda1"), None);
    assert_eq!(card_slot("tmpfs"), None);
}

#[test]
fn space_reads_in_gigabytes_with_one_decimal() {
    let gb = 1u64 << 30;
    let space = CardSpace {
        free: 41 * gb + gb / 5,
        total: 59 * gb + gb * 2 / 5,
    };
    assert_eq!(space.text(), "41.2 of 59.4 GB free");
}
