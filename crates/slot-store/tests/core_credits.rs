// Every core is credited: a licence file in licenses/ and a line in CREDITS.md.

use slot_store::core_credits::credit;
use slot_store::Core;

const CREDITS: &str = include_str!("../../../CREDITS.md");

#[test]
fn every_core_has_its_licence_file_and_a_line_in_the_credits() {
    let licenses = concat!(env!("CARGO_MANIFEST_DIR"), "/../../licenses");
    for core in Core::ALL {
        let c = credit(core);
        let path = std::path::Path::new(licenses).join(c.file);
        assert!(path.is_file(), "{core:?}: no {}", path.display());
        assert!(
            CREDITS.contains(c.name) && CREDITS.contains(c.license),
            "{core:?}: CREDITS.md does not name {} under {}",
            c.name,
            c.license
        );
    }
}
