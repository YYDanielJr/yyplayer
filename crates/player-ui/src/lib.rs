//! Slint views and projection models. Playback remains outside this crate.

mod shell;
pub mod view_model;

slint::include_modules!();

pub use shell::{UiAction, UiShell};
pub mod presenter;

pub mod window_controls;

/// Keep the application's GPU context and GPU validation examples identical.
/// Slint 1.17.1 defaults to GLES; the pinned mpv deband shader is rejected by
/// AMD's Windows GLES compiler (`average` redeclares an existing name).
pub fn select_gpu_backend() -> Result<(), slint::PlatformError> {
    let selector = slint::BackendSelector::new()
        .backend_name("winit".into())
        .renderer_name("femtovg".into());
    #[cfg(windows)]
    let selector = selector.require_opengl_with_version(3, 3);
    selector.select()
}
