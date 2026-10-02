//! Developer-only layout capture. No engine, user media, or user settings.
//! cargo run --locked --offline -p yyplayer-app --example background-preview
use player_ui::UiShell;
use slint::{ComponentHandle, Timer, TimerMode};
use std::time::Duration;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    slint::BackendSelector::new()
        .backend_name("winit".into())
        .renderer_name("femtovg".into())
        .select()?;
    std::fs::create_dir_all("target/background-preview")?;
    let ui = UiShell::new()?;
    let window = ui.component();
    window.set_selected_title("本地歌曲 · 背景预览".into());
    window.set_selected_artist("YYPlayer".into());
    window.set_plain_lyrics("让喜欢的声音，有自己的模样。".into());
    window.set_appearance_design(1);
    window.set_appearance_reduced(true);
    let sample: Vec<u8> = (0..180u32)
        .flat_map(|y| {
            (0..320u32).flat_map(move |x| [25 + (x / 3) as u8, 70 + (y / 2) as u8, 150, 255])
        })
        .collect();
    let custom = slint::Image::from_rgba8(
        slint::SharedPixelBuffer::<slint::Rgba8Pixel>::clone_from_slice(&sample, 320, 180),
    );
    window.set_custom_library_background(custom.clone());
    window.set_custom_lyrics_background(custom);
    let weak = window.as_weak();
    let timer = Timer::default();
    let mut step = 0;
    let mut phase = 0;
    let error = Rc::new(RefCell::new(None));
    let result = error.clone();
    let completed = Rc::new(Cell::new(false));
    let done = completed.clone();
    timer.start(TimerMode::Repeated, Duration::from_millis(850), move || {
        let Some(window) = weak.upgrade() else {
            return;
        };
        if phase == 0 {
            let (page, style) = if step < 4 {
                (0, step + 1)
            } else if step < 8 {
                (4, step - 3)
            } else if step == 8 {
                (3, 0)
            } else if step == 9 {
                (0, 1)
            } else if step == 10 {
                (4, 2)
            } else if step == 11 {
                (5, 4)
            } else if step == 12 {
                (0, 5)
            } else {
                (4, 5)
            };
            if step >= 9 {
                window.set_appearance_dark(false);
                window.set_appearance_accent(slint::Color::from_rgb_u8(23, 137, 128));
            }
            window.set_page(page);
            if page == 0 || page == 5 {
                window.set_library_background_style(style);
                if style == 5 {
                    window.set_library_background_opacity(35);
                }
            }
            if page == 4 {
                window.set_lyrics_background_style(style);
                if style == 5 {
                    window.set_lyrics_background_opacity(35);
                }
            }
            if page == 3 {
                window.set_library_background_style(5);
                window.set_lyrics_background_style(5);
                window.set_library_background_name("sample.png".into());
                window.set_lyrics_background_name("sample.jpg".into());
                window.set_library_background_opacity(35);
                window.set_lyrics_background_opacity(35);
            }
            window.window().request_redraw();
            phase = 1;
            return;
        }
        if phase == 1 {
            let _ = window.window().take_snapshot();
            phase = 2;
            return;
        }
        match window.window().take_snapshot() {
            Ok(pixels) => {
                let path = format!("target/background-preview/{step:02}.png");
                if let Err(err) = image::save_buffer(
                    path,
                    pixels.as_bytes(),
                    pixels.width(),
                    pixels.height(),
                    image::ColorType::Rgba8,
                ) {
                    *result.borrow_mut() = Some(err.to_string());
                }
            }
            Err(err) => *result.borrow_mut() = Some(err.to_string()),
        }
        step += 1;
        phase = 0;
        if step == 14 || result.borrow().is_some() {
            if let Some(message) = result.borrow().as_ref() {
                eprintln!("{message}");
            }
            done.set(step == 14 && result.borrow().is_none());
            let _ = slint::quit_event_loop();
        }
    });
    ui.run()?;
    if !completed.get() || error.borrow().is_some() {
        return Err("Background preview incomplete".into());
    }
    println!(
        "PASS: 8 backgrounds, video library, settings, light/accent and custom-image variants captured"
    );
    Ok(())
}
