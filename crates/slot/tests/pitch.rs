// Ported from marghost/slot_modded@6dd84fd, crates/slot/tests/pitch.rs.

use slot::pitch::PitchStretch;

const RATE: f64 = 32_768.0;
const PRESENT: usize = 546;

fn tone(from: usize, frames: usize) -> Vec<i16> {
    (from..from + frames)
        .flat_map(|i| {
            let v = (8000.0 * (std::f64::consts::TAU * 440.0 * i as f64 / RATE).sin()) as i16;
            [v, v / 2]
        })
        .collect()
}

#[test]
fn fast_forward_keeps_the_tone_and_plays_in_real_time() {
    for speed in [2, 3, 4, 6] {
        let mut stretch = PitchStretch::default();
        let mut wave = Vec::new();
        let mut batch = Vec::new();
        for present in 0..60 {
            let src = tone(present * PRESENT * speed, PRESENT * speed);
            stretch.process(&src, speed as f64, &mut batch);
            assert!(
                stretch.pending_frames() < 4096,
                "{speed}x kept too much audio"
            );
            assert!(
                batch
                    .chunks_exact(2)
                    .all(|p| (i32::from(p[0]) - 2 * i32::from(p[1])).abs() <= 3),
                "{speed}x changed the stereo balance"
            );
            wave.extend_from_slice(&batch);
        }
        let frames = wave.len() / 2;
        assert!(
            (frames as isize - 60 * PRESENT as isize).abs() < 600,
            "{speed}x produced {frames} frames"
        );
        let skip = 2000.min(frames / 4);
        let left: Vec<i16> = wave[skip * 2..].chunks_exact(2).map(|p| p[0]).collect();
        let crossings = left.windows(2).filter(|p| p[0] <= 0 && p[1] > 0).count();
        let hz = crossings as f64 * RATE / left.len() as f64;
        assert!(
            (hz - 440.0).abs() < 35.0,
            "{speed}x moved the tone to {hz:.1} Hz"
        );
    }
}

#[test]
fn a_changing_speed_does_not_pile_up_audio() {
    let mut stretch = PitchStretch::default();
    let mut out = Vec::new();
    let mut total = 0;
    for present in 0..120 {
        let speed = [2, 4, 6, 1][present % 4];
        stretch.process(&vec![0; PRESENT * speed * 2], speed as f64, &mut out);
        total += out.len() / 2;
        assert!(stretch.pending_frames() < 4096);
    }
    assert!(
        (total as isize - 120 * PRESENT as isize).abs() < 1200,
        "{total} output frames"
    );
}

#[test]
fn a_fractional_speed_plays_in_real_time() {
    let mut stretch = PitchStretch::default();
    let mut out = Vec::new();
    let mut total = 0;
    for _ in 0..60 {
        stretch.process(&vec![0; PRESENT * 5 * 2], 2.5, &mut out);
        total += out.len() / 2;
    }
    assert!(
        (total as isize - 60 * PRESENT as isize * 2).abs() < 1200,
        "{total} output frames"
    );
}

#[test]
fn reset_drops_waiting_audio() {
    let mut stretch = PitchStretch::default();
    let mut out = Vec::new();
    stretch.process(&vec![123; 1024], 2.0, &mut out);
    stretch.reset();
    stretch.process(&vec![0; 1024], 2.0, &mut out);
    assert!(out.iter().all(|&s| s == 0));
}
