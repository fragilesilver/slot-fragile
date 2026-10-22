use crate::lcd3x::ScreenEffect;
use crate::power::{screen_brightness, screen_rect_above};
use crate::quad::Quad;
use crate::retroshader::RetroShader;
use crate::shaders::{GAME_FRAG, RECT_VERT};
use crate::surface::{GfxError, OUT_H, OUT_W};

pub const SCALE: u32 = 3;
pub const SRC_W: u32 = OUT_W / SCALE;
pub const SRC_H: u32 = OUT_H / SCALE;

pub const WHOLE_TEXTURE: [f32; 4] = [0.0, 0.0, 1.0, 1.0];

pub struct GamePass {
    prog: gl::types::GLuint,
    game: gl::types::GLuint,
    u_rect: gl::types::GLint,
    u_bright: gl::types::GLint,
    u_uv: gl::types::GLint,
    u_mode: gl::types::GLint,
    u_paper_size: gl::types::GLint,
    u_fbo: gl::types::GLint,
    fbo_scale: [f32; 2],
    u_origin: gl::types::GLint,
    origin: [f32; 2],
    u_pic: gl::types::GLint,
    pic: [f32; 4],
    paper: gl::types::GLuint,
    paper_size: f32,
    effect: ScreenEffect,
    power: f32,
    fills: bool,
    /// Pixels kept for the carousel footer. The picture scales in the space above them.
    bottom: f32,
    src: [f32; 4],
    /// The picture's own part of the texture, when it is drawn at a whole multiple: the quad
    /// fills a viewport cut to fit it, so nothing else is sampled and no footer squeezes it.
    integer: Option<[f32; 4]>,
    /// A shader off the card, drawn instead of the built-in effect while it is set.
    card: Option<RetroShader>,
    /// The frame before this one, for card shaders that blend with it (`PrevTexture`).
    prev: gl::types::GLuint,
    frames: u32,
    // --- extras ---
    /// The game texture's size: the Game Boy family's 240x160 canvas, or a native system's
    /// picture as its core draws it.
    size: (u32, u32),
    u_src: gl::types::GLint,
    /// The shape a native system's picture is shown at, letterboxed in the game's rect.
    aspect: Option<f32>,
    /// A native system's picture at the largest whole multiple of its own size, square pixels.
    whole: bool,
}

impl GamePass {
    pub fn new() -> Result<Self, GfxError> {
        let prog = crate::shaders::program(RECT_VERT, GAME_FRAG)?;
        let game = crate::gl::texture(SRC_W, SRC_H, gl::NEAREST, gl::CLAMP_TO_EDGE, gl::BGRA, None);
        let prev = crate::gl::texture(SRC_W, SRC_H, gl::NEAREST, gl::CLAMP_TO_EDGE, gl::BGRA, None);
        let paper = crate::gl::texture(
            1,
            1,
            gl::NEAREST,
            gl::REPEAT,
            crate::gl::gray_format(),
            Some(&[255]),
        );
        let (u_rect, u_bright, u_uv, u_mode, u_paper_size, u_pic, u_fbo, u_origin);
        let u_src = crate::gl::uniform_location(prog, "u_src");
        unsafe {
            gl::UseProgram(prog);
            gl::Uniform1i(crate::gl::uniform_location(prog, "u_game"), 0);
            gl::Uniform2f(u_src, SRC_W as f32, SRC_H as f32);
            gl::Uniform2f(
                crate::gl::uniform_location(prog, "u_target"),
                OUT_W as f32,
                OUT_H as f32,
            );
            u_rect = crate::gl::uniform_location(prog, "u_rect");
            u_bright = crate::gl::uniform_location(prog, "u_bright");
            u_uv = crate::gl::uniform_location(prog, "u_uv");
            gl::Uniform1i(crate::gl::uniform_location(prog, "u_paper"), 1);
            u_mode = crate::gl::uniform_location(prog, "u_mode");
            u_paper_size = crate::gl::uniform_location(prog, "u_paper_size");
            u_fbo = crate::gl::uniform_location(prog, "u_fbo");
            u_pic = crate::gl::uniform_location(prog, "u_pic");
            u_origin = crate::gl::uniform_location(prog, "u_origin");
        }
        Ok(GamePass {
            prog,
            game,
            u_rect,
            u_bright,
            u_uv,
            u_mode,
            u_paper_size,
            u_fbo,
            fbo_scale: [1.0, 1.0],
            u_origin,
            origin: [0.0, 0.0],
            u_pic,
            pic: WHOLE_TEXTURE,
            paper,
            paper_size: 1.0,
            effect: ScreenEffect::Lcd3x,
            power: 1.0,
            fills: false,
            bottom: 0.0,
            src: WHOLE_TEXTURE,
            integer: None,
            card: None,
            prev,
            frames: 0,
            size: (SRC_W, SRC_H),
            u_src,
            aspect: None,
            whole: false,
        })
    }

