// Ported from markbruno/slot-plus@522fd12, crates/slot-gfx/src/retroshader.rs. Changed: the
// texture coordinates follow the picture's part of the texture, so a card shader draws the
// same Game Boy, Stretch and Integer Scale picture the built-in ones do, and InputSize is that
// picture's size, as RetroArch hands it over.

//! Single-pass RetroArch GLSL shaders (`.glsl`) over the game layer.
//!
//! RetroArch's legacy GLSL format is one file holding both stages, split by
//! `#if defined(VERTEX)` / `#elif defined(FRAGMENT)`, with a compatibility block at the top of
//! each stage that picks `attribute`/`varying`/`texture2D` or `in`/`out`/`texture` from
//! `__VERSION__`. So the same file compiles on the device's GLES 2 driver and on the host's
//! GL 3.3 core context as long as the `#version` line is ours to choose: every `#version` in the
//! file is dropped, the device gets none (100 is the default), the host gets `330 core`.
//!
//! What a shader is handed, by RetroArch's names:
//!
//! - `VertexCoord`: the unit quad. `TexCoord`: the picture's part of the 240x160 frame, (0, 0)
//!   at its top left: the whole frame for a GBA game, the Game Boy's 160x144 in its middle.
//! - `MVPMatrix`: the unit quad to wherever the game layer sits this frame. It is the whole
//!   panel except while the screen is powering on or off.
//! - `Texture` / `OrigTexture` (unit 0): this frame. `PrevTexture` (unit 1): the frame before,
//!   for ghosting and frame blending shaders.
//! - `TextureSize`, `OrigTextureSize`: 240x160. `InputSize`, `OrigInputSize`: the picture's
//!   size in texels, 240x160 or 160x144.
//! - `OutputSize`: the game layer's size, in the 720x480 canvas's pixels.
//! - `FrameCount`, `FrameDirection`: as RetroArch sets them, direction always 1.
//!
//! Parameters (`#pragma parameter`) run at their defaults: `PARAMETER_UNIFORM` is never
//! defined, so each shader's own `#define NAME default` branch is the one compiled.
//!
//! One slot-specific line is understood: `#pragma slot_filter linear` samples the frame with
//! bilinear filtering rather than nearest, which shaders such as sharp-bilinear expect from the
//! `filter_linear` line of their preset. GLSL ignores a pragma it does not know, so the file
//! still loads in RetroArch unchanged.

use std::collections::HashMap;

use crate::pipeline::{SRC_H, SRC_W};
use crate::surface::{GfxError, OUT_H, OUT_W};

const VERTEX_LOC: gl::types::GLuint = 0;
const TEXCOORD_LOC: gl::types::GLuint = 1;

const VERTS: [f32; 8] = [0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 1.0, 1.0];

const LINEAR_PRAGMA: &str = "#pragma slot_filter linear";

/// A uniform the frontend sets, with whether the shader declared it as a float. RetroArch's
/// own shaders declare `FrameCount` as an int, but plenty of hand-written ones say float, and
/// setting either with the wrong call is an error the driver swallows, leaving it at zero.
#[derive(Copy, Clone)]
struct Uniform {
    loc: gl::types::GLint,
    float: bool,
}

impl Uniform {
    const ABSENT: Uniform = Uniform {
        loc: -1,
        float: false,
    };

    unsafe fn set_count(self, n: u32) {
        if self.loc < 0 {
            return;
        }
        match self.float {
            true => gl::Uniform1f(self.loc, n as f32),
            false => gl::Uniform1i(self.loc, n as i32),
        }
    }

    unsafe fn set_vec2(self, x: f32, y: f32) {
        if self.loc >= 0 {
            gl::Uniform2f(self.loc, x, y);
        }
    }
}

pub struct RetroShader {
    prog: gl::types::GLuint,
    vbo: gl::types::GLuint,
    /// The picture's corners in the texture, rewritten when the picture moves.
    tex_vbo: gl::types::GLuint,
    shown: std::cell::Cell<[f32; 4]>,
    /// Desktop GL core cannot draw without one; ES 2.0 has none. Zero on the device.
    vao: gl::types::GLuint,
    linear: bool,
    mvp: gl::types::GLint,
    frame_count: Uniform,
    frame_direction: Uniform,
    output_size: Uniform,
    input_size: [Uniform; 2],
    texture_size: [Uniform; 2],
    /// The frame's texture size, set again when a native system's picture changes size.
    sized: std::cell::Cell<(u32, u32)>,
}

/// The file with its `#version` lines taken out and the stage and version put back in front,
/// in the order GLSL requires: `#version` first, then anything else.
pub fn stage_source(src: &str, stage: &str, es: bool) -> String {
    let body: Vec<&str> = src
        .lines()
        .map(|l| match l.trim_start().starts_with("#version") {
            // Blanked rather than removed, so a compile error still names the right line.
            true => "",
            false => l,
        })
        .collect();
    let version = if es { "" } else { "#version 330 core\n" };
    // `#line 1` puts the line numbers in the driver's log back on the file's own.
    format!("{version}#define {stage}\n#line 1\n{}\n", body.join("\n"))
}

