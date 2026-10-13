//! Actual Slint interactions for the 0.5.4 toolbar, themed choices and overlays.
//! Uses explicit layout fixtures; no engine, settings or user media are opened.
use player_ui::{AppWindow, TrackRow, UiShell};
use slint::{ComponentHandle, ModelRc, SharedString, Timer, TimerMode, VecModel};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    time::{Duration, Instant},
};

// FemtoVG snapshots taken while a native popup is active can be black.
// Keep the GPU interaction run, and allow an independent software visual pass.
fn evidence_directory() -> &'static str {
    if std::env::var_os("YYPLAYER_UI_SOFTWARE").is_some() {
        "target/ui-consistency-software"
    } else {
        "target/ui-consistency"
    }
}
fn strings(values: &[&str]) -> ModelRc<SharedString> {
    Rc::new(VecModel::from(
        values
            .iter()
            .map(|v| SharedString::from(*v))
            .collect::<Vec<_>>(),
    ))
    .into()
}
fn click(w: &AppWindow, x: f32, y: f32) {
    let position = slint::LogicalPosition::new(x, y);
    for pressed in [true, false] {
        w.window().dispatch_event(if pressed {
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
}
fn key(w: &AppWindow, text: SharedString) {
    w.window()
        .dispatch_event(slint::platform::WindowEvent::KeyPressed { text: text.clone() });
    w.window()
        .dispatch_event(slint::platform::WindowEvent::KeyReleased { text });
}
fn capture(w: &AppWindow, name: &str) -> Result<(), Box<dyn std::error::Error>> {
    let pixels = w.window().take_snapshot()?;
    image::save_buffer(
        format!("{}/{name}.png", evidence_directory()),
        pixels.as_bytes(),
        pixels.width(),
        pixels.height(),
        image::ColorType::Rgba8,
    )?;
    Ok(())
}
fn ensure(condition: bool, message: &str) -> Result<(), Box<dyn std::error::Error>> {
    if !condition {
        return Err(message.into());
    }
    Ok(())
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    if std::env::var_os("YYPLAYER_UI_SOFTWARE").is_some() {
        slint::BackendSelector::new()
            .backend_name("winit".into())
            .renderer_name("software".into())
            .select()?;
    } else {
        player_ui::select_gpu_backend()?;
    }
    std::fs::create_dir_all(evidence_directory())?;
    let ui = UiShell::new()?;
    let w = ui.component();
    w.set_page(5);
    w.set_video_library_folders(strings(&["全部视频", "自有视频目录", "单独添加的视频"]));
    w.set_device_names(strings(&["系统默认", "自有输出设备"]));
    w.set_speed_choices(strings(&["1x"]));
    let rows: ModelRc<TrackRow> = Rc::new(VecModel::from(vec![
        TrackRow {
            item_id: 17,
            title: "湖边 · 自有测试视频.mp4".into(),
            artist: "E:\\自有素材\\视频".into(),
            collection: "20.1 MB".into(),
            ..Default::default()
        },
        TrackRow {
            item_id: 18,
            title: "夜航 · 较长的标题用于检查省略和点击.flac".into(),
            artist: "E:\\自有素材\\音乐\\测试目录".into(),
            cover: 1,
            ..Default::default()
        },
    ]))
    .into();
    w.set_video_library_tracks(rows.clone());
    w.set_recent(rows);
    let opened = Rc::new(Cell::new(0));
    let open_count = opened.clone();
    let weak = w.as_weak();
    w.on_control(move |action, value| {
        let Some(w) = weak.upgrade() else {
            return;
        };
        match action.as_str() {
            "video-folder" => w.set_video_library_folder(value.parse().unwrap()),
            "device" => w.set_device_index(value.parse().unwrap()),
            "open-recent" => {
                assert_eq!(value.as_str(), "17");
                open_count.set(open_count.get() + 1);
            }
            "panel" => {
                w.set_queue_open(false);
                w.set_panel_open(!w.get_panel_open());
            }
            "queue-close" => w.set_queue_open(false),
            _ => {}
        }
    });
    let weak = w.as_weak();
    let failed = Rc::new(RefCell::new(None));
    let failure = failed.clone();
    let complete = Rc::new(Cell::new(false));
    let completed = complete.clone();
    let mut step = 0;
    let mut records = Vec::new();
    let mut due = Instant::now() + Duration::from_millis(800);
    let mut original_width = 0.;
    let mut original_video = (0., 0.);
    let mut options_animated = false;
    let mut queue_animated = false;
    let timer = Timer::default();
    timer.start(TimerMode::Repeated, Duration::from_millis(30), move || {
        let Some(w) = weak.upgrade() else { return; };
        let options_progress = w.get_options_opacity();
        let queue_progress = (w.window().size().to_logical(w.window().scale_factor()).width - w.get_drawer_x()) / (w.get_drawer_width() + 12.);
        options_animated |= w.get_panel_open() && options_progress > 0.01 && options_progress < 0.99;
        queue_animated |= w.get_queue_open() && queue_progress > 0.01 && queue_progress < 0.99;
        if Instant::now() < due { return; }
        let size = w.window().size().to_logical(w.window().scale_factor());
        let left = w.get_content_left();
        let mut run = || -> Result<(), Box<dyn std::error::Error>> {
            match step {
                0 => { capture(&w,"video-wide")?; click(&w,left+24.+264.+60.,149.); }
                1 => { capture(&w,"video-dropdown-wide")?; key(&w,slint::platform::Key::Escape.into()); key(&w,slint::platform::Key::DownArrow.into()); }
                2 => { ensure(w.get_shortcuts_blocked(),"Choice focus did not block playback shortcuts")?; ensure(w.get_video_library_folder()==1,"Toolbar choice did not dispatch its new index")?; key(&w,slint::platform::Key::UpArrow.into()); w.window().set_size(slint::LogicalSize::new(1000.,640.)); }
                3 => { ensure(w.get_video_library_folder()==0,"Keyboard choice did not update")?; ensure(size.width<=1000.5,"Video toolbar stretched the minimum window")?; capture(&w,"video-minimum")?; click(&w,left+24.+264.+40.,149.); }
                4 => { capture(&w,"video-dropdown-minimum")?; click(&w,left+24.+264.+40.,256.); }
                5 => { ensure(w.get_video_library_folder()==2,"Pointer choice at the import-button height failed")?; key(&w,slint::platform::Key::Escape.into()); w.set_page(2); }
                6 => { ensure(!w.get_shortcuts_blocked(),"Changing page retained choice focus")?; capture(&w,"recent-minimum")?; original_width=w.get_content_width(); click(&w,left+130.,192.); }
                7 => { ensure(opened.get()==1,"Recent row click failed")?; w.set_panel_open(true); }
                8 => {
                    ensure((w.get_content_width()-original_width).abs()<1.,"Options resized the recent page")?;
                    ensure((size.width-w.get_options_x()-w.get_options_width()-12.).abs()<1.,"Options drawer right margin differs")?;
                    capture(&w,"recent-options")?;
                    click(&w,w.get_options_x()+100.,w.get_options_y()+161.);
                }
                9 => { capture(&w,"options-dropdown")?; key(&w,slint::platform::Key::Escape.into()); key(&w,slint::platform::Key::DownArrow.into()); }
                10 => { ensure(w.get_device_index()==1,"Themed options choice did not dispatch its new index")?; click(&w,left+130.,192.); }
                11 => { ensure(!w.get_panel_open()&&opened.get()==1,"Options backdrop leaked its click to the recent row")?; key(&w, slint::platform::Key::UpArrow.into());
                    w.set_queue_open(true); }
                12 => {
                    ensure(w.get_device_index() == 1, "Hidden options retained keyboard input")?;
                    ensure((w.get_drawer_y()-w.get_options_y()).abs()<1.&&(w.get_drawer_height()-w.get_options_height()).abs()<1.,"Drawer vertical geometry differs")?;
                    ensure((size.width-w.get_drawer_x()-w.get_drawer_width()-12.).abs()<1.,"Queue drawer right margin differs")?;
                    capture(&w,"recent-queue")?; click(&w,left+130.,192.);
                }
                13 => { ensure(!w.get_queue_open()&&opened.get()==1,"Queue backdrop leaked its click")?; w.set_appearance_reduced(true); w.set_page(1); w.set_fullscreen_mode(true); original_video=(w.get_video_width(),w.get_video_height()); w.set_panel_open(true); }
                14 => {
                    ensure(options_animated && queue_animated,"Both drawers must visibly slide before reduced motion is enabled")?;
                    ensure(w.get_options_opacity()>0.99,"Reduced-motion drawer remained closed")?;
                    ensure(original_video==(w.get_video_width(),w.get_video_height()),"Options resized fullscreen video")?;
                    capture(&w,"fullscreen-options")?;
                    w.set_panel_open(false); w.set_fullscreen_mode(false); w.set_page(5); w.set_appearance_dark(false); w.set_can_resume_video(true);
                }
                15 => { capture(&w,"video-light-resume")?; click(&w,left+24.+264.+40.,149.); }
                16 => { capture(&w,"video-dropdown-light")?; key(&w,slint::platform::Key::Escape.into()); w.set_appearance_design(0); w.set_page(2); }
                17 => { capture(&w,"recent-simple-light")?; completed.set(true); slint::quit_event_loop()?; }
                _ => unreachable!(),
            }
            records.push(serde_json::json!({"stage":step,"window_width":size.width,"content_width":w.get_content_width(),"folder":w.get_video_library_folder(),"device":w.get_device_index(),"options_x":w.get_options_x(),"options_y":w.get_options_y(),"options_height":w.get_options_height(),"queue_x":w.get_drawer_x(),"queue_y":w.get_drawer_y(),"queue_height":w.get_drawer_height(),"opened_recent":opened.get(),"shortcuts_blocked":w.get_shortcuts_blocked(),"options_animated":options_animated,"queue_animated":queue_animated}));
            std::fs::write(format!("{}/results.json", evidence_directory()),serde_json::to_vec_pretty(&records)?)?;
            Ok(())
        };
        if let Err(e)=run() { *failure.borrow_mut()=Some(format!("stage {step}: {e}")); let _=slint::quit_event_loop(); }
        step+=1; due=Instant::now()+Duration::from_millis(600);
    });
    ui.run()?;
    if let Some(e) = failed.borrow_mut().take() {
        return Err(e.into());
    }
    ensure(complete.get(), "UI consistency regression interrupted")?;
    println!(
        "PASS: 18 actual toolbar / dropdown / recent / drawer / backdrop / fullscreen-size / theme stages"
    );
    Ok(())
}