    pub fn set_power(&mut self, t: f32) {
        self.power = t.clamp(0.0, 1.0);
    }

    pub fn set_bottom(&mut self, px: f32) {
        self.bottom = px.max(0.0);
    }

    pub fn frame(&self) -> (f32, f32, f32, f32) {
        self.placed()
    }

    fn placed(&self) -> (f32, f32, f32, f32) {
        let foot = self.bottom;
        let rect = match self.fills {
            true => (0.0, 0.0, OUT_W as f32, OUT_H as f32 - foot),
            false => screen_rect_above(self.power, foot),
        };
        match self.aspect {
            Some(aspect) => match self.whole {
                true => whole_multiple(rect, self.size, self.fbo_scale)
                    .unwrap_or_else(|| letterbox(rect, aspect, self.fbo_scale)),
                false => letterbox(rect, aspect, self.fbo_scale),
            },
            None => rect,
        }
    }

    /// Draws a native system's picture at a whole multiple of its size, or letterboxed.
    pub fn set_whole(&mut self, on: bool) {
        self.whole = on;
    }

    /// Shows a native system's picture at this shape, or the Game Boy family's way for `None`.
    pub fn set_aspect(&mut self, aspect: Option<f32>) {
        self.aspect = aspect.filter(|a| a.is_finite() && *a > 0.0);
    }

    pub fn texture_size(&self) -> (u32, u32) {
        self.size
    }

    /// A frame of its own size: the texture is made again when the size changes.
    pub fn upload_sized(&mut self, xrgb8888: &[u8], size: (u32, u32)) {
        let (w, h) = size;
        if w == 0 || h == 0 || xrgb8888.len() < (w * h * 4) as usize {
            return;
        }
        if size != self.size {
            let made = || crate::gl::texture(w, h, gl::NEAREST, gl::CLAMP_TO_EDGE, gl::BGRA, None);
            let (game, prev) = (made(), made());
            unsafe {
                gl::DeleteTextures(1, &self.game);
                gl::DeleteTextures(1, &self.prev);
                gl::UseProgram(self.prog);
                gl::Uniform2f(self.u_src, w as f32, h as f32);
            }
            (self.game, self.prev, self.size) = (game, prev, size);
        }
        self.put(xrgb8888);
    }

    pub fn set_fills_frame(&mut self, on: bool) {
        self.fills = on;
    }

    pub fn set_picture(&mut self, rect: [f32; 4]) {
        self.pic = rect;
    }

    pub fn set_fbo_scale(&mut self, x: f32, y: f32) {
        self.fbo_scale = [x, y];
    }

    pub fn set_origin(&mut self, x: f32, y: f32) {
        self.origin = [x, y];
    }

    pub fn set_effect(&mut self, effect: ScreenEffect) {
        self.effect = effect;
    }

    pub fn set_paper(&mut self, size: u32, gray: &[u8]) {
        let tex = crate::gl::texture(
            size,
            size,
            gl::NEAREST,
            gl::REPEAT,
            crate::gl::gray_format(),
            Some(gray),
        );
        unsafe { gl::DeleteTextures(1, &self.paper) };
        self.paper = tex;
        self.paper_size = size as f32;
    }

    pub fn set_source_rect(&mut self, rect: [f32; 4]) {
        self.src = rect;
    }

