// Ported from marghost/slot_modded@6dd84fd, crates/slot/src/pitch.rs: fast forward played at
// its normal pitch, by overlapping and cross-fading hops of the source (WSOLA).

const HOP: usize = 256;
const SEARCH: usize = 64;

#[derive(Default)]
pub struct PitchStretch {
    source: Vec<i16>,
    tail: Vec<i16>,
    next: f64,
    speeds: Vec<(usize, f64)>,
}

impl PitchStretch {
    pub fn reset(&mut self) {
        self.source.clear();
        self.tail.clear();
        self.next = 0.0;
        self.speeds.clear();
    }

    pub fn pending_frames(&self) -> usize {
        self.source.len() / 2
    }

    fn source_position(&self, time: f64) -> Option<f64> {
        let mut start = 0;
        let mut elapsed = 0.0;
        for &(end, speed) in &self.speeds {
            let duration = (end - start) as f64 / speed;
            if time <= elapsed + duration {
                return Some(start as f64 + (time - elapsed) * speed);
            }
            elapsed += duration;
            start = end;
        }
        None
    }

    pub fn process(&mut self, input: &[i16], speed: f64, out: &mut Vec<i16>) {
        out.clear();
        let speed = speed.max(1.0);
        self.source.extend_from_slice(input);
        let frames = self.source.len() / 2;
        match self.speeds.last_mut() {
            Some(last) if last.1 == speed => last.0 = frames,
            _ if !input.is_empty() => self.speeds.push((frames, speed)),
            _ => {}
        }
        if self.tail.is_empty() {
            if frames < HOP * 2 {
                return;
            }
            out.extend_from_slice(&self.source[..HOP * 2]);
            self.tail.extend_from_slice(&self.source[HOP * 2..HOP * 4]);
            self.next = HOP as f64;
        }
        while let Some(position) = self.source_position(self.next) {
            let centre = position.round() as usize;
            if centre + HOP * 2 > frames {
                break;
            }
            let best = self.best_match(centre, frames);
            for i in 0..HOP {
                let fade = i as f32 / HOP as f32;
                for channel in 0..2 {
                    let old = f32::from(self.tail[i * 2 + channel]);
                    let new = f32::from(self.source[(best + i) * 2 + channel]);
                    out.push((old * (1.0 - fade) + new * fade).round() as i16);
                }
            }
            self.tail
                .copy_from_slice(&self.source[(best + HOP) * 2..(best + HOP * 2) * 2]);
            self.next += HOP as f64;
        }
        self.drop_consumed(frames);
    }

    fn best_match(&self, centre: usize, frames: usize) -> usize {
        let lower = centre.saturating_sub(SEARCH);
        let upper = (centre + SEARCH).min(frames - HOP * 2);
        let mut best = centre.min(upper);
        let mut score = f64::NEG_INFINITY;
        for at in (lower..=upper).step_by(2) {
            let mut dot = 0.0;
            let mut energy = 0.0;
            for i in (0..HOP).step_by(2) {
                let a = (f64::from(self.tail[i * 2]) + f64::from(self.tail[i * 2 + 1])) * 0.5;
                let b = (f64::from(self.source[(at + i) * 2])
                    + f64::from(self.source[(at + i) * 2 + 1]))
                    * 0.5;
                dot += a * b;
                energy += b * b;
            }
            let similarity = dot / energy.max(1.0).sqrt();
            if similarity > score {
                score = similarity;
                best = at;
            }
        }
        best
    }

    fn drop_consumed(&mut self, frames: usize) {
        let consumed = (self.source_position(self.next).unwrap_or(frames as f64) as usize)
            .saturating_sub(SEARCH)
            .min(frames.saturating_sub(SEARCH));
        self.source.drain(..consumed * 2);
        let mut remaining = consumed;
        let mut elapsed = 0.0;
        let mut start = 0;
        for &(end, speed) in &self.speeds {
            let taken = remaining.min(end - start);
            elapsed += taken as f64 / speed;
            remaining -= taken;
            start = end;
            if remaining == 0 {
                break;
            }
        }
        self.next -= elapsed;
        self.speeds.retain_mut(|(end, _)| {
            *end = end.saturating_sub(consumed);
            *end != 0
        });
    }
}
