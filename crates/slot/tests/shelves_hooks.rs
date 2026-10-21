// The one-line hooks the Search and list shelves hang on, so a merge from upstream that drops
// one fails here rather than on the device. The idea is nickclyde/slot-plus@02c5890's
// crates/slot/tests/collections_hooks.rs.

const APP: &str = include_str!("../src/app.rs");
const OVERLAY: &str = include_str!("../src/app/overlay.rs");
const FRONTEND: &str = include_str!("../src/frontend.rs");

const HOOKS: &[(&str, &str, &str)] = &[
    ("app.rs", APP, "pub mod shelves;"),
    ("app.rs", APP, "List(&'static str),"),
    ("app.rs", APP, "shelves::stock(&mut app, root);"),
    ("app.rs", APP, "if shelves::input(self, action) {"),
    ("app.rs", APP, "&& !shelves::covered(self)"),
    (
        "app.rs",
        APP,
        "if let Some(title) = shelves::search_title(self) {",
    ),
    ("app.rs", APP, "shelves::draw(self, out);"),
    (
        "overlay.rs",
        OVERLAY,
        "super::shelves::lists_input(self, action)",
    ),
    (
        "frontend.rs",
        FRONTEND,
        "crate::app::shelves::sync_faces(self.session.app_mut(), compositor);",
    ),
];

#[test]
fn every_shelves_hook_survived_the_merge() {
    for (file, text, hook) in HOOKS {
        assert!(text.contains(hook), "{file}: {hook}");
    }
}