    /// Draws only a `picture`-sized part of the texture, centred in it, or `None` for the band.
    pub fn set_integer(&mut self, picture: Option<(u32, u32)>) {
        self.integer = picture.map(|(w, h)| {
            let (sw, sh) = (SRC_W as f32, SRC_H as f32);
            let (w, h) = (w.min(SRC_W) as f32, h.min(SRC_H) as f32);
            [(sw - w) / 2.0 / sw, (sh - h) / 2.0 / sh, w / sw, h / sh]
        });
    }

    pub fn set_card_shader(&mut self, shader: Option<RetroShader>) {
        self.card = shader;
    }

    pub fn card_shader(&self) -> bool {
        self.card.is_some()
    }

    pub fn upload(&mut self, xrgb8888: &[u8]) {
        self.upload_sized(xrgb8888, (SRC_W, SRC_H));
    }

    fn put(&mut self, xrgb8888: &[u8]) {
        // Last frame becomes `prev`, and its texture takes this one.
        std::mem::swap(&mut self.game, &mut self.prev);
        self.frames = self.frames.wrapping_add(1);
        unsafe {
            gl::BindTexture(gl::TEXTURE_2D, self.game);
            gl::PixelStorei(gl::UNPACK_ALIGNMENT, 1);
            gl::TexSubImage2D(
                gl::TEXTURE_2D,
                0,
                0,
                0,
                self.size.0 as i32,
                self.size.1 as i32,
                gl::BGRA,
                gl::UNSIGNED_BYTE,
                xrgb8888.as_ptr() as *const std::ffi::c_void,
            );
        }
    }

    pub fn draw(&self, quad: &Quad) {
        if let Some(card) = &self.card {
            let (rect, src) = match self.integer {
                Some(src) => ((0.0, 0.0, OUT_W as f32, OUT_H as f32), src),
                None => (self.placed(), self.footed(self.src)),
            };
            return card.draw(self.game, self.prev, rect, src, self.frames, self.size);
        }
        if let Some(src) = self.integer {
            return self.draw_whole(self.game, quad, src);
        }
        let src = match self.effect {
            ScreenEffect::Grid if self.aspect.is_none() => {
                whole_scale_source(self.src, self.fbo_scale)
            }
            _ => self.src,
        };
        self.draw_source(self.game, quad, src);
    }

    pub fn draw_still(&self, tex: gl::types::GLuint, quad: &Quad) {
        self.draw_source(tex, quad, WHOLE_TEXTURE);
    }

    /// The picture alone across the whole viewport, at full brightness once the screen is on.
    fn draw_whole(&self, tex: gl::types::GLuint, quad: &Quad, src: [f32; 4]) {
        let whole = [src[0], src[1], src[0] + src[2], src[1] + src[3]];
        self.draw_at(
            tex,
            quad,
            (0.0, 0.0, OUT_W as f32, OUT_H as f32),
            src,
            screen_brightness(self.power),
            whole,
        );
    }

    fn draw_source(&self, tex: gl::types::GLuint, quad: &Quad, src: [f32; 4]) {
        let (x, y, w, h) = self.placed();
        let bright = if self.fills {
            1.0
        } else {
            screen_brightness(self.power)
        };
        let src = self.footed(src);
        self.draw_at(tex, quad, (x, y, w, h), src, bright, self.pic);
    }

    /// The footer covers the bottom of a full-frame picture. Keep the part that sat above it,
    /// so opening the scale does not squash the picture into the shorter rect.
    fn footed(&self, src: [f32; 4]) -> [f32; 4] {
        let mut src = src;
        if self.bottom > 0.0 {
            src[3] *= (OUT_H as f32 - self.bottom) / OUT_H as f32;
        }
        src
    }

