use crate::surface::GfxError;

const VERT_PREAMBLE: &str = "#version 330 core\n#define attribute in\n#define varying out\n";

const FRAG_PREAMBLE: &str = "#version 330 core\n#define varying in\n\
                             #define texture2D texture\nout vec4 FRAG_COLOR;\n";

const VERT_PREAMBLE_ES: &str = "";
const FRAG_PREAMBLE_ES: &str = "#define FRAG_COLOR gl_FragColor\n";

pub fn program(vert: &str, frag: &str) -> Result<gl::types::GLuint, GfxError> {
    let (vp, fp) = match crate::gl::es() {
        true => (VERT_PREAMBLE_ES, FRAG_PREAMBLE_ES),
        false => (VERT_PREAMBLE, FRAG_PREAMBLE),
    };
    crate::gl::program(&format!("{vp}{vert}"), &format!("{fp}{frag}"))
}

pub const RECT_VERT: &str = r#"
attribute vec2 a_pos;
uniform vec4 u_rect;
uniform vec2 u_target;
varying vec2 v_uv;
void main() {
    v_uv = a_pos;
    vec2 p = (u_rect.xy + a_pos * u_rect.zw) / u_target;
    gl_Position = vec4(p.x * 2.0 - 1.0, 1.0 - p.y * 2.0, 0.0, 1.0);
}
"#;

pub const SPRITE_VERT: &str = r#"
attribute vec2 a_pos;
uniform vec4 u_rect;
uniform vec2 u_target;
uniform vec2 u_turn;
uniform vec4 u_src;
varying vec2 v_uv;
void main() {
    v_uv = u_src.xy + a_pos * u_src.zw;
    vec2 mid = u_rect.zw * 0.5;
    vec2 local = a_pos * u_rect.zw - mid;
    vec2 turned = vec2(u_turn.x * local.x - u_turn.y * local.y,
                       u_turn.y * local.x + u_turn.x * local.y);
    vec2 p = (u_rect.xy + a_pos * u_rect.zw + (turned - local)) / u_target;
    gl_Position = vec4(p.x * 2.0 - 1.0, 1.0 - p.y * 2.0, 0.0, 1.0);
}
"#;

