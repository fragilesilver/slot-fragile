//! Which core a game opens on when that is not simply the one chosen for it. Fragile's.

use slot_store::{format_stamp, Core, Platform, StateRing};

use super::App;

impl App {
    /// The core to open `stem` on, and whether it is not the chosen one. A game linked on a
    /// core with no link play (Gambatte) opens on mGBA. Resume states do not cross cores, so
    /// both cores' resumes go into their saved states (still there to load) and mGBA starts
    /// from the save file; the chosen core will too, next time, rather than from a state older
    /// than the save the link session leaves. Each resume is kept as a saved state of its core.
    pub fn core_to_open(&mut self, stem: &str, platform: Platform, chosen: Core) -> (Core, bool) {
        let linked = self.link_player().is_some();
        if !linked || chosen.links() || !Core::Mgba.runs(platform) {
            return (chosen, false);
        }
        if let Some(h) = self.pending_save.take() {
            let _ = h.join();
        }
        if let Some(root) = self.root.clone() {
            let stamp = format_stamp(self.wall_secs());
            for core in [chosen, Core::Mgba] {
                let ring = StateRing::new(&root, platform, core, stem);
                match ring.keep_resume_as_state(&stamp) {
                    Ok(Some(to)) => eprintln!(
                        "slot: link: {} resume kept as {}",
                        core.as_str(),
                        to.display()
                    ),
                    Ok(None) => {}
                    Err(e) => eprintln!(
                        "slot: link: could not set the {} resume aside: {e}",
                        core.as_str()
                    ),
                }
            }
        }
        eprintln!(
            "slot: link: {} has no link play, opening {stem} on mGBA",
            chosen.text()
        );
        (Core::Mgba, true)
    }

    /// The two cores the open picker offers, left then right.
    pub fn core_picker_choices(&self) -> Option<[Core; 2]> {
        self.core_picker.map(|p| p.choices())
    }
}
