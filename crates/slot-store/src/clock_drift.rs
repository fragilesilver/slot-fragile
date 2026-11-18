use std::fs;
use std::path::{Path, PathBuf};

use crate::atomic::atomic_write;

pub const CLOCK_FILE: &str = "Config/clock.txt";

const MIN_LEARN_SECS: i64 = 3 * 3600;
const MAX_PPM: i64 = 50_000;

#[derive(Copy, Clone, PartialEq, Eq, Debug, Default)]
pub struct ClockDrift {
    pub shutdown_utc: Option<i64>,
    pub shutdown_hw: Option<i64>,
    pub rate_ppm: i64,
}

fn clock_path(root: &Path) -> PathBuf {
    root.join(CLOCK_FILE)
}

pub fn read_clock_drift(root: &Path) -> ClockDrift {
    let Ok(text) = fs::read_to_string(clock_path(root)) else {
        return ClockDrift::default();
    };
    let mut drift = ClockDrift::default();
    for line in text.lines() {
        let Some((k, v)) = line.trim().split_once('=') else {
            continue;
        };
        match k {
            "shutdown_utc" => drift.shutdown_utc = v.parse().ok(),
            "shutdown_hw" => drift.shutdown_hw = v.parse().ok(),
            "rate_ppm" => {
                if let Ok(ppm) = v.parse::<i64>() {
                    drift.rate_ppm = ppm.clamp(-MAX_PPM, MAX_PPM);
                }
            }
            _ => {}
        }
    }
    drift
}

pub fn write_clock_drift(root: &Path, drift: ClockDrift) {
    let mut out = format!("rate_ppm={}\n", drift.rate_ppm.clamp(-MAX_PPM, MAX_PPM));
    if let (Some(utc), Some(hw)) = (drift.shutdown_utc, drift.shutdown_hw) {
        out.push_str(&format!("shutdown_utc={utc}\nshutdown_hw={hw}\n"));
    }
    let _ = atomic_write(&clock_path(root), out.as_bytes());
}

/// Compensate an offline RTC boot using the last clean shutdown timestamp and learned drift rate.
/// Clears the clean-shutdown anchor immediately so crashes never re-apply stale offsets.
pub fn adjust_boot_utc(root: &Path, hw_now: i64) -> Option<i64> {
    let mut drift = read_clock_drift(root);
    let (Some(su), Some(sh)) = (drift.shutdown_utc, drift.shutdown_hw) else {
        return None;
    };
    let elapsed_hw = hw_now.saturating_sub(sh);
    // Clear shutdown anchor until the next clean shutdown.
    drift.shutdown_utc = None;
    drift.shutdown_hw = None;
    write_clock_drift(root, drift);
    if elapsed_hw <= 0 || drift.rate_ppm == 0 {
        return None;
    }
    let delta = (elapsed_hw as i128 * drift.rate_ppm as i128 / 1_000_000) as i64;
    Some(su + elapsed_hw + delta)
}

/// Record clean shutdown UTC + raw RTC counter (only called on clean shutdown / reboot).
pub fn record_clean_shutdown(root: &Path, utc_now: i64, hw_now: i64) {
    let mut drift = read_clock_drift(root);
    drift.shutdown_utc = Some(utc_now);
    drift.shutdown_hw = Some(hw_now);
    write_clock_drift(root, drift);
}

/// Refine the learned drift rate when the user corrects the clock manually after >=3h offline.
pub fn learn_manual_adjustment(root: &Path, prev_utc: i64, new_utc: i64) {
    if new_utc <= 0 || prev_utc <= 0 {
        return;
    }
    let mut drift = read_clock_drift(root);
    let Some(last_u) = drift.shutdown_utc.or_else(|| {
        // Check if caller passed elapsed time via a persisted anchor before clearing.
        None
    }) else {
        let _ = (prev_utc, new_utc, MIN_LEARN_SECS);
        return;
    };
    let elapsed = new_utc.saturating_sub(last_u);
    if elapsed < MIN_LEARN_SECS {
        return;
    }
    let error = new_utc - prev_utc;
    let obs_ppm = ((error as i128 * 1_000_000) / elapsed as i128) as i64;
    let total_ppm = (drift.rate_ppm + obs_ppm).clamp(-MAX_PPM, MAX_PPM);
    drift.rate_ppm = if drift.rate_ppm == 0 {
        total_ppm
    } else {
        (drift.rate_ppm + total_ppm) / 2
    };
    write_clock_drift(root, drift);
}

/// Anchor-preserving variant when boot already cleared `shutdown_*`: keeps `boot_anchor` for
/// manual refinement if corrected during that session.
#[derive(Copy, Clone, PartialEq, Eq, Debug, Default)]
pub struct DriftSession {
    pub anchor_utc: Option<i64>,
}

impl DriftSession {
    pub fn on_boot(root: &Path, hw_now: i64) -> (Self, Option<i64>) {
        let mut drift = read_clock_drift(root);
        let (Some(su), Some(sh)) = (drift.shutdown_utc, drift.shutdown_hw) else {
            return (Self::default(), None);
        };
        let elapsed_hw = hw_now.saturating_sub(sh);
        drift.shutdown_utc = None;
        drift.shutdown_hw = None;
        write_clock_drift(root, drift);
        let adjusted = if elapsed_hw > 0 && drift.rate_ppm != 0 {
            let delta = (elapsed_hw as i128 * drift.rate_ppm as i128 / 1_000_000) as i64;
            Some(su + elapsed_hw + delta)
        } else {
            None
        };
        (
            Self {
                anchor_utc: Some(su),
            },
            adjusted,
        )
    }

    pub fn learn_from_correction(&mut self, root: &Path, old_utc: i64, new_utc: i64) {
        let Some(anchor) = self.anchor_utc else {
            return;
        };
        let elapsed = new_utc.saturating_sub(anchor);
        if elapsed < MIN_LEARN_SECS {
            return;
        }
        let mut drift = read_clock_drift(root);
        let error = new_utc - old_utc;
        let obs_ppm = ((error as i128 * 1_000_000) / elapsed as i128) as i64;
        let updated = (drift.rate_ppm + obs_ppm).clamp(-MAX_PPM, MAX_PPM);
        drift.rate_ppm = if drift.rate_ppm == 0 {
            updated
        } else {
            (drift.rate_ppm + updated) / 2
        };
        write_clock_drift(root, drift);
        self.anchor_utc = Some(new_utc);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn drift_is_learned_after_three_hours_and_applied_next_boot() {
        let d = tempdir().unwrap();
        std::fs::create_dir_all(d.path().join("Config")).unwrap();
        let t0 = 1_700_000_000i64;
        record_clean_shutdown(d.path(), t0, t0);

        // Next boot 10 hours later (36,000s), oscillator ran slow by 360s (~10,000 ppm)
        let (mut sess, adj) = DriftSession::on_boot(d.path(), t0 + 35_640);
        assert_eq!(adj, None, "no learned drift yet");

        // User corrects clock forward by 360s to actual time
        sess.learn_from_correction(d.path(), t0 + 35_640, t0 + 36_000);
        assert!(read_clock_drift(d.path()).rate_ppm > 9000);

        // Crash check: without clean shutdown, next boot does NOT double-apply
        assert_eq!(adjust_boot_utc(d.path(), t0 + 40_000), None);

        // Clean shutdown & second 10-hour offline stretch applies learned rate
        let t1 = t0 + 40_000;
        record_clean_shutdown(d.path(), t1, t1);
        let corrected = adjust_boot_utc(d.path(), t1 + 35_640).unwrap();
        assert!((corrected - (t1 + 36_000)).abs() < 60);
    }
}
