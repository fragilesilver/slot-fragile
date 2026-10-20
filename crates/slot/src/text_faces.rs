//! Textures for words slot only knows at run time: the rows of a list page. Each face is
//! drawn once and kept in a small pool; when the pool is full the face used longest ago gives
//! up its texture, which is redrawn in place since textures are never freed.

use slot_gfx::{Compositor, TexId};
use slot_ui::UndoFace;

pub type Face = (TexId, u32, u32);

/// Where the pool gets its textures, so it can be tested without GL.
pub trait Textures {
    fn create(&mut self, w: u32, h: u32, rgba: &[u8]) -> TexId;
    fn update(&mut self, id: TexId, w: u32, h: u32, rgba: &[u8]);
}

impl Textures for Compositor {
    fn create(&mut self, w: u32, h: u32, rgba: &[u8]) -> TexId {
        self.create_texture(w, h, rgba)
    }

    fn update(&mut self, id: TexId, w: u32, h: u32, rgba: &[u8]) {
        self.update_texture(id, w, h, rgba);
    }
}

struct Slot<K> {
    key: K,
    face: Face,
    used: u64,
}

pub struct TextFaces<K> {
    cap: usize,
    slots: Vec<Slot<K>>,
    tick: u64,
    /// The tick the current frame began on. Faces handed out since are never given up, so a
    /// frame that needs more than `cap` grows the pool rather than redraw a face it still draws.
    frame: u64,
}

impl<K: PartialEq + Clone> TextFaces<K> {
    pub fn new(cap: usize) -> Self {
        TextFaces {
            cap: cap.max(1),
            slots: Vec::new(),
            tick: 0,
            frame: 1,
        }
    }

    pub fn begin_frame(&mut self) {
        self.tick += 1;
        self.frame = self.tick;
    }

    pub fn len(&self) -> usize {
        self.slots.len()
    }

    pub fn is_empty(&self) -> bool {
        self.slots.is_empty()
    }

    /// The face for `key`, drawn by `make` only when the pool does not have it.
    pub fn get(
        &mut self,
        key: &K,
        make: impl FnOnce() -> UndoFace,
        gpu: &mut impl Textures,
    ) -> Face {
        self.tick += 1;
        if let Some(slot) = self.slots.iter_mut().find(|s| s.key == *key) {
            slot.used = self.tick;
            return slot.face;
        }
        let face = make();
        let full = self.slots.len() >= self.cap;
        let oldest = self
            .slots
            .iter_mut()
            .filter(|s| s.used < self.frame)
            .min_by_key(|s| s.used);
        match oldest {
            Some(slot) if full => {
                let id = slot.face.0;
                gpu.update(id, face.w, face.h, &face.rgba);
                *slot = Slot {
                    key: key.clone(),
                    face: (id, face.w, face.h),
                    used: self.tick,
                };
                slot.face
            }
            _ => {
                let id = gpu.create(face.w, face.h, &face.rgba);
                let face = (id, face.w, face.h);
                self.slots.push(Slot {
                    key: key.clone(),
                    face,
                    used: self.tick,
                });
                face
            }
        }
    }
}
