//! UI-only regression: real Slint timers, close-button click, and interaction guards.
//! cargo run --locked -p yyplayer-app --example ui-feedback --offline
use player_ui::{UiShell, view_model::ShellViewModel};
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

fn main() -> Result<(), Box<dyn std::error::Error>> {
    slint::BackendSelector::new()
        .backend_name("winit".into())
        .renderer_name("software".into())
        .select()?;
    std::fs::create_dir_all("target/ui-feedback")?;
    let ui = Rc::new(UiShell::new()?);
    let mut model = ShellViewModel {
        page: 1,
        fullscreen: true,
        status: "设置已保存".into(),
        status_revision: 1,
        selected_title: "UI 交互验证".into(),
        volume_percent: 70.0,
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
        0.8, 3.6, 4.5, 4.8, 5.2, 5.5, 9.0, 9.8, 13.2, 16.5, 19.8, 23.1,
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
                    if !window.get_toast_visible() || !window.get_controls_visible() { return Err("Initial feedback missing".into()); }
                    video_size = (window.get_video_width(), window.get_video_height());
                    capture(window, "fullscreen-visible")?;
                }
                1 => {
                    if window.get_controls_visible() || !window.get_toast_visible() { return Err("3s controls timeout failed".into()); }
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
                    window.window().dispatch_event(slint::platform::WindowEvent::PointerPressed { position, button: slint::platform::PointerEventButton::Left });
                    window.window().dispatch_event(slint::platform::WindowEvent::PointerReleased { position, button: slint::platform::PointerEventButton::Left });
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
                _ => unreachable!(),
            }
            checkpoints.push(serde_json::json!({ "step": step, "seconds": started.elapsed().as_secs_f64(), "controls": window.get_controls_visible(), "toast": window.get_toast_visible(), "video_width": window.get_video_width(), "video_height": window.get_video_height() }));
            if step == 11 {
                std::fs::write("target/ui-feedback/results.json", serde_json::to_vec_pretty(&checkpoints)?)?;
                success.set(true);
                println!("PASS: toast timeout / real close click / repeated messages / stable projection; fullscreen timeout / wake / drag / hover / panel / windowed restoration");
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
