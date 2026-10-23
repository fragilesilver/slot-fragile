use std::collections::{HashMap, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Condvar, Mutex};
use std::thread;

use slot_gfx::TexId;
use slot_store::Cart;
use slot_ui::{cart_face_with, cart_interior, shows_interior, CartFace};

use crate::app::{App, FaceWant};
use crate::label_cache::label_art;
use crate::text_faces::Textures;

const WORKERS: usize = 2;

type Key = (usize, usize);

/// A cart's face and, for a clear GBA cart, the game-coloured inside it is drawn over.
type Built = (Key, CartFace, Option<CartFace>);

#[derive(Default)]
struct Queue {
    jobs: VecDeque<(Key, Cart)>,
    closed: bool,
}

type Shared = Arc<(Mutex<Queue>, Condvar)>;

pub struct CartFaces {
    queue: Shared,
    built: Receiver<Built>,
    /// The rom each face being built is for. A shelf restocked meanwhile has another cart
    /// at that key, and that face must not land on it.
    pending: HashMap<Key, PathBuf>,
    asked: Vec<FaceWant>,
    free: Vec<TexId>,
}

impl CartFaces {
    pub fn spawn(root: PathBuf) -> Self {
        let queue: Shared = Arc::default();
        let (outbox, built) = mpsc::channel();
        for n in 0..WORKERS {
            let (queue, outbox, root) = (queue.clone(), outbox.clone(), root.clone());
            let spawned = thread::Builder::new()
                .name(format!("slot-carts-{n}"))
                .spawn(move || work(&queue, &outbox, &root));
            if let Err(e) = spawned {
                eprintln!("slot: cart faces: worker thread failed to start: {e}");
            }
        }
        CartFaces {
            queue,
            built,
            pending: HashMap::new(),
            asked: Vec::new(),
            free: Vec::new(),
        }
    }

    pub fn sync(&mut self, app: &mut App, compositor: &mut impl Textures) {
        self.free.extend(app.shed_faces());
        while let Ok((key, face, interior)) = self.built.try_recv() {
            self.place(app, compositor, key, face, interior);
        }
        let wants = app.face_wants();
        if wants != self.asked {
            self.requeue(app, &wants);
            self.asked = wants;
        }
        self.rebuild(app);
        let mut waiting: Vec<Key> = self
            .asked
            .iter()
            .filter(|w| w.on_screen)
            .map(|w| (w.shelf, w.index))
            .collect();
        while !waiting.is_empty() {
            let Ok((key, face, interior)) = self.built.recv() else {
                return;
            };
            waiting.retain(|k| *k != key);
            self.place(app, compositor, key, face, interior);
        }
    }

    fn requeue(&mut self, app: &App, wants: &[FaceWant]) {
        let (lock, wake) = &*self.queue;
        let mut queue = lock.lock().unwrap_or_else(|e| e.into_inner());
        for (key, _) in queue.jobs.drain(..) {
            self.pending.remove(&key);
        }
        for w in wants {
            let key = (w.shelf, w.index);
            let Some(cart) = app.face_cart(w.shelf, w.index) else {
                continue;
            };
            if self.pending.get(&key) == Some(&cart.rom) {
                continue;
            }
            queue.jobs.push_back((key, cart.clone()));
            self.pending.insert(key, cart.rom.clone());
        }
        wake.notify_all();
    }

    /// Fragile: queues the carts Game Art has given a label since their face was built (ported
    /// from cloudwithax/unslot@6339c18). Nothing waits on these: the old face is drawn until the
    /// new one is placed over it.
    fn rebuild(&mut self, app: &mut App) {
        let relabelled = app.take_relabelled();
        if relabelled.is_empty() {
            return;
        }
        let (lock, wake) = &*self.queue;
        let mut queue = lock.lock().unwrap_or_else(|e| e.into_inner());
        for key in relabelled {
            // Queued, maybe with the cart as it was: build it from the cart as it is.
            queue.jobs.retain(|(k, _)| *k != key);
            if let Some(cart) = app.face_cart(key.0, key.1) {
                queue.jobs.push_back((key, cart.clone()));
                self.pending.insert(key, cart.rom.clone());
            }
        }
        wake.notify_all();
    }

    fn texture(&mut self, compositor: &mut impl Textures, face: &CartFace) -> TexId {
        match self.free.pop() {
            Some(tex) => {
                compositor.update(tex, face.w, face.h, &face.rgba);
                tex
            }
            None => compositor.create(face.w, face.h, &face.rgba),
        }
    }

