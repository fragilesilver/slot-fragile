//! Fragile: how Game Art draws a game's label (Phase 13). Fragile is its carts and discs, so a
//! label is the game's own by default: the cart's label as printed, a PlayStation game's disc.

use crate::platform::Platform;

/// Where Game Art's labels go, read when `Labels/` has none of the player's own for a game.
pub const SCRAPED_LABELS: &str = "System/Cache/Scraper/Labels";

#[derive(Copy, Clone, PartialEq, Eq, Debug, Default)]
pub enum LabelStyle {
    /// The box, or a PlayStation game's disc.
    Art,
    /// The cart's own label as printed, or a PlayStation game's disc.
    #[default]
    Label,
    /// The game's logo on a ground coloured from its box.
    Logo,
}

impl LabelStyle {
    pub const ALL: [LabelStyle; 3] = [LabelStyle::Label, LabelStyle::Art, LabelStyle::Logo];

    pub fn key(self) -> &'static str {
        match self {
            LabelStyle::Art => "art",
            LabelStyle::Label => "label",
            LabelStyle::Logo => "logo",
        }
    }

    pub fn parse(s: &str) -> Option<LabelStyle> {
        LabelStyle::ALL.into_iter().find(|l| l.key() == s.trim())
    }

    /// The styles a system's games can take: a disc has no printed cart label of its own.
    pub fn of(platform: Platform) -> &'static [LabelStyle] {
        match platform.spec().discs {
            true => &[LabelStyle::Art, LabelStyle::Logo],
            false => &LabelStyle::ALL,
        }
    }

    /// The name a menu shows, for one system's game, or for every system's.
    pub fn text(self, platform: Option<Platform>) -> &'static str {
        let disc = platform.is_some_and(|p| p.spec().discs);
        match self {
            LabelStyle::Art if disc => "Disc Art",
            LabelStyle::Art => "Box Art",
            LabelStyle::Label if disc => "Disc Art",
            LabelStyle::Label => "Cart Label",
            LabelStyle::Logo => "Logo",
        }
    }

    /// The next or previous style a system's games can take, round the ends.
    pub fn step(self, platform: Option<Platform>, right: bool) -> LabelStyle {
        let all = platform.map_or(&LabelStyle::ALL[..], LabelStyle::of);
        let at = all.iter().position(|s| *s == self).unwrap_or(0);
        let n = all.len();
        all[if right {
            (at + 1) % n
        } else {
            (at + n - 1) % n
        }]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_disc_takes_its_art_or_its_logo_and_a_cart_its_label_too() {
        assert_eq!(
            LabelStyle::of(Platform::PlayStation),
            [LabelStyle::Art, LabelStyle::Logo]
        );
        assert_eq!(LabelStyle::of(Platform::Gba).len(), 3);
        assert_eq!(
            LabelStyle::Art.text(Some(Platform::PlayStation)),
            "Disc Art"
        );
        assert_eq!(LabelStyle::Art.text(Some(Platform::Snes)), "Box Art");
        assert_eq!(
            LabelStyle::Logo.step(Some(Platform::PlayStation), true),
            LabelStyle::Art
        );
        assert_eq!(LabelStyle::Label.step(None, false), LabelStyle::Logo);
        assert_eq!(LabelStyle::default(), LabelStyle::Label);
        for s in LabelStyle::ALL {
            assert_eq!(LabelStyle::parse(s.key()), Some(s));
        }
    }
}
