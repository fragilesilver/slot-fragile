const WARMEST: [f32; 3] = [1.0, 0.82, 0.62];

pub const BLUE_LIGHT_MAX: u8 = 9;
pub const SATURATION_MAX: u8 = 9;
pub const CONTRAST_MAX: u8 = 9;
pub const DIM_MAX: u8 = 6;

const DIM_GAINS: [f32; 7] = [1.0, 0.8, 0.65, 0.52, 0.42, 0.33, 0.25];

const SATURATION_TOP: f32 = 0.6;
const CONTRAST_TOP: f32 = 0.2;

pub fn dim_gain(step: u8) -> f32 {
    DIM_GAINS[usize::from(step).min(DIM_GAINS.len() - 1)]
}

/// Pure-shader warmth and green/magenta tint gains in `[-100, 100]` (neutral at `0, 0`).
pub fn warmth_tint_gain(warm: i8, tint: i8) -> [f32; 3] {
    let w = (warm.clamp(-100, 100) as f32) / 100.0;
    let t = (tint.clamp(-100, 100) as f32) / 100.0;
    let mut g = [
        1.0 + w.min(0.0) * 0.4,
        1.0 - w.abs() * 0.12,
        1.0 - w.max(0.0) * 0.4,
    ];
    if t > 0.0 {
        g[1] *= 1.0 - t * 0.25;
    } else {
        g[0] *= 1.0 + t * 0.25;
        g[2] *= 1.0 + t * 0.25;
    }
    let top = g.iter().copied().fold(0.0f32, f32::max).max(1e-4);
    g.map(|v| v / top)
}

/// Pure-shader gamma curve exponent from integer slider `[-50, 50]` (`0` => `1.0`).
pub fn gamma_curve(curve: i8) -> f32 {
    2f32.powf((curve.clamp(-50, 50) as f32) / 100.0)
}

pub fn saturation_boost(step: u8) -> f32 {
    SATURATION_TOP * step.min(SATURATION_MAX) as f32 / SATURATION_MAX as f32
}

pub fn contrast_boost(step: u8) -> f32 {
    CONTRAST_TOP * step.min(CONTRAST_MAX) as f32 / CONTRAST_MAX as f32
}

pub fn blue_light_gain(step: u8) -> [f32; 3] {
    let t = step.min(BLUE_LIGHT_MAX) as f32 / BLUE_LIGHT_MAX as f32;
    let mut gain = [1.0f32; 3];
    for (g, warm) in gain.iter_mut().zip(WARMEST) {
        *g = 1.0 + (warm - 1.0) * t;
    }
    gain
}