pub fn wants_linear(src: &str) -> bool {
    src.lines().any(|l| l.trim() == LINEAR_PRAGMA)
}

impl RetroShader {
    pub fn new(src: &str) -> Result<Self, GfxError> {
        let es = crate::gl::es();
        let prog = crate::gl::program_with(
            &stage_source(src, "VERTEX", es),
            &stage_source(src, "FRAGMENT", es),
            &[("VertexCoord", VERTEX_LOC), ("TexCoord", TEXCOORD_LOC)],
        )?;
        let kinds = unsafe { uniform_kinds(prog) };
        let uniform = |name: &str| match kinds.get(name) {
            Some(&kind) => Uniform {
                loc: crate::gl::uniform_location(prog, name),
                float: kind == gl::FLOAT,
            },
            None => Uniform::ABSENT,
        };
        let shader = unsafe {
            gl::UseProgram(prog);
            // The frame and its predecessor never change units, so everything but the geometry,
            // the count and a native picture's size is set once here.
            for (name, unit) in [("Texture", 0), ("OrigTexture", 0), ("PrevTexture", 1)] {
                let loc = crate::gl::uniform_location(prog, name);
                if loc >= 0 {
                    gl::Uniform1i(loc, unit);
                }
            }
            for name in ["TextureSize", "OrigTextureSize"] {
                uniform(name).set_vec2(SRC_W as f32, SRC_H as f32);
            }
            let mut vao = 0;
            if !es {
                gl::GenVertexArrays(1, &mut vao);
                gl::BindVertexArray(vao);
            }
            let buffer = |usage| {
                let mut vbo = 0;
                gl::GenBuffers(1, &mut vbo);
                gl::BindBuffer(gl::ARRAY_BUFFER, vbo);
                gl::BufferData(
                    gl::ARRAY_BUFFER,
                    std::mem::size_of_val(&VERTS) as isize,
                    VERTS.as_ptr() as *const std::ffi::c_void,
                    usage,
                );
                vbo
            };
            let (vbo, tex_vbo) = (buffer(gl::STATIC_DRAW), buffer(gl::DYNAMIC_DRAW));
            if !es {
                // Recorded in the VAO once; the device sets the same pointers every draw.
                attribs(vbo, tex_vbo);
                gl::BindVertexArray(0);
            }
            RetroShader {
                prog,
                vbo,
                tex_vbo,
                shown: std::cell::Cell::new([0.0, 0.0, 1.0, 1.0]),
                vao,
                linear: wants_linear(src),
                mvp: crate::gl::uniform_location(prog, "MVPMatrix"),
                frame_count: uniform("FrameCount"),
                frame_direction: uniform("FrameDirection"),
                output_size: uniform("OutputSize"),
                input_size: [uniform("InputSize"), uniform("OrigInputSize")],
                texture_size: [uniform("TextureSize"), uniform("OrigTextureSize")],
                sized: std::cell::Cell::new((SRC_W, SRC_H)),
            }
        };
        Ok(shader)
    }

