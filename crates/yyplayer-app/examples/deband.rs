//! Actual GPU screenshots across paused deband/decoder reloads and render teardown.
#[path = "../src/controller.rs"]
#[allow(dead_code)]
mod controller;
#[path = "../src/demo.rs"]
mod demo;
#[path = "../src/services.rs"]
#[allow(dead_code)]
mod services;
use player_core::{PlaybackCommand, PlaybackEngine, PlaybackPhase};
use player_ui::{UiAction, UiShell};
use slint::{ComponentHandle, Timer, TimerMode};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    time::{Duration, Instant},
};

fn control(c: &mut controller::AppController, action: &str, value: &str) {
    c.dispatch(UiAction::Control(action.into(), value.into()));
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let base = std::env::current_dir()?.join("target/deband");
    std::fs::create_dir_all(&base)?;
    let path = services::settings_path();
    if !path.starts_with(&base) {
        return Err("Use an independent target/deband/settings.json config".into());
    }
    let fixture = std::env::current_dir()?
        .join("target/render-regression/static/static-hevc.mp4")
        .canonicalize()?;
    let settings = player_core::settings::Settings {
        volume: 0.0,
        ..Default::default()
    };
    services::atomic_save(&path, &settings)?;
    player_ui::select_gpu_backend()?;
    let controller = Rc::new(RefCell::new(controller::AppController::live(
        player_mpv::MpvEngine::start(),
        player_platform::current(),
    )));
    let ui = UiShell::new()?;
    player_ui::presenter::attach(
        ui.component(),
        controller.borrow_mut().engine_mut().render_endpoint(),
    )?;
    controller.borrow_mut().prepare_paths(vec![fixture]);
    let weak = ui.component().as_weak();
    let project = ui.projector();
    let tick = controller.clone();
    let failure = Rc::new(RefCell::new(None::<String>));
    let failed = failure.clone();
    let complete = Rc::new(Cell::new(false));
    let done = complete.clone();
    let timer = Timer::default();
    let started = Instant::now();
    let mut due = started;
    let mut stage = 0;
    let mut generation = 0;
    let mut frames = 0;
    let mut position = 0.;
    let mut initializations = 0;
    let mut records = vec![];
    timer.start(TimerMode::Repeated, Duration::from_millis(60), move || {
        let Some(w) = weak.upgrade() else { return; };
        let mut c = tick.borrow_mut();
        c.tick();
        c.render_failed(w.get_render_error().as_str());
        project(&c.view_model());
        w.invoke_dismiss_toast();
        if Instant::now() < due { return; }
        let result = (|| -> Result<bool, Box<dyn std::error::Error>> {
            macro_rules! ensure { ($value:expr, $message:expr) => { let accepted: bool = $value; if !accepted { return Err($message.into()); } }; }
            let s = c.engine_mut().snapshot().clone();
            ensure!(started.elapsed() < Duration::from_secs(60), format!("Timeout at stage {stage}: {:?}", s.phase));
            ensure!(s.error.is_empty(), s.error.clone());
            ensure!(w.get_render_error().is_empty(), w.get_render_error().to_string());
            let reload_ready = s.phase == PlaybackPhase::Paused && s.generation > generation && w.get_render_frames() > frames;
            let capture = |name: &str| -> Result<(), Box<dyn std::error::Error>> {
                let p = w.window().take_snapshot()?;
                image::save_buffer(base.join(format!("{name}.png")), p.as_bytes(), p.width(), p.height(), image::ColorType::Rgba8)?;
                Ok(())
            };
            let applied_deband = c.view_model().deband;
            match stage {
                0 => {
                    if s.phase != PlaybackPhase::Playing || s.position.is_none_or(|p| p.as_secs_f64() < 1.5) || w.get_render_frames() < 40 { return Ok(false); }
                    ensure!(!s.hwdec.is_empty() && s.hwdec != "no", "No real hardware decoder");
                    c.engine_mut().submit(PlaybackCommand::Pause)?;
                }
                1 => {
                    if s.phase != PlaybackPhase::Paused { return Ok(false); }
                    position = s.position.ok_or("Unknown paused position")?.as_secs_f64();
                    initializations = w.get_render_initializations();
                    capture("hardware-plain")?;
                    generation = s.generation; frames = w.get_render_frames();
                    control(&mut c, "deband", "yes"); control(&mut c, "save-decode", "");
                }
                2..=4 => {
                    if !reload_ready { return Ok(false); }
                    ensure!((s.position.ok_or("Reload position unknown")?.as_secs_f64() - position).abs() <= 0.2, "Paused reload moved position");
                    ensure!(w.get_render_initializations() == initializations, "Decode reload recreated renderer");
                    ensure!(c.view_model().deband == (stage != 4), "Deband setting was not applied");
                    capture(match stage { 2 => "hardware-deband", 3 => "software-deband", _ => "software-plain" })?;
                    generation = s.generation; frames = w.get_render_frames();
                    if stage == 2 { control(&mut c, "decode-mode", "1"); control(&mut c, "save-decode", ""); }
                    else if stage == 3 { ensure!(s.hwdec == "no", "Software mode not confirmed"); control(&mut c, "deband", "no"); control(&mut c, "save-decode", ""); }
                    else { ensure!(s.hwdec == "no", "Software mode lost"); c.dispatch(UiAction::Navigate(5)); }
                }
                5 => {
                    if w.get_render_ready() || s.video_output_enabled { return Ok(false); }
                    control(&mut c, "video", "");
                }
                6 => {
                    if !w.get_render_ready() || s.phase != PlaybackPhase::Playing { return Ok(false); }
                    ensure!(w.get_render_initializations() == initializations + 1, "Resume did not rebuild renderer once");
                    ensure!((s.position.ok_or("Resume position unknown")?.as_secs_f64() - position).abs() <= 1.5, "Resume restarted at zero or lost position");
                    c.engine_mut().submit(PlaybackCommand::Pause)?;
                }
                7 => {
                    if s.phase != PlaybackPhase::Paused { return Ok(false); }
                    capture("resumed-software")?;
                    done.set(true); slint::quit_event_loop()?;
                }
                _ => unreachable!(),
            }
            records.push(serde_json::json!({"stage":stage,"phase":format!("{:?}",s.phase),"generation":s.generation,"position":s.position.map(|p|p.as_secs_f64()),"hwdec":s.hwdec,"frames":w.get_render_frames(),"initializations":w.get_render_initializations(),"deband":applied_deband}));
            std::fs::write(base.join("results.json"), serde_json::to_vec_pretty(&records)?)?;
            Ok(true)
        })();
        match result {
            Ok(true) => { stage += 1; due = Instant::now() + Duration::from_millis(1000); }
            Ok(false) => {}
            Err(error) => { *failed.borrow_mut() = Some(format!("stage {stage}: {error}")); let _ = slint::quit_event_loop(); }
        }
        project(&c.view_model());
    });
    ui.run()?;
    drop(timer);
    drop(ui);
    drop(controller);
    if let Some(error) = failure.borrow().as_ref() {
        return Err(error.clone().into());
    }
    if !complete.get() {
        return Err("Deband qualification did not complete".into());
    }
    println!(
        "PASS: paused deband/decoder reloads, GPU captures, renderer release/resume and normal exit"
    );
    Ok(())
}