pub const GAME_FRAG: &str = r#"
#ifdef GL_FRAGMENT_PRECISION_HIGH
precision highp float;
#else
precision mediump float;
#endif
uniform sampler2D u_game;
uniform sampler2D u_paper;
uniform vec2 u_src;
uniform vec4 u_uv;
uniform float u_bright;
uniform float u_mode;
uniform float u_paper_size;
uniform vec2 u_origin;
uniform vec4 u_rect;
uniform vec2 u_fbo;
uniform vec4 u_pic;
uniform float u_sharp;
varying vec2 v_uv;
const vec3 offsets = vec3(3.141592654) * vec3(1.0 / 2.0, 1.0 / 2.0 - 2.0 / 3.0, 1.0 / 2.0 - 4.0 / 3.0);
const vec3 luma_coeff = vec3(0.2126, 0.7152, 0.0722);
vec2 grid_band(vec2 x) {
    return floor(x) / 3.0 + min(fract(x), vec2(1.0 / 3.0));
}
float dot_weight(vec2 f, vec2 o, float bloom) {
    return exp(-4.0 * length(f - (o + vec2(0.5))) * bloom);
}
const vec3 lcd_centres = vec3(1.0 / 6.0, 0.5, 5.0 / 6.0);
const float lcd_stripe = 0.9;
const float lcd_gap_y = 0.35;
vec3 lcd_ramp(vec3 x) {
    vec3 u = fract(x + 0.5) - 0.5;
    return u - 2.0 * u * abs(u);
}
void main() {
    vec2 uv = u_uv.xy + v_uv * u_uv.zw;
    vec2 f = fract(uv * u_src);
    vec2 px = max((u_rect.zw * u_fbo) / max(u_uv.zw * u_src, vec2(1.0)), vec2(1.0));
    vec2 texel = uv * u_src;
    vec2 from_centre = fract(texel) - 0.5;
    vec2 region = 0.5 - 0.5 / px;
    vec2 snap = (from_centre - clamp(from_centre, -region, region)) * px + 0.5;
    vec2 sharp_uv = (floor(texel) + snap) / u_src;
    vec2 sample_uv = mix(uv, sharp_uv, u_sharp);
    vec3 c = texture2D(u_game, sample_uv).rgb;
    bool inside = all(greaterThanEqual(uv, u_pic.xy)) && all(lessThan(uv, u_pic.zw));
    if (!inside) {
    } else if (u_mode > 6.5) {
        vec2 hp = 0.5 * u_uv.zw * u_src / (u_rect.zw * u_fbo);
        vec2 p = uv * u_src;
        vec2 b = floor(p + hp);
        vec2 t = clamp((p + hp - b) / (2.0 * hp), 0.0, 1.0);
        vec2 lo = u_pic.xy * u_src + 0.5;
        vec2 hi = u_pic.zw * u_src - 0.5;
        vec2 a0 = clamp(b - 0.5, lo, hi) / u_src;
        vec2 a1 = clamp(b + 0.5, lo, hi) / u_src;
        vec3 top = mix(texture2D(u_game, a0).rgb, texture2D(u_game, vec2(a1.x, a0.y)).rgb, t.x);
        vec3 bottom = mix(texture2D(u_game, vec2(a0.x, a1.y)).rgb, texture2D(u_game, a1).rgb, t.x);
        c = mix(top, bottom, t.y);
        vec2 edge = (grid_band(f + hp) - grid_band(f - hp)) / (2.0 * hp);
        c *= 1.0 - 0.75 * max(edge.x, edge.y);
    } else if (u_mode > 5.5) {
        vec2 hp = 0.5 * u_uv.zw * u_src / (u_rect.zw * u_fbo);
        vec2 p = uv * u_src;
        vec2 b = floor(p + hp);
        vec2 l0 = min(p - hp, b) - b + 1.0;
        vec2 r0 = max(p - hp, b) - b;
        vec2 r1 = p + hp - b;
        mediump vec3 gl0 = lcd_ramp(l0.x - lcd_centres);
        mediump vec3 gr0 = lcd_ramp(r0.x - lcd_centres);
        mediump vec3 gr1 = lcd_ramp(r1.x - lcd_centres);
        mediump vec3 gy = lcd_ramp(vec3(l0.y, r0.y, r1.y) - 0.5);
        mediump vec3 wl = vec3(1.0 - l0.x) + lcd_stripe * (lcd_ramp(-lcd_centres) - gl0);
        mediump vec3 wr = vec3(r1.x - r0.x) + lcd_stripe * (gr1 - gr0);
        mediump float wt = (1.0 - l0.y) - lcd_gap_y * gy.x;
        mediump float wb = (r1.y - r0.y) + lcd_gap_y * (gy.z - gy.y);
        vec2 lo = u_pic.xy * u_src + 0.5;
        vec2 hi = u_pic.zw * u_src - 0.5;
        vec2 a0 = clamp(b - 0.5, lo, hi) / u_src;
        vec2 a1 = clamp(b + 0.5, lo, hi) / u_src;
        mediump vec3 c00 = texture2D(u_game, a0).rgb;
        mediump vec3 c10 = texture2D(u_game, vec2(a1.x, a0.y)).rgb;
        mediump vec3 c01 = texture2D(u_game, vec2(a0.x, a1.y)).rgb;
        mediump vec3 c11 = texture2D(u_game, a1).rgb;
        mediump vec3 lit = (c00 * c00 * wl + c10 * c10 * wr) * wt + (c01 * c01 * wl + c11 * c11 * wr) * wb;
        c = sqrt(clamp(lit / (4.0 * hp.x * hp.y), 0.0, 1.0));
    } else if (u_mode > 4.5) {
        vec2 hp = 0.5 * u_uv.zw * u_src / (u_rect.zw * u_fbo);
        vec2 p = uv * u_src;
        vec2 b = floor(p + hp);
        vec2 t = clamp((p + hp - b) / (2.0 * hp), 0.0, 1.0);
        vec2 lo = u_pic.xy * u_src + 0.5;
        vec2 hi = u_pic.zw * u_src - 0.5;
        vec2 a0 = clamp(b - 0.5, lo, hi) / u_src;
        vec2 a1 = clamp(b + 0.5, lo, hi) / u_src;
        vec3 top = mix(texture2D(u_game, a0).rgb, texture2D(u_game, vec2(a1.x, a0.y)).rgb, t.x);
        vec3 bottom = mix(texture2D(u_game, vec2(a0.x, a1.y)).rgb, texture2D(u_game, a1).rgb, t.x);
        c = mix(top, bottom, t.y);
    } else if (u_mode > 3.5) {
        c = pow(c, vec3(1.05));
        vec2 d = abs(f - 0.5);
        float x2 = max(d.x, d.y);
        x2 = x2 * x2;
        float x4 = x2 * x2;
        float w = 48.0 * (x4 - (8.0 / 3.0) * x4 * x2);
        w = w * (w + (1.0 - w) * 0.65) * 0.65;
        float luma = dot(c, luma_coeff);
        w = w * luma;
        c = mix(c, vec3(1.0), w);
        vec3 bg = vec3(texture2D(u_paper, (floor(gl_FragCoord.xy - u_origin) + 0.5) / u_paper_size).r) * c;
        c = mix(c, bg, dot(c, luma_coeff));
    } else if (u_mode > 2.5) {
        float bloom = mix(1.05, 0.95, dot(c, vec3(0.30, 0.59, 0.11)));
        float mid = dot_weight(f, vec2(0.0), bloom);
        float sum = 0.0;
        for (int j = -1; j <= 1; j++) {
            for (int i = -1; i <= 1; i++) {
                sum += dot_weight(f, vec2(float(i), float(j)), bloom);
            }
        }
        c = mix(c, mix(1.2 * c * mid, c * sum, 0.85), 0.85);
    } else if (u_mode > 1.5) {
        vec2 hp = 0.5 * u_uv.zw * u_src / (u_rect.zw * u_fbo);
        vec2 edge = (grid_band(f + hp) - grid_band(f - hp)) / (2.0 * hp);
        c *= 1.0 - 0.75 * max(edge.x, edge.y);
    } else if (u_mode > 0.5) {
        vec2 angle = f * 6.283185307;
        float yfactor = (16.0 + sin(angle.y)) / 17.0;
        vec3 xfactors = (4.0 + sin(angle.x + offsets)) / 5.0;
        c *= yfactor * xfactors;
    }
    FRAG_COLOR = vec4(c * u_bright, 1.0);
}
"#;

