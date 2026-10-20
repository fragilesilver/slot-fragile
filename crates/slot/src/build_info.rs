/// The upstream slot release Fragile is built on. The crates stay at 0.1.0, so this is the
/// number About shows; a test holds it to the newest heading in CHANGELOG.md.
pub const SLOT_RELEASE: &str = "1.8.0";

#[derive(Copy, Clone, Debug)]
pub struct Build {
    pub version: &'static str,
    pub hash: &'static str,
    pub date: &'static str,
}

impl Build {
    pub fn current() -> Build {
        Build {
            version: env!("CARGO_PKG_VERSION"),
            hash: env!("SLOT_GIT_HASH"),
            date: env!("SLOT_BUILD_DATE"),
        }
    }

    pub fn serial(&self) -> String {
        self.hash.to_uppercase()
    }
}
