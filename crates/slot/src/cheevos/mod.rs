// Ported from nickclyde/slot-plus@c218577, crates/slot/src/cheevos/mod.rs.

#[cfg(feature = "cheevos")]
pub mod boot;
#[cfg(feature = "cheevos")]
mod live;
#[cfg(not(feature = "cheevos"))]
mod off;
#[cfg(feature = "cheevos")]
pub mod overlay;

#[cfg(feature = "cheevos")]
pub use live::*;
#[cfg(not(feature = "cheevos"))]
pub use off::*;
