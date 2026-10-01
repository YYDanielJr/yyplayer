//! Theme chrome actions use the backend window on the UI thread, never mpv.
use crate::AppWindow;
use slint::{
    ComponentHandle,
    winit_030::{WinitWindowAccessor, winit},
};
use std::rc::Rc;
pub fn bind(window: &AppWindow, request_mode: Rc<dyn Fn(i32)>) {
    let weak = window.as_weak();
    window.on_window_action(move |action| {
        let Some(window) = weak.upgrade() else {
            return;
        };
        match action.as_str() {
            "close" => {
                let _ = slint::quit_event_loop();
            }
            "minimize" => window.window().set_minimized(true),
            "maximize" => request_mode(if window.window().is_maximized() { 0 } else { 1 }),
            "drag" => {
                let _ = window.window().with_winit_window(|w| w.drag_window());
            }
            value if value.starts_with("resize-") => {
                use winit::window::ResizeDirection as R;
                let direction = match &value[7..] {
                    "w" => R::West,
                    "e" => R::East,
                    "n" => R::North,
                    "s" => R::South,
                    "nw" => R::NorthWest,
                    "ne" => R::NorthEast,
                    "sw" => R::SouthWest,
                    "se" => R::SouthEast,
                    _ => return,
                };
                let _ = window
                    .window()
                    .with_winit_window(|w| w.drag_resize_window(direction));
            }
            _ => {}
        }
    });
}