    fn place(
        &mut self,
        app: &mut App,
        compositor: &mut impl Textures,
        key: Key,
        face: CartFace,
        interior: Option<CartFace>,
    ) {
        // Ported from nickclyde/slot-plus@02c5890, crates/slot/src/cart_faces.rs.
        let asked_for = self.pending.remove(&key);
        self.asked.retain(|w| (w.shelf, w.index) != key);
        if asked_for.is_some_and(|rom| app.face_cart(key.0, key.1).map(|c| &c.rom) != Some(&rom)) {
            return;
        }
        let tex = self.texture(compositor, &face);
        self.free.extend(app.set_cart_face(key.0, key.1, tex));
        if let Some(interior) = interior {
            let tex = self.texture(compositor, &interior);
            self.free.extend(app.set_cart_interior(key.0, key.1, tex));
        }
    }
}

impl Drop for CartFaces {
    fn drop(&mut self) {
        let (lock, wake) = &*self.queue;
        lock.lock().unwrap_or_else(|e| e.into_inner()).closed = true;
        wake.notify_all();
    }
}

fn work(queue: &Shared, outbox: &Sender<Built>, root: &Path) {
    let (lock, wake) = &**queue;
    loop {
        let (key, cart) = {
            let mut q = lock.lock().unwrap_or_else(|e| e.into_inner());
            loop {
                if q.closed {
                    return;
                }
                if let Some(job) = q.jobs.pop_front() {
                    break job;
                }
                q = wake.wait(q).unwrap_or_else(|e| e.into_inner());
            }
        };
        let face = cart_face_with(&cart, label_art(root, &cart));
        let interior = shows_interior(&cart).then(|| cart_interior(&cart));
        if outbox.send((key, face, interior)).is_err() {
            return;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use slot_store::Platform;

    #[derive(Default)]
    struct Gpu {
        made: usize,
    }

    impl Textures for Gpu {
        fn create(&mut self, _w: u32, _h: u32, _rgba: &[u8]) -> TexId {
            self.made += 1;
            TexId::from_raw(self.made)
        }

        fn update(&mut self, _id: TexId, _w: u32, _h: u32, _rgba: &[u8]) {}
    }

    fn cart(stem: &str) -> Cart {
        Cart {
            platform: Platform::Gba,
            stem: stem.into(),
            rom: format!("Games/GBA/{stem}.gba").into(),
            label: None,
            title: stem.to_uppercase(),
            code: String::new(),
            shell: None,
        }
    }

    fn idle() -> (CartFaces, Sender<Built>) {
        let (outbox, built) = mpsc::channel();
        let faces = CartFaces {
            queue: Arc::default(),
            built,
            pending: HashMap::new(),
            asked: Vec::new(),
            free: Vec::new(),
        };
        (faces, outbox)
    }

    fn face() -> CartFace {
        CartFace {
            rgba: vec![0; 4],
            w: 1,
            h: 1,
        }
    }

    #[test]
    fn a_face_built_for_a_cart_that_has_since_moved_is_dropped() {
        let before = App::new(vec![cart("Alpha")]);
        let after = App::new(vec![cart("Beta")]);
        let want = before.face_wants()[0];
        let key = (want.shelf, want.index);
        let (mut faces, _) = idle();
        let mut gpu = Gpu::default();

        faces.requeue(&before, &[want]);
        let mut moved = after;
        faces.place(&mut moved, &mut gpu, key, face(), None);
        assert_eq!(gpu.made, 0, "Alpha's face went onto Beta");

        let mut same = before;
        faces.requeue(&same, &[want]);
        faces.place(&mut same, &mut gpu, key, face(), None);
        assert_eq!(gpu.made, 1, "the face for the cart still there was dropped");
    }

    #[test]
    fn a_cart_that_moves_while_its_old_face_is_built_is_asked_for_again() {
        let before = App::new(vec![cart("Alpha")]);
        let after = App::new(vec![cart("Beta")]);
        let want = before.face_wants()[0];
        let (mut faces, _) = idle();

        faces.requeue(&before, &[want]);
        // A worker takes the job: it leaves the queue but is still being built.
        faces.queue.0.lock().unwrap().jobs.clear();
        faces.requeue(&after, &[want]);
        let jobs = &faces.queue.0.lock().unwrap().jobs;
        assert_eq!(
            jobs.iter()
                .map(|(_, c)| c.stem.as_str())
                .collect::<Vec<_>>(),
            ["Beta"],
            "Beta's face was never asked for"
        );
    }
}
