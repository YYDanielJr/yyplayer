//! Backend adapter for optional native surface styling, independent of mpv.
use slint::ComponentHandle;
#[cfg(windows)]
use slint::winit_030::{WinitWindowAccessor, winit};
#[derive(Default)]
pub struct Surface {
    previous: Option<bool>,
    pub state: player_platform::window_surface::SurfaceState,
}
impl Surface {
    pub fn update(&mut self, window: &player_ui::AppWindow) {
        let rounded = !window.window().is_fullscreen() && !window.window().is_maximized();
        if self.previous == Some(rounded) {
            return;
        }
        #[cfg(windows)]
        {
            use raw_window_handle::HasWindowHandle;
            use winit::platform::windows::WindowExtWindows;
            if let Some(state) = window
                .window()
                .with_winit_window(|w| {
                    w.set_undecorated_shadow(rounded);
                    w.window_handle()
                        .ok()
                        .map(|h| player_platform::window_surface::apply(h, rounded))
                })
                .flatten()
            {
                self.state = state;
                self.previous = Some(rounded);
            }
        }
        #[cfg(not(windows))]
        {
            self.previous = Some(rounded);
        }
    }
}