pub const SPRITE_FRAG: &str = r#"
precision mediump float;
uniform sampler2D u_tex;
uniform vec4 u_colour;
varying vec2 v_uv;
void main() {
    FRAG_COLOR = texture2D(u_tex, v_uv) * u_colour;
}
"#;

pub const BLIT_VERT: &str = r#"
attribute vec2 a_pos;
uniform vec4 u_uv;
varying vec2 v_uv;
void main() {
    v_uv = u_uv.xy + a_pos * u_uv.zw;
    gl_Position = vec4(a_pos * 2.0 - 1.0, 0.0, 1.0);
}
"#;

pub const TURN_VERT: &str = r#"
attribute vec2 a_pos;
uniform float u_ccw;
varying vec2 v_uv;
void main() {
    vec2 cw = vec2(1.0 - a_pos.y, a_pos.x);
    vec2 ccw = vec2(a_pos.y, 1.0 - a_pos.x);
    v_uv = mix(cw, ccw, u_ccw);
    gl_Position = vec4(a_pos * 2.0 - 1.0, 0.0, 1.0);
}
"#;

pub const BLIT_FRAG: &str = r#"
precision mediump float;
uniform sampler2D u_tex;
uniform vec3 u_gain;
uniform float u_dim;
uniform float u_saturation;
uniform float u_contrast;
uniform vec3 u_cal_gain;
uniform float u_cal_gamma;
varying vec2 v_uv;
void main() {
    vec3 c = texture2D(u_tex, v_uv).rgb;
    c = mix(vec3(dot(c, vec3(0.2126, 0.7152, 0.0722))), c, 1.0 + u_saturation);
    c = (c - 0.5) * (1.0 + u_contrast) + 0.5;
    c = clamp(c, 0.0, 1.0);
    vec3 curved = mix(pow(max(c, vec3(1e-4)), vec3(u_cal_gamma)), c, step(abs(u_cal_gamma - 1.0), 0.001));
    FRAG_COLOR = vec4(clamp(curved * u_cal_gain, 0.0, 1.0) * u_gain * u_dim, 1.0);
}
"#;

pub const GHOST_FRAG: &str = r#"
precision mediump float;
uniform sampler2D u_tex;
uniform sampler2D u_prev;
uniform vec2 u_keep;
varying vec2 v_uv;
void main() {
    vec3 now = texture2D(u_tex, v_uv).rgb;
    vec3 was = texture2D(u_prev, v_uv).rgb;
    vec3 keep = mix(vec3(u_keep.y), vec3(u_keep.x), step(was, now));
    vec3 c = mix(now, was, keep);
    c = mix(c, now, step(abs(now - was), vec3(1.5 / 255.0)));
    FRAG_COLOR = vec4(c, 1.0);
}
"#;
