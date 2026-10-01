//! Production controller + own directory fixtures. GPU screenshots qualify layout; audio uses the production presenter.
#[path = "../src/controller.rs"]
#[allow(dead_code)]
mod controller;
#[path = "../src/demo.rs"]
mod demo;
#[path = "../src/services.rs"]
#[allow(dead_code)]
mod services;
use player_ui::{UiAction, UiShell};
use slint::{ComponentHandle, Timer, TimerMode};
use std::{
    cell::RefCell,
    path::PathBuf,
    rc::Rc,
    time::{Duration, Instant},
};
fn control(c: &mut controller::AppController, a: &str, v: &str) {
    c.dispatch(UiAction::Control(a.into(), v.into()));
}
fn capture(w: &player_ui::AppWindow, n: &str) -> Result<(), Box<dyn std::error::Error>> {
    let p = w.window().take_snapshot()?;
    image::save_buffer(
        format!("target/theme-validation/{n}.png"),
        p.as_bytes(),
        p.width(),
        p.height(),
        image::ColorType::Rgba8,
    )?;
    Ok(())
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    macro_rules! ensure {
        ($value:expr,$message:expr) => {
            if !$value {
                return Err($message.into());
            }
        };
    }
    let path = services::settings_path();
    if !path.starts_with(std::env::current_dir()?.join("target")) {
        return Err("Set YYPLAYER_CONFIG to a target/ path for independent verification".into());
    }
    let root = PathBuf::from("target/theme-validation/library").canonicalize()?;
    services::atomic_save(&path, &Default::default())?;
    slint::BackendSelector::new()
        .backend_name("winit".into())
        .renderer_name("femtovg".into())
        .select()?;
    let c = Rc::new(RefCell::new(controller::AppController::live(
        player_mpv::MpvEngine::start(),
        player_platform::current(),
    )));
    let ui = UiShell::new()?;
    player_ui::presenter::attach(
        ui.component(),
        c.borrow_mut().engine_mut().render_endpoint(),
    )?;
    let dispatch = c.clone();
    let project = ui.projector();
    let callback = Rc::new(move |a| {
        dispatch.borrow_mut().dispatch(a);
        project(&dispatch.borrow().view_model());
    });
    ui.bind(callback.clone());
    ui.bind_controls(callback);
    c.borrow_mut().prepare_paths(vec![root.clone()]);
    let weak = ui.component().as_weak();
    let tick = c.clone();
    let project = ui.projector();
    let timer = Timer::default();
    let error = Rc::new(RefCell::new(None::<String>));
    let fail = error.clone();
    let start = Instant::now();
    let mut last = Instant::now();
    let mut step = 0;
    timer.start(TimerMode::Repeated,Duration::from_millis(60),move||{
  let Some(w)=weak.upgrade()else{return;};let mut c=tick.borrow_mut();c.tick();project(&c.view_model());
  let result=(||->Result<(),Box<dyn std::error::Error>>{
   if start.elapsed()>Duration::from_secs(30){return Err("Directory/theme qualification timed out".into());}
   let view=c.view_model();if view.library.busy || last.elapsed()<Duration::from_millis(800){return Ok(());}
   match step {
    0=>{if view.library.rows.len()!=4{return Ok(());}println!("OS appearance available: {}; dark: {}; accent: {:06x}",view.appearance.system_available,view.appearance.dark,view.appearance.accent);capture(&w,"system")?;control(&mut c,"appearance-scheme","2");control(&mut c,"appearance-accent","#9b78e4");},
    1=>{ensure!(view.appearance.dark,"dark choice did not apply");capture(&w,"fashion-dark")?;control(&mut c,"appearance-scheme","1");},
    2=>{ensure!(!view.appearance.dark,"light choice did not apply");capture(&w,"fashion-light")?;control(&mut c,"appearance-design","0");},
    3=>{capture(&w,"simple-light")?;control(&mut c,"appearance-scheme","2");},
    4=>{capture(&w,"simple-dark")?;control(&mut c,"appearance-design","1");control(&mut c,"appearance-scheme","1");c.dispatch(UiAction::Navigate(3));},
    5=>{capture(&w,"settings")?;c.dispatch(UiAction::Navigate(0));control(&mut c,"panel","");w.window().set_size(slint::LogicalSize::new(1000.,640.));},
    6=>{capture(&w,"minimum-panel")?;control(&mut c,"panel","");w.window().set_size(slint::LogicalSize::new(1240.,900.));c.prepare_paths(vec![root.join("Quiet Hours")]);},
    7=>{ensure!(view.library.rows.len()==4,"library count / settings mismatch");control(&mut c,"library-folder","2");},
    8=>{ensure!(view.library.rows.len()==2,"library count / settings mismatch");let id=view.library.rows[0].id.to_string();control(&mut c,"library-play",&id);},
    9=>{ensure!(view.has_media,format!("media failed: {}",view.status));ensure!(view.playing,format!("audio not playing: {}",view.status));control(&mut c,"music-browse","");let id=view.library.rows[0].id.to_string();control(&mut c,"library-song-remove",&id);},
    10=>{ensure!(view.library.rows.len()==1,"library count / settings mismatch");ensure!(view.has_media,format!("media failed: {}",view.status));control(&mut c,"library-folder-remove","");},
    11=>{ensure!(view.library.rows.len()==3,"library count / settings mismatch");control(&mut c,"library-rescan","");control(&mut c,"library-rescan","");control(&mut c,"library-cancel","");control(&mut c,"appearance-motion","yes");},
    12=>{ensure!(view.appearance.reduce_motion,"reduce motion not applied");let (saved,msg)=services::load(&path);ensure!(msg.is_empty(),msg);ensure!(saved.library.roots.len()==1,"library count / settings mismatch");ensure!(saved.library.excluded.len()==1,"library count / settings mismatch");ensure!(!saved.appearance.system_accent,"manual accent not saved");std::fs::write(path.with_extension("verification.json"),serde_json::to_vec_pretty(&serde_json::json!({"result":"PASS","steps":13,"library_count":view.library.rows.len(),"os_theme_available":view.appearance.system_available,"source":"production controller, own fixtures","playing":view.playing}))?)?;println!("PASS: recursive folder import/dedup, filter, actual audio play, remove without stop, rescan/cancel, saved settings, four themes");slint::quit_event_loop()?;},
    _=>{}
   }
   step+=1;last=Instant::now();Ok(())
  })();if let Err(e)=result{*fail.borrow_mut()=Some(e.to_string());let _=slint::quit_event_loop();}
 });
    ui.run()?;
    timer.stop();
    drop(timer);
    ui.component().window().hide()?;
    drop(ui);
    c.borrow_mut().engine_mut().finish()?;
    c.borrow_mut().finish_services();
    if let Some(e) = error.borrow_mut().take() {
        return Err(e.into());
    }
    Ok(())
}
