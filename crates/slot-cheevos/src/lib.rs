// Ported from nickclyde/slot-plus@29b8a81, crates/slot-cheevos/src/lib.rs.

mod client;
mod ffi;

pub mod badge;
pub mod config;
pub mod game;
pub mod memory;
pub mod notice;
pub mod transport;

pub use client::{Achievement, Cheevos, Login};
pub use config::{Credentials, Secret, CHEEVOS_FILE};
pub use notice::Notice;
pub use transport::{Http, Request, Response, Transport};
