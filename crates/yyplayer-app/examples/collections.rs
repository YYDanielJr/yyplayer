//! Production controller / libmpv / GPU and actual drawer clicks, own fixtures only.
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
fn control(c: &mut controller::AppController, a: &str, v: &str) {
    c.dispatch(UiAction::Control(a.into(), v.into()));
}
fn click(w: &player_ui::AppWindow, x: f32, y: f32) {
    for pressed in [true, false] {
        let position = slint::LogicalPosition::new(x, y);
        let button = slint::platform::PointerEventButton::Left;
        w.window().dispatch_event(if pressed {
            slint::platform::WindowEvent::PointerPressed { position, button }
        } else {
            slint::platform::WindowEvent::PointerReleased { position, button }
        });
    }
}
fn capture(w: &player_ui::AppWindow, name: &str) -> Result<(), Box<dyn std::error::Error>> {
    let p = w.window().take_snapshot()?;
    image::save_buffer(
        format!("target/collections/{name}.png"),
        p.as_bytes(),
        p.width(),
        p.height(),
        image::ColorType::Rgba8,
    )?;
    Ok(())
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let base = std::env::current_dir()?.join("target/collections");
    let path = services::settings_path();
    if !path.starts_with(&base) {
        return Err("Use target/collections/settings.json".into());
    }
    let mut settings = player_core::settings::Settings::default();
    settings
        .library
        .roots
        .push(base.join("music").canonicalize()?);
    settings
        .video_library
        .roots
        .push(base.join("video").canonicalize()?);
    settings.online.history = vec![
        "jixwang".into(),
        "monsters".into(),
        "远航星的告别".into(),
        "radiant".into(),
        "一条较长的历史搜索记录，用于验证小窗口".into(),
    ];
    settings.online.music_enabled = true;
    services::atomic_save(&path, &settings)?;
    player_ui::select_gpu_backend()?;
    let c = Rc::new(RefCell::new(controller::AppController::live(
        player_mpv::MpvEngine::start(),
        player_platform::current(),
    )));
    let ui = UiShell::new()?;
    player_ui::presenter::attach(
        ui.component(),
        c.borrow_mut().engine_mut().render_endpoint(),
    )?;
    let state = c.clone();
    let project = ui.projector();
    let dispatch = Rc::new(move |a| {
        state.borrow_mut().dispatch(a);
        project(&state.borrow().view_model());
    });
    ui.bind(dispatch.clone());
    ui.bind_controls(dispatch);
    ui.project(&c.borrow().view_model());
    let weak = ui.component().as_weak();
    let project = ui.projector();
    let tick = c.clone();
    let error = Rc::new(RefCell::new(None::<String>));
    let failed = error.clone();
    let done = Rc::new(Cell::new(false));
    let complete = done.clone();
    let timer = Timer::default();
    let started = Instant::now();
    let mut due = started + Duration::from_millis(700);
    let mut stage = 0;
    let mut generation = 0;
    let mut records = vec![];
    timer.start(TimerMode::Repeated,Duration::from_millis(60),move||{
        let Some(w)=weak.upgrade()else{return;};let mut c=tick.borrow_mut();c.tick();c.render_failed(w.get_render_error().as_str());project(&c.view_model());
        if Instant::now()<due{return;}let mut pending=None;
        let result=(||->Result<bool,Box<dyn std::error::Error>>{
            macro_rules! ensure{($v:expr,$m:expr)=>{let passed=$v;if !passed{return Err($m.into());}};}
            let v=c.view_model();let s=c.engine_mut().snapshot().clone();let size=w.window().size().to_logical(w.window().scale_factor());
            ensure!(started.elapsed()<Duration::from_secs(120) && due.elapsed() < Duration::from_secs(12),format!("Timeout at {stage}: phase {:?}, selected {}, source {:?}, position {:?}, status {}, has_video {}, render_ready {}, output {}",s.phase,v.selected_id,s.source,s.position,v.status,v.has_video,w.get_render_ready(),s.video_output_enabled));ensure!(s.error.is_empty(),s.error.clone());ensure!(w.get_render_error().is_empty(),w.get_render_error().to_string());
            match stage {
                0=>{if !s.ready||v.library.busy||v.library.rows.len()!=4||v.video_library.rows.len()!=2{return Ok(false);}capture(&w,"songs")?;pending=Some((w.get_content_left()+24.+262.+84.,139.));},
                1=>{ensure!(v.collection.groups.len()==2,"artist classification failed");capture(&w,"artists")?;pending=Some((w.get_content_left()+180.,200.));},
                2=>{ensure!(v.library.rows.len()==2,"artist songs failed");control(&mut c,"library-check","0");control(&mut c,"library-check","1");control(&mut c,"playlist-create","夜航精选");control(&mut c,"library-category","1");control(&mut c,"library-group","0");control(&mut c,"playlist-add","1");},
                3=>{control(&mut c,"library-category","2");},
                4=>{ensure!(v.collection.groups.len()==2,"album artists were conflated");capture(&w,"albums")?;control(&mut c,"library-category","3");control(&mut c,"playlist-select","1");},
                5=>{ensure!(v.library.rows.len()==2,"playlist membership failed");capture(&w,"playlist")?;control(&mut c,"library-batch-play","");},
                6=>{if s.phase!=PlaybackPhase::Playing||s.duration.is_none(){return Ok(false);}ensure!(v.drawer.music_count==2&&!v.has_video&&w.get_render_initializations()==0,"music queue / lazy renderer failed");c.engine_mut().submit(PlaybackCommand::Seek(Duration::from_secs_f64(7.5)))?;},
                7=>{if v.selected_id!=1||s.phase!=PlaybackPhase::Playing{return Ok(false);}ensure!(v.drawer.music_count==2,"sequential EOF lost queue");c.engine_mut().submit(PlaybackCommand::Pause)?;pending=Some((size.width-35.,size.height-40.));},
                8=>{ensure!(v.drawer.open,"bottom-right queue button failed");capture(&w,"music-queue")?;control(&mut c,"queue-order","2");c.engine_mut().submit(PlaybackCommand::Seek(Duration::from_secs_f64(7.5)))?;c.engine_mut().submit(PlaybackCommand::Resume)?;generation=s.generation;},
                9=>{if s.phase!=PlaybackPhase::Playing||s.position.is_none_or(|p|p.as_secs_f64()>1.2){return Ok(false);}ensure!(v.selected_id==1&&s.generation==generation,"repeat-one reloaded or advanced");c.engine_mut().submit(PlaybackCommand::Pause)?;control(&mut c,"queue-order","1");c.engine_mut().submit(PlaybackCommand::Seek(Duration::from_secs_f64(7.5)))?;c.engine_mut().submit(PlaybackCommand::Resume)?;},
                10=>{if v.selected_id!=0||s.phase!=PlaybackPhase::Playing{return Ok(false);}c.engine_mut().submit(PlaybackCommand::Pause)?;control(&mut c,"queue-order","3");c.dispatch(UiAction::Next);},
                11=>{if v.selected_id!=1||s.phase!=PlaybackPhase::Playing{return Ok(false);}c.engine_mut().submit(PlaybackCommand::Pause)?;c.dispatch(UiAction::Previous);},
                12=>{if v.selected_id!=0||s.phase!=PlaybackPhase::Playing{return Ok(false);}c.engine_mut().submit(PlaybackCommand::Pause)?;control(&mut c,"video-enqueue-all","");},
                13=>{ensure!(v.drawer.music_count==2&&v.drawer.video_count==2&&v.drawer.tab==1,"independent video enqueue failed");capture(&w,"video-queue")?;pending=Some((w.get_drawer_x()+95.,w.get_drawer_y()+215.));},
                14=>{if !v.has_video||!w.get_render_ready()||w.get_render_frames()<3{return Ok(false);}ensure!(v.drawer.music_count==2&&v.drawer.video_count==2,"video play replaced music queue");c.engine_mut().submit(PlaybackCommand::Pause)?;capture(&w,"video-playing")?;pending=Some((w.get_drawer_x()+95.,w.get_drawer_y()+78.));},
                15=>{ensure!(v.drawer.tab==0,"actual music-tab click failed");pending=Some((w.get_drawer_x()+95.,w.get_drawer_y()+215.));},
                16=>{if v.has_video||s.phase!=PlaybackPhase::Playing||w.get_render_ready(){return Ok(false);}ensure!(v.drawer.video_count==2,"music switch lost video queue");c.engine_mut().submit(PlaybackCommand::Pause)?;w.window().set_size(slint::LogicalSize::new(1000.,640.));},
                17=>{ensure!(w.get_drawer_width()>=400.&&w.get_drawer_x()>0.&&w.get_drawer_height()>350.,"minimum drawer clipped");capture(&w,"minimum-queue")?;control(&mut c,"queue-close","");c.dispatch(UiAction::Navigate(6));},
                18=>{ensure!(w.get_content_opacity()>0.99,"search transparent");capture(&w,"history-minimum")?;w.window().set_size(slint::LogicalSize::new(1840.,1040.));},
                19=>{capture(&w,"history-wide")?;c.dispatch(UiAction::Navigate(0));control(&mut c,"library-category","3");},
                20=>{if w.get_content_opacity()<0.99{return Ok(false);}capture(&w,"playlist-wide")?;w.window().set_size(slint::LogicalSize::new(1000.,640.));},
                21=>{if w.get_content_opacity()<0.99{return Ok(false);}capture(&w,"playlist-minimum")?;control(&mut c,"playlist-rename","夜航收藏");control(&mut c,"library-check","0");control(&mut c,"playlist-remove","");},
                22=>{ensure!(v.library.rows.len()==1&&v.collection.title=="夜航收藏","playlist edit failed");let(saved,e)=services::load(&path);ensure!(e.is_empty(),e);ensure!(saved.playlists.lists[1].name=="夜航收藏"&&saved.playlists.lists[1].tracks.len()==1,"background persistence failed");ensure!(saved.music_order==player_core::queue::PlayOrder::Shuffle&&saved.video_order==player_core::queue::PlayOrder::Sequential,"independent saved modes failed");pending=Some((w.get_content_left()+24.+26.,185.));},
                23=>{ensure!(v.collection.category==0&&v.library.rows.len()==4,"minimum toolbar category click failed");capture(&w,"songs-minimum")?;control(&mut c,"panel","");},
                24=>{ensure!(v.panel_open&&size.width<=1000.5,"minimum options stretched window");ensure!(w.get_options_opacity()>0.99,"options remained transparent during live projection");capture(&w,"songs-minimum-options")?;pending=Some((w.get_content_left()+24.+142.,185.));},
                25=>{ensure!(v.collection.category==2&&v.collection.groups.len()==2,"narrow toolbar category click failed");capture(&w,"albums-minimum-options")?;control(&mut c,"panel","");w.window().set_size(slint::LogicalSize::new(1840.,1040.));},
                26=>{pending=Some((w.get_content_left()+24.+262.+26.,139.));},
                27=>{ensure!(v.collection.category==0&&v.library.rows.len()==4,"wide toolbar category click failed");capture(&w,"songs-wide")?;complete.set(true);slint::quit_event_loop()?;},
                _=>unreachable!(),
            }
            records.push(serde_json::json!({"stage":stage,"seconds":started.elapsed().as_secs_f64(),"phase":format!("{:?}",s.phase),"generation":s.generation,"selected":v.selected_id,"music":v.drawer.music_count,"video":v.drawer.video_count,"renderer_ready":w.get_render_ready(),"frames":w.get_render_frames(),"library_rows":v.library.rows.len(),"error":s.error}));println!("stage {stage}: {:?}, selected {}",s.phase,v.selected_id);std::fs::write("target/collections/progress.json",serde_json::to_vec_pretty(&records)?)?;if complete.get(){std::fs::write("target/collections/results.json",serde_json::to_vec_pretty(&records)?)?;}Ok(true)
        })();match result{Ok(true)=>{stage+=1;due=Instant::now()+Duration::from_millis(if stage==28{0}else{1000});},Ok(false)=>{},Err(e)=>{*failed.borrow_mut()=Some(format!("stage {stage}: {e}"));let _=slint::quit_event_loop();}}
        project(&c.view_model());drop(c);if let Some((x,y))=pending{click(&w,x,y);}
    });
    ui.run()?;
    drop(timer);
    drop(ui);
    c.borrow_mut().engine_mut().finish()?;
    c.borrow_mut().finish_services();
    if let Some(e) = error.borrow_mut().take() {
        return Err(e.into());
    }
    if !done.get() {
        return Err("Interrupted".into());
    }
    let mut restored =
        controller::AppController::live(player_mpv::MpvEngine::start(), player_platform::current());
    control(&mut restored, "library-category", "3");
    control(&mut restored, "playlist-select", "1");
    let v = restored.view_model();
    let ok = v.collection.title == "夜航收藏"
        && v.library.rows.len() == 1
        && v.drawer.music_count == 0
        && v.drawer.video_count == 0;
    restored.engine_mut().finish()?;
    restored.finish_services();
    if !ok {
        return Err("Restart persistence failed".into());
    }
    println!(
        "PASS: production classification / playlists / sequential + repeat-one + repeat-all + shuffle / independent queues / GPU lifecycle / drawer clicks / persistence / three sizes"
    );
    Ok(())
}
