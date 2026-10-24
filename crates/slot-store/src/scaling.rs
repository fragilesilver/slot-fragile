//! Screen scaling options per platform: Fit (nearest aspect fit), Sharp Fit (integer-snap
//! sharp-bilinear filtering), and Integer (whole integer pixel multiples).

use crate::{ExtrasState, Platform};

#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
pub enum Scaling {
    #[default]
    Fit,
    SharpFit,
    Integer,
}

impl Scaling {
    pub const ALL: [Scaling; 3] = [Scaling::Fit, Scaling::SharpFit, Scaling::Integer];

    pub fn key(self) -> &'static str {
        match self {
            Scaling::Fit => "fit",
            Scaling::SharpFit => "sharp",
            Scaling::Integer => "integer",
        }
    }

    pub fn text(self) -> &'static str {
        match self {
            Scaling::Fit => "Fit",
            Scaling::SharpFit => "Sharp Fit",
            Scaling::Integer => "Integer",
        }
    }

    pub fn parse(s: &str) -> Option<Scaling> {
        match s.trim().to_ascii_lowercase().as_str() {
            "fit" => Some(Scaling::Fit),
            "sharp" | "sharp_fit" | "sharp fit" | "sharpfit" => Some(Scaling::SharpFit),
            "integer" | "integer_scale" => Some(Scaling::Integer),
            _ => None,
        }
    }

    pub fn default_for(platform: Platform) -> Scaling {
        match platform {
            Platform::Gba => Scaling::SharpFit,
            _ => Scaling::Fit,
        }
    }

    pub fn step(self, right: bool) -> Scaling {
        match (self, right) {
            (Scaling::Fit, true) => Scaling::SharpFit,
            (Scaling::SharpFit, true) => Scaling::Integer,
            (Scaling::Integer, true) => Scaling::Fit,
            (Scaling::Fit, false) => Scaling::Integer,
            (Scaling::SharpFit, false) => Scaling::Fit,
            (Scaling::Integer, false) => Scaling::SharpFit,
        }
    }
}

pub fn scaling_key(platform: Platform) -> String {
    format!("scaling_{}", platform.key())
}

pub fn get_scaling(extras: &ExtrasState, platform: Platform) -> Scaling {
    if let Some(val) = extras.get(&scaling_key(platform)) {
        if let Some(s) = Scaling::parse(val) {
            return s;
        }
    }
    // Migration: an old integer_<platform> flag in extras reads as Scaling = Integer
    let old_key = format!("integer_{}", platform.key());
    if extras.flag(&old_key, false) {
        return Scaling::Integer;
    }
    Scaling::default_for(platform)
}

pub fn set_scaling(extras: &mut ExtrasState, platform: Platform, scaling: Scaling) {
    extras.set(&scaling_key(platform), Some(scaling.key()));
    let old_key = format!("integer_{}", platform.key());
    if extras.get(&old_key).is_some() {
        extras.set_flag(&old_key, scaling == Scaling::Integer);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scaling_step_cycles_both_directions() {
        let s = Scaling::Fit;
        assert_eq!(s.step(true), Scaling::SharpFit);
        assert_eq!(s.step(true).step(true), Scaling::Integer);
        assert_eq!(s.step(true).step(true).step(true), Scaling::Fit);

        assert_eq!(s.step(false), Scaling::Integer);
        assert_eq!(s.step(false).step(false), Scaling::SharpFit);
        assert_eq!(s.step(false).step(false).step(false), Scaling::Fit);
    }

    #[test]
    fn scaling_parse_and_keys() {
        assert_eq!(Scaling::parse("fit"), Some(Scaling::Fit));
        assert_eq!(Scaling::parse("sharp"), Some(Scaling::SharpFit));
        assert_eq!(Scaling::parse("sharp_fit"), Some(Scaling::SharpFit));
        assert_eq!(Scaling::parse("integer"), Some(Scaling::Integer));
        assert_eq!(Scaling::parse("unknown"), None);

        assert_eq!(Scaling::Fit.key(), "fit");
        assert_eq!(Scaling::SharpFit.key(), "sharp");
        assert_eq!(Scaling::Integer.key(), "integer");

        assert_eq!(Scaling::Fit.text(), "Fit");
        assert_eq!(Scaling::SharpFit.text(), "Sharp Fit");
        assert_eq!(Scaling::Integer.text(), "Integer");
    }

    #[test]
    fn scaling_defaults_and_migration() {
        let mut extras = ExtrasState::default();

        // Defaults: GBA is SharpFit, others are Fit
        assert_eq!(get_scaling(&extras, Platform::Gba), Scaling::SharpFit);
        assert_eq!(get_scaling(&extras, Platform::Gb), Scaling::Fit);
        assert_eq!(get_scaling(&extras, Platform::Snes), Scaling::Fit);

        // Old integer_<platform> flag reads as Integer
        extras.set_flag("integer_gba", true);
        extras.set_flag("integer_gb", true);
        assert_eq!(get_scaling(&extras, Platform::Gba), Scaling::Integer);
        assert_eq!(get_scaling(&extras, Platform::Gb), Scaling::Integer);

        // Setting scaling explicitly overrides old flag and updates it
        set_scaling(&mut extras, Platform::Gba, Scaling::Fit);
        assert_eq!(get_scaling(&extras, Platform::Gba), Scaling::Fit);
        assert_eq!(extras.get("scaling_gba"), Some("fit"));
        assert_eq!(extras.get("integer_gba"), Some("0"));
    }
}
