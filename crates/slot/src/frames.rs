use std::ops::Deref;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};

pub struct Frames {
    inner: Mutex<Inner>,
    size: usize,
    taken: AtomicU64,
}

/// A frame's size and the shape to show it at, when it is not the Game Boy family's canvas.
#[derive(Copy, Clone, PartialEq, Debug)]
pub struct Shape {
    pub size: (u32, u32),
    pub aspect: Option<f32>,
}

impl Default for Shape {
    fn default() -> Self {
        Shape {
            size: (slot_retro::GBA_W, slot_retro::GBA_H),
            aspect: None,
        }
    }
}

struct Inner {
    ready: Option<(Vec<u8>, Shape)>,
    spare: Vec<Vec<u8>>,
    allocated: usize,
}

impl Frames {
    pub fn new(size: usize) -> Arc<Self> {
        Arc::new(Frames {
            inner: Mutex::new(Inner {
                ready: None,
                spare: Vec::new(),
                allocated: 0,
            }),
            size,
            taken: AtomicU64::new(0),
        })
    }

    pub fn take_write(&self) -> Vec<u8> {
        let mut i = self.lock();
        match i.spare.pop() {
            Some(buf) => buf,
            None => {
                i.allocated += 1;
                Vec::with_capacity(self.size)
            }
        }
    }

    pub fn publish(&self, buf: Vec<u8>) {
        self.publish_shaped(buf, Shape::default());
    }

    pub fn publish_shaped(&self, buf: Vec<u8>, shape: Shape) {
        let mut i = self.lock();
        if let Some((dropped, _)) = i.ready.replace((buf, shape)) {
            i.spare.push(dropped);
        }
    }

    pub fn latest(self: &Arc<Self>) -> Option<FrameRef> {
        let (buf, shape) = self.lock().ready.take()?;
        self.taken.fetch_add(1, Ordering::Relaxed);
        Some(FrameRef {
            frames: self.clone(),
            buf,
            shape,
        })
    }

    pub fn is_ready(&self) -> bool {
        self.lock().ready.is_some()
    }

    pub fn taken(&self) -> u64 {
        self.taken.load(Ordering::Relaxed)
    }

    pub fn allocated(&self) -> usize {
        self.lock().allocated
    }

    fn recycle(&self, buf: Vec<u8>) {
        self.lock().spare.push(buf);
    }

    fn lock(&self) -> MutexGuard<'_, Inner> {
        self.inner.lock().unwrap_or_else(|e| e.into_inner())
    }
}

pub struct FrameRef {
    frames: Arc<Frames>,
    buf: Vec<u8>,
    shape: Shape,
}

impl FrameRef {
    pub fn shape(&self) -> Shape {
        self.shape
    }
}

impl Deref for FrameRef {
    type Target = [u8];

    fn deref(&self) -> &[u8] {
        &self.buf
    }
}

impl Drop for FrameRef {
    fn drop(&mut self) {
        self.frames.recycle(std::mem::take(&mut self.buf));
    }
}