    /// `rect` is where the game layer goes, in offscreen pixels with the origin top left, as
    /// every pass in this crate thinks of it. `src` is the picture's part of the texture, as
    /// x, y, w, h in texture coordinates.
    pub fn draw(
        &self,
        tex: gl::types::GLuint,
        prev: gl::types::GLuint,
        rect: (f32, f32, f32, f32),
        src: [f32; 4],
        frame: u32,
        size: (u32, u32),
    ) {
        let (x, y, w, h) = rect;
        let (tw, th) = (OUT_W as f32, OUT_H as f32);
        // The unit quad to clip space, y flipped exactly as `RECT_VERT` flips it. Column major,
        // since ES 2.0 refuses a transposed upload.
        let (sx, sy) = (2.0 * w / tw, -2.0 * h / th);
        let (ox, oy) = (2.0 * x / tw - 1.0, 1.0 - 2.0 * y / th);
        let mvp: [f32; 16] = [
            sx, 0.0, 0.0, 0.0, //
            0.0, sy, 0.0, 0.0, //
            0.0, 0.0, 1.0, 0.0, //
            ox, oy, 0.0, 1.0,
        ];
        let filter = match self.linear {
            true => gl::LINEAR,
            false => gl::NEAREST,
        };
        unsafe {
            gl::UseProgram(self.prog);
            if self.mvp >= 0 {
                gl::UniformMatrix4fv(self.mvp, 1, gl::FALSE, mvp.as_ptr());
            }
            self.output_size.set_vec2(w, h);
            let (sw, sh) = (size.0 as f32, size.1 as f32);
            if self.sized.get() != size {
                self.sized.set(size);
                for u in self.texture_size {
                    u.set_vec2(sw, sh);
                }
            }
            for u in self.input_size {
                u.set_vec2(src[2] * sw, src[3] * sh);
            }
            if self.shown.get() != src {
                self.shown.set(src);
                let [x, y, w, h] = src;
                let corners: [f32; 8] = [x, y, x + w, y, x, y + h, x + w, y + h];
                gl::BindBuffer(gl::ARRAY_BUFFER, self.tex_vbo);
                gl::BufferSubData(
                    gl::ARRAY_BUFFER,
                    0,
                    std::mem::size_of_val(&corners) as isize,
                    corners.as_ptr() as *const std::ffi::c_void,
                );
            }
            self.frame_count.set_count(frame);
            self.frame_direction.set_count(1);
            gl::ActiveTexture(gl::TEXTURE1);
            gl::BindTexture(gl::TEXTURE_2D, prev);
            set_filter(filter);
            gl::ActiveTexture(gl::TEXTURE0);
            gl::BindTexture(gl::TEXTURE_2D, tex);
            set_filter(filter);
            if crate::gl::es() {
                attribs(self.vbo, self.tex_vbo);
            } else {
                gl::BindVertexArray(self.vao);
            }
            gl::DrawArrays(gl::TRIANGLE_STRIP, 0, 4);
            if crate::gl::es() {
                // Nothing else in the tree reads location 1, and a stale enabled array is one
                // more thing for the next pass's draw to trip over.
                gl::DisableVertexAttribArray(TEXCOORD_LOC);
            } else {
                gl::BindVertexArray(0);
            }
            // Back to how every other pass expects to find the frame: nearest.
            if self.linear {
                set_filter(gl::NEAREST);
                gl::ActiveTexture(gl::TEXTURE1);
                set_filter(gl::NEAREST);
                gl::ActiveTexture(gl::TEXTURE0);
            }
        }
    }
}

impl Drop for RetroShader {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteBuffers(1, &self.vbo);
            gl::DeleteBuffers(1, &self.tex_vbo);
            if self.vao != 0 {
                gl::DeleteVertexArrays(1, &self.vao);
            }
            gl::DeleteProgram(self.prog);
        }
    }
}

/// The unit quad for `VertexCoord`, the picture's corners for `TexCoord`. Both are vec4s in
/// the shader; a two component array fills the rest with (0, 1), which is exactly the position
/// and texture coordinate RetroArch would have handed over.
unsafe fn attribs(quad: gl::types::GLuint, corners: gl::types::GLuint) {
    for (loc, vbo) in [(VERTEX_LOC, quad), (TEXCOORD_LOC, corners)] {
        gl::BindBuffer(gl::ARRAY_BUFFER, vbo);
        gl::EnableVertexAttribArray(loc);
        gl::VertexAttribPointer(loc, 2, gl::FLOAT, gl::FALSE, 0, std::ptr::null());
    }
}

unsafe fn set_filter(filter: gl::types::GLenum) {
    gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, filter as i32);
    gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, filter as i32);
}

/// Every active uniform the program kept after linking, and its type.
unsafe fn uniform_kinds(prog: gl::types::GLuint) -> HashMap<String, gl::types::GLenum> {
    let mut out = HashMap::new();
    let mut count = 0;
    gl::GetProgramiv(prog, gl::ACTIVE_UNIFORMS, &mut count);
    let mut name = vec![0u8; 256];
    for i in 0..count.max(0) as gl::types::GLuint {
        let (mut len, mut size, mut kind) = (0, 0, 0);
        gl::GetActiveUniform(
            prog,
            i,
            name.len() as gl::types::GLsizei,
            &mut len,
            &mut size,
            &mut kind,
            name.as_mut_ptr() as *mut gl::types::GLchar,
        );
        let len = (len.max(0) as usize).min(name.len());
        let key = String::from_utf8_lossy(&name[..len]).into_owned();
        // An array reports its first element; nothing set here is an array, but the name is
        // what `GetUniformLocation` would be asked for.
        let key = key.strip_suffix("[0]").map(str::to_string).unwrap_or(key);
        out.insert(key, kind);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const SHADER: &str = "// header\n#version 130\n#pragma slot_filter linear\nvoid main() {}\n";

    #[test]
    fn the_device_gets_no_version_line_at_all() {
        let s = stage_source(SHADER, "FRAGMENT", true);
        assert!(!s.contains("#version"));
        assert!(s.starts_with("#define FRAGMENT\n"));
    }

    #[test]
    fn the_host_gets_its_own_version_first_and_the_files_own_is_blanked() {
        let s = stage_source(SHADER, "VERTEX", false);
        assert!(s.starts_with("#version 330 core\n#define VERTEX\n"));
        assert_eq!(s.matches("#version").count(), 1);
    }

    #[test]
    fn the_linear_pragma_is_read_and_its_absence_is_nearest() {
        assert!(wants_linear(SHADER));
        assert!(!wants_linear("void main() {}"));
    }
}
