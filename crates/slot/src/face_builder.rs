use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;

use slot_store::{play_log::play_time_text, Cart, Platform};
use slot_ui::shelf_life::Look;
use slot_ui::{
    board_face_with_stickers, cart_face_with_look, cart_interior, interior_face, padded, CartFace,
    TURN_PAD,
};

use crate::label_cache::label_art;

pub struct BuiltFaces {
    pub platform: Platform,
    pub stem: String,
    pub interior: CartFace,
    pub board: CartFace,
    pub lid: CartFace,
    pub has_set: bool,
}

#[derive(Clone, Debug)]
pub struct FaceAsk {
    pub cart: Cart,
    pub look: Look,
    pub played: Option<u64>,
}

impl From<Cart> for FaceAsk {
    fn from(cart: Cart) -> Self {
        FaceAsk {
            cart,
            look: Look::default(),
            played: None,
        }
    }
}

pub struct FaceBuilder {
    requests: Sender<FaceAsk>,
    built: Receiver<BuiltFaces>,
}

impl FaceBuilder {
    pub fn spawn(root: PathBuf) -> Self {
        let (requests, inbox) = mpsc::channel::<FaceAsk>();
        let (outbox, built) = mpsc::channel();
        let spawned = thread::Builder::new()
            .name("slot-faces".into())
            .spawn(move || {
                while let Ok(mut ask) = inbox.recv() {
                    while let Ok(newer) = inbox.try_recv() {
                        ask = newer;
                    }
                    let cart = &ask.cart;
                    let mut look = ask.look;
                    if look.wrap && crate::persist::has_save(&root, cart.platform, &cart.stem) {
                        look.wrap = false;
                    }
                    let play_text = ask.played.map(play_time_text);
                    let ach_text =
                        crate::cheevos::set_list(&root, &cart.rom).and_then(|s| s.progress_label());
                    let has_set = ach_text.is_some();
                    let board =
                        board_face_with_stickers(cart, play_text.as_deref(), ach_text.as_deref());
                    let lid = cart_face_with_look(cart, label_art(&root, cart), &look);
                    let interior = interior_face(cart).unwrap_or_else(|| cart_interior(cart));
                    let faces = BuiltFaces {
                        platform: cart.platform,
                        stem: cart.stem.clone(),
                        interior,
                        board,
                        lid: padded(&lid, TURN_PAD),
                        has_set,
                    };
                    if outbox.send(faces).is_err() {
                        return;
                    }
                }
            });
        if let Err(e) = spawned {
            eprintln!("slot: faces: worker thread failed to start: {e}");
        }
        FaceBuilder { requests, built }
    }

    pub fn request(&self, ask: impl Into<FaceAsk>) {
        let _ = self.requests.send(ask.into());
    }

    pub fn take(&self) -> Option<BuiltFaces> {
        let mut newest = None;
        while let Ok(faces) = self.built.try_recv() {
            newest = Some(faces);
        }
        newest
    }
}
