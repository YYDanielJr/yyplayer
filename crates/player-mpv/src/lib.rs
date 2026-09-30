//! The worker owns ordinary libmpv calls; only the leased render endpoint crosses to GL.
pub mod ffi;
mod loader;
mod worker;
pub use worker::{MpvEngine, RenderBridge};
