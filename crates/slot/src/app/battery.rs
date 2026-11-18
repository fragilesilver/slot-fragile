//! Fragile's battery extras: a warning at 15% and again at 10% before upstream's power off at
//! 5%, and a badge in the corner while a game plays. Ideas and tests from marghost/slot_modded.

use slot_power::{Battery, Charge};
use slot_store::BatteryDisplay;
use slot_ui::{draw_battery, gauge_width_for, Draw, Toast, GAUGE_H, HUD_INK};

use super::{App, Phase, BATTERY_CRITICAL};

const WARNINGS: [(u8, Toast); 2] = [(15, Toast::BatteryLow15), (10, Toast::BatteryLow10)];

const BADGE_MARGIN: f32 = 12.0;
const BADGE_PAD: f32 = 6.0;
const BADGE_GROUND: [f32; 4] = [0.0, 0.0, 0.0, 0.45];

/// The lowest level warned of since the battery last charged, so each warning shows once.
#[derive(Default)]
pub struct Warned(Option<u8>);

impl App {
    pub(super) fn warn_battery(&mut self, b: Battery) {
        if matches!(b.charge, Charge::Charging | Charge::Full) {
            self.battery_warned = Warned(None);
            return;
        }
        if b.percent <= BATTERY_CRITICAL
            || self.shutting_down()
            || matches!(self.phase, Phase::Doze { .. })
        {
            return;
        }
        let warned = self.battery_warned.0;
        let due = WARNINGS
            .iter()
            .filter(|(level, _)| b.percent <= *level)
            .rfind(|(level, _)| warned.is_none_or(|w| *level < w))
            .copied();
        if let Some((level, toast)) = due {
            self.battery_warned = Warned(Some(level));
            self.hud.toast(toast, self.now());
            self.pulse_haptics();
        }
    }

    pub fn battery_display(&self) -> BatteryDisplay {
        self.extras.battery_display()
    }

    pub fn battery_badge(&self) -> bool {
        self.battery_display() != BatteryDisplay::Off
    }

    pub(super) fn step_battery_display(&mut self, right: bool) {
        let next = self.battery_display().step(right);
        self.extras.set_battery_display(next);
    }

    /// The gauge in the bottom right corner over a game, while nothing else is up there.
    pub(super) fn draw_battery_badge(&self, out: &mut Vec<Draw>) {
        let display = self.battery_display();
        if display == BatteryDisplay::Off
            || !matches!(self.phase, Phase::Playing { .. })
            || self.in_game_menu_shown().is_some()
            || self.footer > 0.0
        {
            return;
        }
        let Some(b) = self.battery else {
            return;
        };
        let canvas = self.canvas();
        let w = gauge_width_for(display, self.battery_percent.w);
        if w <= 0.0 {
            return;
        }
        let h = slot_ui::HINT_H as f32;
        let (x, y) = (canvas.w - BADGE_MARGIN - w, canvas.h - BADGE_MARGIN - h);
        out.push(Draw::Rect {
            x: x - BADGE_PAD,
            y: y - BADGE_PAD / 2.0,
            w: w + 2.0 * BADGE_PAD,
            h: h + BADGE_PAD,
            colour: BADGE_GROUND,
        });
        let gauge_y = y + (h - GAUGE_H) / 2.0;
        draw_battery(
            x,
            gauge_y,
            Some(b),
            display,
            self.battery_percent,
            self.hud_bolt,
            HUD_INK,
            out,
        );
    }
}
