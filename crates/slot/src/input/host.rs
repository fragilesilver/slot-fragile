use gilrs::{Axis, EventType, Gilrs};
use slot_input::{Btn, InputSource, Millis, RawEvent, StickAxis};
use winit::event::WindowEvent;
use winit::keyboard::{KeyCode, PhysicalKey};

use super::keys::key_to_btn;
use super::pad::pad_to_btn;

pub struct HostInput {
    pending: Vec<RawEvent>,
    pad: Option<Gilrs>,
    lid_closed: bool,
    held_keys: Vec<Btn>,
}

impl HostInput {
    pub fn new() -> Self {
        HostInput {
            pending: Vec::new(),
            pad: Gilrs::new().ok(),
            lid_closed: false,
            held_keys: Vec::new(),
        }
    }

    pub fn on_window_event(&mut self, event: &WindowEvent) {
        match event {
            WindowEvent::Focused(false) => self.release_keys(),
            WindowEvent::KeyboardInput { event, .. } => {
                let PhysicalKey::Code(code) = event.physical_key else {
                    return;
                };
                self.key(code, event.state.is_pressed(), event.repeat);
            }
            _ => {}
        }
    }

    fn release_keys(&mut self) {
        for btn in std::mem::take(&mut self.held_keys) {
            self.pending.push(RawEvent::Up(btn));
        }
    }

    pub fn key(&mut self, code: KeyCode, pressed: bool, repeat: bool) {
        if repeat {
            return;
        }
        let Some(btn) = key_to_btn(code) else {
            return;
        };
        if btn == Btn::Lid {
            if pressed {
                self.lid_closed = !self.lid_closed;
                self.pending.push(if self.lid_closed {
                    RawEvent::Down(Btn::Lid)
                } else {
                    RawEvent::Up(Btn::Lid)
                });
            }
            return;
        }
        match pressed {
            true if !self.held_keys.contains(&btn) => self.held_keys.push(btn),
            true => {}
            false => self.held_keys.retain(|b| *b != btn),
        }
        self.push(btn, pressed);
    }

    fn push(&mut self, btn: Btn, pressed: bool) {
        if pressed {
            crate::latency::read();
        }
        self.pending.push(if pressed {
            RawEvent::Down(btn)
        } else {
            RawEvent::Up(btn)
        });
    }

    fn drain_pad(&mut self) {
        let Some(pad) = self.pad.as_mut() else {
            return;
        };
        let mut edges = Vec::new();
        let mut axes = Vec::new();
        while let Some(ev) = pad.next_event() {
            match ev.event {
                EventType::ButtonPressed(b, _) => edges.push((b, true)),
                EventType::ButtonReleased(b, _) => edges.push((b, false)),
                EventType::AxisChanged(axis, val, _) => {
                    let mapped = match axis {
                        Axis::LeftStickX => Some((StickAxis::LeftX, val)),
                        Axis::LeftStickY => Some((StickAxis::LeftY, -val)),
                        Axis::RightStickX => Some((StickAxis::RightX, val)),
                        Axis::RightStickY => Some((StickAxis::RightY, -val)),
                        _ => None,
                    };
                    if let Some((a, v)) = mapped {
                        let scaled = (v.clamp(-1.0, 1.0) * 32767.0).round() as i16;
                        let snapped = if scaled.abs() < 1600 { 0 } else { scaled };
                        axes.push((a, snapped));
                    }
                }
                _ => {}
            }
        }
        for (b, pressed) in edges {
            if let Some(btn) = pad_to_btn(b) {
                self.push(btn, pressed);
            }
        }
        for (axis, val) in axes {
            self.pending.push(RawEvent::Stick(axis, val));
        }
    }
}

impl Default for HostInput {
    fn default() -> Self {
        HostInput::new()
    }
}

impl InputSource for HostInput {
    fn poll(&mut self, _now: Millis) -> Vec<RawEvent> {
        self.drain_pad();
        std::mem::take(&mut self.pending)
    }
}
