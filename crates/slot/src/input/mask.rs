use slot_input::{Action, Btn, StickAxis};
use slot_retro::ButtonMask;
use slot_store::{Controls, GameButton, Modifier};

use super::bind::device_btn;

#[derive(Default)]
pub struct Pad {
    held: Vec<Btn>,
    chorded: Vec<Btn>,
    analog: [i16; 4],
    analog_seen: bool,
}

impl Pad {
    pub fn apply(&mut self, action: Action) {
        if let Action::Stick(axis, value) = action {
            let slot = match axis {
                StickAxis::LeftX => 0,
                StickAxis::LeftY => 1,
                StickAxis::RightX => 2,
                StickAxis::RightY => 3,
            };
            self.analog[slot] = value;
            self.analog_seen = true;
            return;
        }
        let (list, b, down) = match action {
            Action::GbaDown(b) => (&mut self.held, b, true),
            Action::GbaUp(b) => (&mut self.held, b, false),
            Action::MenuChordDown(b) => (&mut self.chorded, b, true),
            Action::MenuChordUp(b) => (&mut self.chorded, b, false),
            _ => return,
        };
        list.retain(|x| *x != b);
        if down {
            list.push(b);
        }
    }

    pub fn analog(&self) -> [i16; 4] {
        self.analog
    }

    pub fn analog_seen(&self) -> bool {
        self.analog_seen
    }

    pub fn mask(&self, controls: &Controls) -> ButtonMask {
        let mut bits = GameButton::ALL
            .into_iter()
            .filter(|g| self.pressed(controls, *g))
            .fold(0, |bits, g| bits | bit(g));
        if self.held.contains(&Btn::L3) {
            bits |= ButtonMask::L3;
        }
        if self.held.contains(&Btn::R3) {
            bits |= ButtonMask::R3;
        }
        ButtonMask(bits)
    }

    fn pressed(&self, controls: &Controls, g: GameButton) -> bool {
        controls.game(g).is_some_and(|binding| {
            let b = device_btn(binding.button);
            match binding.modifier {
                None => self.held.contains(&b),
                Some(Modifier::Menu) => self.chorded.contains(&b),
                Some(Modifier::Select | Modifier::Button(_)) => false,
            }
        })
    }

    /// The extra buttons the playing system's pad has (`GameButton::of`), held now. Fragile's.
    pub fn extra_mask(&self, controls: &Controls, pad: &[GameButton]) -> ButtonMask {
        let mut bits = GameButton::EXTRA
            .into_iter()
            .filter(|g| pad.contains(g) && self.pressed(controls, *g))
            .fold(0, |bits, g| bits | bit(g));
        if self.held.contains(&Btn::L3) {
            bits |= ButtonMask::L3;
        }
        if self.held.contains(&Btn::R3) {
            bits |= ButtonMask::R3;
        }
        ButtonMask(bits)
    }

    pub fn clear(&mut self) {
        self.held.clear();
        self.chorded.clear();
        self.analog = [0; 4];
    }
}

fn bit(g: GameButton) -> u16 {
    match g {
        GameButton::Up => ButtonMask::UP,
        GameButton::Down => ButtonMask::DOWN,
        GameButton::Left => ButtonMask::LEFT,
        GameButton::Right => ButtonMask::RIGHT,
        GameButton::A => ButtonMask::A,
        GameButton::B => ButtonMask::B,
        GameButton::Select => ButtonMask::SELECT,
        GameButton::Start => ButtonMask::START,
        GameButton::L => ButtonMask::L,
        GameButton::R => ButtonMask::R,
        GameButton::TurboA => ButtonMask::X,
        GameButton::TurboB => ButtonMask::Y,
        GameButton::X => ButtonMask::X,
        GameButton::Y => ButtonMask::Y,
        GameButton::L2 => ButtonMask::L2,
        GameButton::R2 => ButtonMask::R2,
    }
}
