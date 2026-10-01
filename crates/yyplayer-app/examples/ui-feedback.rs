//! UI-only regression: real Slint timers, close-button click, and interaction guards.
//! cargo run --locked -p yyplayer-app --example ui-feedback --offline
use player_ui::{UiAction, UiShell, view_model::ShellViewModel};
use slint::{ComponentHandle, Timer, TimerMode};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    time::{Duration, Instant},
};

fn capture(window: &player_ui::AppWindow, name: &str) -> Result<(), Box<dyn std::error::Error>> {
    let pixels = window.window().take_snapshot()?;
    image::save_buffer(
        format!("target/ui-feedback/{name}.png"),
        pixels.as_bytes(),
        pixels.width(),
        pixels.height(),
        image::ColorType::Rgba8,
    )?;
    Ok(())
}

fn click(window: &player_ui::AppWindow, position: slint::LogicalPosition) {
    window
        .window()
        .dispatch_event(slint::platform::WindowEvent::PointerPressed {
            position,
            button: slint::platform::PointerEventButton::Left,
        });
    window
        .window()
        .dispatch_event(slint::platform::WindowEvent::PointerReleased {
            position,
            button: slint::platform::PointerEventButton::Left,
        });
}

fn click_center_play(
    window: &player_ui::AppWindow,
    count: &Cell<u32>,
) -> Result<(), Box<dyn std::error::Error>> {
    let size = window
        .window()
        .size()
        .to_logical(window.window().scale_factor());
    let before = count.get();
    click(
        window,
        slint::LogicalPosition::new(size.width / 2.0, size.height - 23.0),
    );
    if count.get() != before + 1 {
        return Err("Play button is not at window center".into());
    }
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    slint::BackendSelector::new()
        .backend_name("winit".into())
        .renderer_name("software".into())
        .select()?;
    std::fs::create_dir_all("target/ui-feedback")?;
    let ui = Rc::new(UiShell::new()?);
    let play_count = Rc::new(Cell::new(0u32));
    let play_callback = play_count.clone();
    ui.bind(Rc::new(move |action| {
        if matches!(action, UiAction::RequestPlayback) {
            play_callback.set(play_callback.get() + 1);
        }
    }));
    let mut model = ShellViewModel {
        page: 1,
        fullscreen: true,
        status: "设置已保存".into(),
        status_revision: 1,
        selected_title: "UI 交互验证".into(),
        volume_percent: 70.0,
        speed_index: 2,
        speed_choices: ["0.5x", "0.75x", "1x", "1.25x", "1.5x", "2x", "3x", "4x"]
            .into_iter()
            .map(str::to_owned)
            .collect(),
        ..Default::default()
    };
    ui.project(&model);
    let failure = Rc::new(RefCell::new(None));
    let result = failure.clone();
    let completed = Rc::new(Cell::new(false));
    let success = completed.clone();
    let test_ui = ui.clone();
    let timer = Timer::default();
    let started = Instant::now();
    let times = [
        0.8, 3.6, 4.5, 4.8, 5.2, 5.5, 9.0, 9.8, 13.2, 16.5, 19.8, 23.1, 26.5, 29.9, 33.3, 34.7,
        36.1,
    ];
    let mut step = 0;
    let mut video_size = (0.0, 0.0);
    let mut checkpoints = Vec::new();
    timer.start(TimerMode::Repeated, Duration::from_millis(50), move || {
        if step >= times.len() || started.elapsed().as_secs_f64() < times[step] { return; }
        let window = test_ui.component();
        let mut run = || -> Result<(), Box<dyn std::error::Error>> {
            match step {
                0 => {
                    if !window.get_toast_visible() || !window.get_controls_visible() || !window.get_header_visible() { return Err("Initial feedback missing".into()); }
                    video_size = (window.get_video_width(), window.get_video_height());
                    capture(window, "fullscreen-visible")?;
                    click_center_play(window, &play_count)?;
                }
                1 => {
                    if window.get_controls_visible() || window.get_header_visible() || !window.get_toast_visible() { return Err("3s controls timeout failed".into()); }
                }
                2 => {
                    if window.get_toast_visible() { return Err("4s toast timeout failed".into()); }
                    if video_size != (window.get_video_width(), window.get_video_height()) { return Err("Hiding controls resized video".into()); }
                    capture(window, "fullscreen-hidden")?;
                    window.invoke_pointer_activity();
                    model.status_revision = 2;
                    test_ui.project(&model);
                }
                3 => {
                    if !window.get_controls_visible() || !window.get_toast_visible() { return Err("Wake / repeated message failed".into()); }
                    let size = window.window().size().to_logical(window.window().scale_factor());
                    let position = slint::LogicalPosition::new(size.width / 2.0 + 234.0, size.height - 114.0);
                    click(window, position);
                }
                4 => {
                    if window.get_toast_visible() { return Err("Close-button click failed".into()); }
                    test_ui.project(&model);
                }
                5 => {
                    if window.get_toast_visible() { return Err("Projection revived dismissed toast".into()); }
                    model.status_revision = 3;
                    test_ui.project(&model);
                    window.set_pointer_held(true);
                    window.invoke_pointer_activity();
                }
                6 => {
                    if !window.get_toast_visible() || !window.get_controls_visible() { return Err("Repeated-message deadline / drag guard failed".into()); }
                    capture(window, "floating-toast")?;
                }
                7 => {
                    if window.get_toast_visible() || !window.get_controls_visible() { return Err("Toast timeout / drag guard failed".into()); }
                    window.set_pointer_held(false);
                    window.invoke_pointer_activity();
                }
                8 => {
                    if window.get_controls_visible() { return Err("Release did not re-arm hiding".into()); }
                    window.set_pointer_in_controls(true);
                    window.invoke_pointer_activity();
                }
                9 => {
                    if !window.get_controls_visible() { return Err("Control hover guard failed".into()); }
                    window.set_pointer_in_controls(false);
                    window.set_panel_open(true);
                }
                10 => {
                    if !window.get_controls_visible() { return Err("Options panel guard failed".into()); }
                    window.set_panel_open(false);
                    window.invoke_pointer_activity();
                }
                11 => {
                    if window.get_controls_visible() { return Err("Closing panel did not re-arm hiding".into()); }
                    window.set_fullscreen_mode(false);
                    if !window.get_controls_visible() { return Err("Windowed controls not restored".into()); }
                }
                12 => {
                    if !window.get_controls_visible() || window.get_header_visible() { return Err("Windowed header timeout failed".into()); }
                    video_size = (window.get_video_width(), window.get_video_height());
                    let pixels = window.window().take_snapshot()?;
                    let top = (pixels.width() / 2) as usize * 4;
                    if pixels.as_bytes()[top..top + 3] != [8, 11, 13] { return Err("Unused space remains above video".into()); }
                    capture(window, "windowed-hidden-header")?;
                    window.set_pointer_in_header(true);
                    window.invoke_pointer_activity();
                }
                13 => {
                    if !window.get_header_visible() { return Err("Header hover guard failed".into()); }
                    if video_size != (window.get_video_width(), window.get_video_height()) { return Err("Header visibility resized video".into()); }
                    capture(window, "windowed-visible-header")?;
                    window.set_pointer_in_header(false);
                    window.invoke_pointer_activity();
                }
                14 => {
                    if window.get_header_visible() { return Err("Leaving header did not re-arm hiding".into()); }
                    window.window().set_size(slint::LogicalSize::new(1000.0, 640.0));
                    window.invoke_pointer_activity();
                }
                15 => {
                    click_center_play(window, &play_count)?;
                    capture(window, "minimum-window")?;
                    window.window().set_size(slint::LogicalSize::new(1460.0, 920.0));
                    window.invoke_pointer_activity();
                }
                16 => {
                    click_center_play(window, &play_count)?;
                    capture(window, "wide-window")?;
                }
                _ => unreachable!(),
            }
            checkpoints.push(serde_json::json!({ "step": step, "seconds": started.elapsed().as_secs_f64(), "controls": window.get_controls_visible(), "header": window.get_header_visible(), "toast": window.get_toast_visible(), "video_width": window.get_video_width(), "video_height": window.get_video_height() }));
            if step == 16 {
                std::fs::write("target/ui-feedback/results.json", serde_json::to_vec_pretty(&checkpoints)?)?;
                success.set(true);
                println!("PASS: toast / fullscreen controls / windowed header timeout and hover / no top gap / centered play click at normal, minimum and wide sizes");
                slint::quit_event_loop()?;
            }
            Ok(())
        };
        if let Err(error) = run() {
            *result.borrow_mut() = Some(format!("step {step}: {error}"));
            let _ = slint::quit_event_loop();
        }
        step += 1;
    });
    ui.run()?;
    if let Some(error) = failure.borrow_mut().take() {
        return Err(error.into());
    }
    if !completed.get() {
        return Err("UI regression ended before completion".into());
    }
    Ok(())
}