    fn draw_at(
        &self,
        tex: gl::types::GLuint,
        quad: &Quad,
        (x, y, w, h): (f32, f32, f32, f32),
        src: [f32; 4],
        bright: f32,
        pic: [f32; 4],
    ) {
        unsafe {
            gl::UseProgram(self.prog);
            gl::Uniform4f(self.u_rect, x, y, w, h);
            gl::Uniform4f(self.u_uv, src[0], src[1], src[2], src[3]);
            gl::Uniform1f(self.u_bright, bright);
            gl::Uniform1f(self.u_mode, self.effect.mode());
            gl::Uniform1f(self.u_paper_size, self.paper_size);
            gl::Uniform2f(self.u_fbo, self.fbo_scale[0], self.fbo_scale[1]);
            gl::Uniform2f(self.u_origin, self.origin[0], self.origin[1]);
            gl::Uniform4f(self.u_pic, pic[0], pic[1], pic[2], pic[3]);
            gl::ActiveTexture(gl::TEXTURE1);
            gl::BindTexture(gl::TEXTURE_2D, self.paper);
            gl::ActiveTexture(gl::TEXTURE0);
            gl::BindTexture(gl::TEXTURE_2D, tex);
        }
        quad.draw();
    }
}

/// The largest rect of `aspect` (as seen, after the viewport's own scale) centred in `rect`.
fn letterbox(rect: (f32, f32, f32, f32), aspect: f32, fbo: [f32; 2]) -> (f32, f32, f32, f32) {
    let (x, y, w, h) = rect;
    if w <= 0.0 || h <= 0.0 {
        return rect;
    }
    let seen = (w * fbo[0]) / (h * fbo[1]);
    let (nw, nh) = match seen > aspect {
        true => (w * aspect / seen, h),
        false => (w, h * seen / aspect),
    };
    (x + (w - nw) / 2.0, y + (h - nh) / 2.0, nw, nh)
}

/// The largest whole multiple of `size` that fits `rect`, centred, counted in the target's own
/// pixels; None when not even one fits.
fn whole_multiple(
    rect: (f32, f32, f32, f32),
    size: (u32, u32),
    fbo: [f32; 2],
) -> Option<(f32, f32, f32, f32)> {
    let (x, y, w, h) = rect;
    let (sw, sh) = (size.0.max(1) as f32, size.1.max(1) as f32);
    let k = ((w * fbo[0]) / sw).min((h * fbo[1]) / sh).floor();
    if k < 1.0 {
        return None;
    }
    let (nw, nh) = (sw * k / fbo[0], sh * k / fbo[1]);
    Some((x + (w - nw) / 2.0, y + (h - nh) / 2.0, nw, nh))
}

fn whole_scale_source(src: [f32; 4], fbo: [f32; 2]) -> [f32; 4] {
    let (x, w) = whole_scale(src[0], src[2], SRC_W as f32, OUT_W as f32 * fbo[0]);
    let (y, h) = whole_scale(src[1], src[3], SRC_H as f32, OUT_H as f32 * fbo[1]);
    [x, y, w, h]
}

fn whole_scale(at: f32, len: f32, texels: f32, out: f32) -> (f32, f32) {
    let shown = len * texels;
    let scale = (out / shown).ceil();
    let trim = scale * shown - out;
    if trim >= 0.1 * shown {
        return (at, len);
    }
    let top = (trim / 2.0).floor();
    (at + top / scale / texels, out / scale / texels)
}

impl Drop for GamePass {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteTextures(1, &self.game);
            gl::DeleteTextures(1, &self.prev);
            gl::DeleteTextures(1, &self.paper);
            gl::DeleteProgram(self.prog);
        }
    }
}

#[cfg(test)]
mod whole_tests {
    use super::*;

    #[test]
    fn an_nes_picture_sits_at_twice_its_size_in_the_middle_of_640_by_480() {
        let r = whole_multiple((0.0, 0.0, 640.0, 480.0), (256, 224), [1.0, 1.0]).unwrap();
        assert_eq!(r, (64.0, 16.0, 512.0, 448.0));
    }

    #[test]
    fn a_picture_too_big_to_fit_once_is_left_to_the_letterbox() {
        assert!(whole_multiple((0.0, 0.0, 640.0, 440.0), (640, 480), [1.0, 1.0]).is_none());
    }

    #[test]
    fn the_multiple_is_counted_in_the_targets_own_pixels() {
        let r = whole_multiple((0.0, 0.0, 640.0, 480.0), (256, 224), [2.0, 2.0]).unwrap();
        assert_eq!(
            r,
            (64.0, 16.0, 512.0, 448.0),
            "4x of 256x224 in 1280x960 pixels"
        );
    }
}
