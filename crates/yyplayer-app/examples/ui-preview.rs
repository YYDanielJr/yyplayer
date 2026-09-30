//! Render the actual Slint UI once, save a PNG, then exit. Not a playback test.
//! cargo run -p yyplayer-app --example ui-preview -- --output docs/ui-preview.png

#[path = "../src/controller.rs"]
#[allow(dead_code)]
mod controller;
#[path = "../src/demo.rs"]
mod demo;
#[path = "../src/services.rs"]
#[allow(dead_code)]
mod services;

use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;
use std::time::Duration;

use player_mpv::MpvEngine;
use player_ui::{UiAction, UiShell};
use slint::ComponentHandle;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Offscreen capture must not depend on the contents of a swapped GL back buffer.
    // The application itself continues to use FemtoVG; this renderer is dev-only.
    slint::BackendSelector::new()
        .backend_name("winit".into())
        .renderer_name("software".into())
        .select()?;
    let arguments: Vec<_> = std::env::args_os().collect();
    let output = arguments
        .windows(2)
        .find(|pair| pair[0] == "--output")
        .map(|pair| PathBuf::from(&pair[1]))
        .unwrap_or_else(|| PathBuf::from("ui-preview.png"));
    let page = arguments
        .windows(2)
        .find(|pair| pair[0] == "--page")
        .map(|pair| match pair[1].to_string_lossy().as_ref() {
            "video" => 1,
            "recent" => 2,
            "settings" => 3,
            _ => 0,
        })
        .unwrap_or(0);

    let mut controller =
        controller::AppController::new(MpvEngine::default(), player_platform::current());
    controller.dispatch(UiAction::Navigate(page));
    let ui = UiShell::new()?;
    ui.project(&controller.view_model());
    let weak = ui.component().as_weak();
    let failure = Rc::new(RefCell::new(None));
    let capture_failure = failure.clone();
    slint::Timer::single_shot(Duration::from_millis(500), move || {
        let capture = || -> Result<(), Box<dyn std::error::Error>> {
            let window = weak.upgrade().ok_or("Preview window closed")?;
            let pixels = window.window().take_snapshot()?;
            if let Some(directory) = output.parent().filter(|path| !path.as_os_str().is_empty()) {
                std::fs::create_dir_all(directory)?;
            }
            image::save_buffer(
                &output,
                pixels.as_bytes(),
                pixels.width(),
                pixels.height(),
                image::ColorType::Rgba8,
            )?;
            println!("Saved UI preview: {}", output.display());
            Ok(())
        };
        if let Err(error) = capture() {
            *capture_failure.borrow_mut() = Some(error);
        }
        let _ = slint::quit_event_loop();
    });
    ui.run()?;
    let _ = player_core::PlaybackEngine::submit(
        controller.engine_mut(),
        player_core::PlaybackCommand::Shutdown,
    );
    if let Some(error) = failure.borrow_mut().take() {
        return Err(error);
    }
    Ok(())
}
