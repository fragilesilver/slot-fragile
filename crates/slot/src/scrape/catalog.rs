//! Fragile: LaunchBox CRC32 index lookup (Phase 13). Ported from pvaibhav/slot `catalog.rs`.
//! Maps No-Intro dump CRC32s directly to image filenames on LaunchBox's open CDN.

pub(crate) const INDEX: &str = include_str!("../../data/launchbox.tsv");
pub(crate) const LAUNCHBOX_CDN: &str = "https://images.launchbox-app.com/";

/// Look up image URLs on LaunchBox's CDN for a ROM's CRC32. Best first.
pub fn lookup_launchbox(crc: u32) -> Vec<String> {
    let key = format!("{crc:08X}\t");
    INDEX
        .lines()
        .find_map(|line| line.strip_prefix(&key))
        .map(|files| {
            files
                .split('\t')
                .map(|f| format!("{LAUNCHBOX_CDN}{f}"))
                .collect()
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn launchbox_index_loads_and_finds_known_dumps() {
        assert!(!INDEX.is_empty());
        // First entry in launchbox.tsv: 000206C0
        let urls = lookup_launchbox(0x000206C0);
        assert!(!urls.is_empty());
        assert!(urls[0].starts_with(LAUNCHBOX_CDN));
        assert!(urls[0].contains("r2_0c9be901"));
    }
}
