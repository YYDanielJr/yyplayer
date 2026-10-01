//! Slint views and projection models. Playback remains outside this crate.

mod shell;
pub mod view_model;

slint::include_modules!();

pub use shell::{UiAction, UiShell};
pub mod presenter;

pub mod window_controls;
