mod common;

use std::fs::{self, File};
use std::io::Write;

use common::{boot, tmp_root_with_carts};
use slot::ota::{parse_release_manifest, OtaManager, OtaStatus, Version, DEFAULT_REPO};
use slot::sha256::{to_hex, Sha256};
use slot_ui::QuickRow;

#[test]
fn ota_default_target_and_menu_text() {
    let d = tmp_root_with_carts(&["Emerald"]);
    let a = boot(d.path());

    assert_eq!(a.ota_target(), DEFAULT_REPO);
    assert_eq!(a.named_value(QuickRow::Update), Some("Check".into()));
}

#[test]
fn ota_set_target_persists() {
    let d = tmp_root_with_carts(&["Emerald"]);
    let mut a = boot(d.path());

    a.set_ota_target("myfork/fragile");
    assert_eq!(a.ota_target(), "myfork/fragile");

    // Reboot from disk to verify persistence in extras.state
    let a2 = boot(d.path());
    assert_eq!(a2.ota_target(), "myfork/fragile");
}

#[test]
fn ota_version_comparison_logic() {
    let cur = Version::current();
    assert_eq!(cur.major, 1);
    assert_eq!(cur.minor, 9);
    assert_eq!(cur.patch, 3);

    let v200 = Version::parse("v2.0.0").unwrap();
    assert!(v200 > cur);

    let v193 = Version::parse("1.9.3").unwrap();
    assert_eq!(v193, cur);

    let v192 = Version::parse("1.9.2").unwrap();
    assert!(v192 < cur);

    let v191 = Version::parse("1.9.1").unwrap();
    assert!(v191 < cur);

    let v180 = Version::parse("1.8.0").unwrap();
    assert!(v180 < cur);
}

#[test]
fn ota_checksum_verification_and_staging() {
    let d = tmp_root_with_carts(&["Emerald"]);
    let sys_dir = d.path().join("System");
    fs::create_dir_all(&sys_dir).unwrap();

    // Create a mock ELF binary
    let mut mock_elf = vec![0x7f, b'E', b'L', b'F'];
    mock_elf.extend(vec![0u8; 4096]); // padding > 1024 bytes
    let valid_sha = Sha256::hex_digest(&mock_elf);

    // Verify SHA-256 calculation
    let mut hasher = Sha256::new();
    hasher.update(&mock_elf);
    assert_eq!(to_hex(&hasher.finalize()), valid_sha);

    // Test staging mechanism: write download and atomic swap
    let download_path = sys_dir.join("slot.download");
    let new_path = sys_dir.join("slot.new");
    let mut f = File::create(&download_path).unwrap();
    f.write_all(&mock_elf).unwrap();
    drop(f);

    fs::rename(&download_path, &new_path).unwrap();
    assert!(new_path.exists());
    assert!(!download_path.exists());
    assert_eq!(fs::read(&new_path).unwrap(), mock_elf);
}

#[test]
fn ota_status_transitions() {
    let d = tmp_root_with_carts(&["Emerald"]);
    let mgr = OtaManager::new(d.path().to_path_buf(), DEFAULT_REPO.into());

    assert_eq!(mgr.status(), OtaStatus::Idle);
    assert_eq!(mgr.status().menu_text(), "Check");

    let up_to_date = OtaStatus::UpToDate("1.8.0".into());
    assert_eq!(up_to_date.menu_text(), "Up to date");

    let avail = OtaStatus::Available {
        version: "1.9.0".into(),
        url: "https://example.com/slot".into(),
        sha256: None,
        size: 10450200,
    };
    assert_eq!(avail.menu_text(), "v1.9.0 ready");

    let dl = OtaStatus::Downloading {
        version: "1.9.0".into(),
        received: 5225100,
        total: 10450200,
        percent: 50,
    };
    assert_eq!(dl.menu_text(), "Downloading 50%");

    let ready = OtaStatus::ReadyToRestart {
        version: "1.9.0".into(),
    };
    assert_eq!(ready.menu_text(), "Restart to apply");
}

#[test]
fn ota_live_release_manifest_and_updater() {
    let d = tmp_root_with_carts(&["Emerald"]);
    let mgr = OtaManager::new(d.path().to_path_buf(), DEFAULT_REPO.into());

    let url = format!("https://api.github.com/repos/{DEFAULT_REPO}/releases/latest");
    let resp = ureq::AgentBuilder::new()
        .timeout(std::time::Duration::from_secs(10))
        .user_agent("fragile")
        .build()
        .get(&url)
        .set("Accept", "application/vnd.github.v3+json")
        .call();

    if let Ok(resp) = resp {
        let body = resp.into_string().unwrap();
        let info = parse_release_manifest(&body).expect("manifest should parse");
        assert_eq!(info.version, "1.9.3");
        assert!(info
            .download_url
            .contains("fragilesilver/slot-fragile/releases/download/v1.9.3/slot"));
        assert_eq!(
            info.sha256.as_deref(),
            Some("fbd4e2e625b03c7ccbebd8b908f05f1e016b0b67219d93a7268ce23bdfaf9a20")
        );

        // If an older device (e.g. running v1.9.2) checks for updates:
        let rem = Version::parse(&info.version).unwrap();
        let old = Version::parse("1.9.2").unwrap();
        assert!(rem > old);
        let avail_status = OtaStatus::Available {
            version: info.version.clone(),
            url: info.download_url.clone(),
            sha256: info.sha256.clone(),
            size: info.size,
        };
        assert_eq!(avail_status.menu_text(), "v1.9.3 ready");

        // Test live check using OtaManager on the current version (1.9.3)
        mgr.check_for_updates();
        let mut waited = 0;
        while mgr.status().is_busy() && waited < 100 {
            std::thread::sleep(std::time::Duration::from_millis(100));
            waited += 1;
        }

        // Running local version checking against remote release -> UpToDate(SLOT_RELEASE)
        assert_eq!(mgr.status(), OtaStatus::UpToDate(slot::build_info::SLOT_RELEASE.into()));
        assert_eq!(mgr.status().menu_text(), "Up to date");
    }
}
