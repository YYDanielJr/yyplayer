//! Production font discovery/controller, native window actions and real SRT/ASS rendering.
#[path = "../src/controller.rs"]
#[allow(dead_code)]
mod controller;
#[path = "../src/demo.rs"]
mod demo;
#[path = "../src/services.rs"]
#[allow(dead_code)]
mod services;
#[path = "../src/window_surface.rs"]
mod window_surface;
use player_core::{PlaybackCommand, PlaybackEngine};
use player_ui::{UiAction, UiShell};
use slint::{ComponentHandle, Timer, TimerMode};
use std::{
    cell::RefCell,
    rc::Rc,
    time::{Duration, Instant},
};
fn pointer(w: &player_ui::AppWindow, x: f32, y: f32, press: bool) {
    let position = slint::LogicalPosition::new(x, y);
    w.window().dispatch_event(if press {
        slint::platform::WindowEvent::PointerPressed {
            position,
            button: slint::platform::PointerEventButton::Left,
        }
    } else {
        slint::platform::WindowEvent::PointerReleased {
            position,
            button: slint::platform::PointerEventButton::Left,
        }
    });
}
fn click(w: &player_ui::AppWindow, x: f32, y: f32) {
    pointer(w, x, y, true);
    pointer(w, x, y, false);
}
fn control(c: &Rc<RefCell<controller::AppController>>, a: &str, v: &str) {
    c.borrow_mut()
        .dispatch(UiAction::Control(a.into(), v.into()));
}
fn capture(w: &player_ui::AppWindow, name: &str) -> Result<(), Box<dyn std::error::Error>> {
    let p = w.window().take_snapshot()?;
    image::save_buffer(
        format!("target/ui-customization/{name}.png"),
        p.as_bytes(),
        p.width(),
        p.height(),
        image::ColorType::Rgba8,
    )?;
    Ok(())
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = std::env::current_dir()?.join("target/ui-customization");
    let path = services::settings_path();
    if !path.starts_with(&root) {
        return Err("Set independent YYPLAYER_CONFIG inside target/ui-customization".into());
    }
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
    let callback = c.clone();
    let project = ui.projector();
    let dispatch = Rc::new(move |a| {
        callback.borrow_mut().dispatch(a);
        project(&callback.borrow().view_model());
    });
    ui.bind(dispatch.clone());
    ui.bind_controls(dispatch);
    let weak = ui.component().as_weak();
    let mode_window = weak.clone();
    player_ui::window_controls::bind(
        ui.component(),
        Rc::new(move |mode| {
            if let Some(w) = mode_window.upgrade() {
                w.window().set_maximized(mode == 1);
            }
        }),
    );
    c.borrow_mut().prepare_paths(vec![root.join("library")]);
    let timer = Timer::default();
    let started = Instant::now();
    let mut last = Instant::now();
    let mut step = 0;
    let mut drag_width = 0.;
    let mut selected_font = String::new();
    let mut font_count = 0;
    let mut records = Vec::new();
    let mut surface = window_surface::Surface::default();
    let failure = Rc::new(RefCell::new(None));
    let failed = failure.clone();
    let complete = Rc::new(std::cell::Cell::new(false));
    let success = complete.clone();
    let test = c.clone();
    let project = ui.projector();
    timer.start(TimerMode::Repeated,Duration::from_millis(60),move||{
  let Some(w)=weak.upgrade()else{return;};surface.update(&w);test.borrow_mut().request_library_covers(w.get_library_visible_first(),w.get_library_visible_count());test.borrow_mut().tick();project(&test.borrow().view_model());w.set_window_maximized(w.window().is_maximized());
  // Keep the simulated macOS layout visible for a complete rendered frame.
  if step==11 { w.set_window_controls_left(true); }
  if (8..=10).contains(&step) || step==18 { w.invoke_dismiss_toast(); }
  let run=(||->Result<(),Box<dyn std::error::Error>>{
   if started.elapsed()>Duration::from_secs(60){return Err("Customization validation timed out".into());}
   if last.elapsed()<Duration::from_millis(900){return Ok(());}
   let view=test.borrow().view_model();let size=w.window().size().to_logical(w.window().scale_factor());
   macro_rules! ensure {($v:expr,$m:expr)=>{let accepted:bool=$v;if !accepted{return Err($m.into());}};}
   match step {
    0=>{if view.library.busy || view.fonts.busy || view.library.rows.len()!=80{return Ok(());}
      if view.library.rows[0].artwork.size().width==0{return Ok(());}ensure!(view.library.rows[0].artwork.size().width<=64,"Thumbnail size invalid");ensure!(surface.state.result==Some(0) && surface.state.preference==Some(2),"Native rounding preference failed");font_count=view.fonts.names.len()-1;ensure!(font_count>10,"Font catalog empty");capture(&w,"library-default")?;
      drag_width=size.width-202.-48.-24.-38.-88.-58.;let x=202.+24.+88.+drag_width*0.46;pointer(&w,x,451.,true);w.window().dispatch_event(slint::platform::WindowEvent::PointerMoved{position:slint::LogicalPosition::new(x+drag_width*0.1,451.)});pointer(&w,x+drag_width*0.1,451.,false);},
    1=>{ensure!((w.get_column_song()-0.56).abs()<0.015,"Column drag failed");capture(&w,"library-resized")?;w.window().set_size(slint::LogicalSize::new(1000.,640.));},
    2=>{capture(&w,"library-minimum")?;w.window().set_size(slint::LogicalSize::new(1240.,900.));test.borrow_mut().dispatch(UiAction::Navigate(3));},
    3=>{click(&w,393.,176.);},
    4=>{capture(&w,"font-settings")?;click(&w,496.,290.);w.window().dispatch_event(slint::platform::WindowEvent::KeyPressed{text:slint::platform::Key::Escape.into()});w.window().dispatch_event(slint::platform::WindowEvent::KeyReleased{text:slint::platform::Key::Escape.into()});w.window().dispatch_event(slint::platform::WindowEvent::KeyPressed{text:slint::platform::Key::DownArrow.into()});w.window().dispatch_event(slint::platform::WindowEvent::KeyReleased{text:slint::platform::Key::DownArrow.into()});},
    5=>{ensure!(view.fonts.ui_index>0,"Actual font ComboBox event failed");ensure!(!w.get_ui_font().is_empty(),"Global font not applied");selected_font=view.fonts.names.iter().find(|n|n.as_str()=="Times New Roman").cloned().unwrap_or_else(||view.fonts.names[1].clone());let index=view.fonts.names.iter().position(|n|n==&selected_font).unwrap().to_string();control(&test,"font-lyrics",&index);control(&test,"font-subtitles",&index);control(&test,"font-ass","yes");},
    6=>{let snap=test.borrow_mut().engine_mut().snapshot().clone();ensure!(snap.subtitle_font==selected_font,"Engine subtitle font differs");ensure!(snap.subtitle_font_overrides.contains(&selected_font),"ASS override not confirmed");capture(&w,"font-selected")?;let (saved,error)=services::load(&path);ensure!(error.is_empty(),error);ensure!(saved.fonts.lyrics==selected_font,"Fonts not saved");ensure!((saved.library_columns.song-0.56).abs()<0.015,"Column width not saved");test.borrow_mut().prepare_paths(vec![root.join("subtitle.mp4")]);},
    7=>{if !view.has_video{return Ok(());}w.invoke_dismiss_toast();test.borrow_mut().engine_mut().submit(PlaybackCommand::AddSubtitle(root.join("subtitle.srt")))?;test.borrow_mut().engine_mut().submit(PlaybackCommand::Pause)?;},
    8=>{ensure!(w.get_render_frames()>0 && w.get_render_error().is_empty(),"Subtitle fixture not rendered");capture(&w,"subtitle-srt")?;test.borrow_mut().engine_mut().submit(PlaybackCommand::AddSubtitle(root.join("subtitle.ass")))?;},
    9=>{capture(&w,"subtitle-ass")?;control(&test,"font-ass","no");},
    10=>{ensure!(test.borrow_mut().engine_mut().snapshot().subtitle_font_overrides.is_empty(),"ASS default not restored");test.borrow_mut().dispatch(UiAction::Navigate(0));w.set_window_controls_left(true);},
    11=>{capture(&w,"left-controls")?;click(&w,103.,18.);w.set_window_controls_left(false);},
    12=>{ensure!(w.window().is_maximized(),"Native maximize click failed");ensure!(surface.state.preference==Some(1),"Maximized corners not reset");click(&w,size.width-67.,18.);},
    13=>{ensure!(!w.window().is_maximized(),"Native restore click failed");ensure!(surface.state.preference==Some(2),"Restored rounding not applied");click(&w,size.width-107.,18.);},
    14=>{ensure!(w.window().is_minimized(),"Native minimize click failed");w.window().set_minimized(false);},
    15=>{capture(&w,"buttons-scrollbar")?;click(&w,size.width-66.,507.);},
    16=>{ensure!(view.library.rows.len()==79,"Library removal failed");control(&test,"library-play",&view.library.rows[0].id.to_string());},
    17=>{if view.has_video || view.audio.lyrics.is_empty(){return Ok(());}ensure!(!w.get_toast_visible(),"Playing song showed settings toast");test.borrow_mut().engine_mut().submit(PlaybackCommand::Pause)?;control(&test,"music-detail","");},
    18=>{ensure!(view.page==4,"Music detail missing");ensure!(w.get_lyric_font().as_str()==selected_font,"Lyric font not projected");capture(&w,"lyric-font")?;click(&w,44.,size.height-40.);},
    19=>{ensure!(view.page==0,"Bottom artwork did not return to library");click(&w,44.,size.height-40.);},
    20=>{ensure!(view.page==4,"Bottom artwork did not expand music");click(&w,size.width*0.27,size.height*0.4);},
    21=>{ensure!(view.page==0,"Large artwork did not return to library");capture(&w,"library-covers")?;
      records.push(serde_json::json!({"result":"PASS","stages":22,"native_corner_preference":surface.state.preference,"native_corner_hresult":surface.state.result,"silent_play_save":true,"cover_navigation":true,"visible_thumbnails":true,"font_families":font_count,"subtitle_font":selected_font,"column_song":view.column_song,"native_window":"maximize/restore/minimize/close","library_rows":view.library.rows.len()}));std::fs::write(root.join("results.json"),serde_json::to_vec_pretty(&records)?)?;success.set(true);click(&w,size.width-29.,18.);},
    _=>unreachable!(),
   }
   step+=1;last=Instant::now();Ok(())
  })();if let Err(e)=run{*failed.borrow_mut()=Some(format!("stage {step}: {e}"));let _=slint::quit_event_loop();}
 });
    ui.run()?;
    timer.stop();
    drop(timer);
    ui.component().window().hide()?;
    drop(ui);
    c.borrow_mut().engine_mut().finish()?;
    c.borrow_mut().finish_services();
    if let Some(e) = failure.borrow_mut().take() {
        return Err(e.into());
    }
    if !complete.get() {
        return Err("Validation interrupted".into());
    }
    println!(
        "PASS: fonts / column drag and save / SRT+ASS / native window controls / scrolling layout"
    );
    Ok(())
}
