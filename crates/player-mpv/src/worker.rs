use crate::ffi::{self, Api, Handle};
use crossbeam_channel::{Receiver, Sender, bounded};
use player_core::state::{AudioDevice, Chapter, MediaTrack};
use player_core::{
    EngineCapabilities, EngineError, MediaSource, PlaybackCommand, PlaybackEngine, PlaybackPhase,
    PlaybackSnapshot,
};
use serde_json::Value;
use std::ffi::{CStr, CString, c_void};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, AtomicU8, Ordering},
};
use std::thread::JoinHandle;
use std::time::Duration;
mod audio;

/// Address is opaque outside renderer creation. Atomic lease keeps worker-owned core
/// and library alive until render_free; no Send/Sync assertion for raw pointers.
pub struct RenderBridge {
    pub api: Arc<Api>,
    address: usize,
    state: AtomicU8,
}
impl RenderBridge {
    pub fn acquire(&self) -> Option<Handle> {
        self.state
            .compare_exchange(1, 3, Ordering::AcqRel, Ordering::Acquire)
            .ok()
            .map(|_| self.address as Handle)
    }
    pub fn ready(&self) {
        self.state.fetch_or(8, Ordering::AcqRel);
    }
    pub fn release(&self) {
        self.state.fetch_and(!(2 | 8), Ordering::AcqRel);
    }
}

type Endpoint = Arc<Mutex<Option<Arc<RenderBridge>>>>;
#[derive(Default)]
pub struct MpvEngine {
    snapshot: PlaybackSnapshot,
    latest: Arc<Mutex<PlaybackSnapshot>>,
    endpoint: Endpoint,
    commands: Option<Sender<PlaybackCommand>>,
    shutdown: Arc<AtomicBool>,
    worker: Option<JoinHandle<()>>,
}
impl MpvEngine {
    pub fn start() -> Self {
        let (tx, rx) = bounded(64);
        let mut engine = Self::default();
        engine.commands = Some(tx);
        let latest = engine.latest.clone();
        let endpoint = engine.endpoint.clone();
        let shutdown = engine.shutdown.clone();
        engine.worker = Some(std::thread::spawn(move || {
            if let Err(error) = run(rx, latest.clone(), endpoint, shutdown) {
                let mut snapshot = latest.lock().unwrap();
                snapshot.error = error;
                snapshot.phase = PlaybackPhase::Error;
                snapshot.revision += 1;
            }
        }));
        engine
    }
    pub fn render_endpoint(&self) -> Endpoint {
        self.endpoint.clone()
    }
    pub fn refresh(&mut self) -> bool {
        let latest = self.latest.lock().unwrap();
        if latest.revision == self.snapshot.revision {
            return false;
        }
        self.snapshot = latest.clone();
        true
    }
    pub fn finish(&mut self) -> Result<(), String> {
        self.shutdown.store(true, Ordering::Release);
        if let Some(worker) = self.worker.take() {
            for _ in 0..100 {
                if worker.is_finished() {
                    return worker.join().map_err(|_| "播放线程异常退出".into());
                }
                std::thread::sleep(Duration::from_millis(50));
            }
            // Never destroy a core while the GL renderer still holds its lease.
            return Err("渲染资源尚未释放，播放线程正在等待安全清理".into());
        }
        Ok(())
    }
}
impl Drop for MpvEngine {
    fn drop(&mut self) {
        self.shutdown.store(true, Ordering::Release);
    }
}
impl PlaybackEngine for MpvEngine {
    fn capabilities(&self) -> EngineCapabilities {
        EngineCapabilities {
            playback: self.snapshot.ready,
            video: self.snapshot.ready,
            audio_device_selection: self.snapshot.ready,
            exclusive_audio: self.snapshot.ready && cfg!(windows),
            equalizer: self.snapshot.ready,
        }
    }
    fn snapshot(&self) -> &PlaybackSnapshot {
        &self.snapshot
    }
    fn submit(&mut self, command: PlaybackCommand) -> Result<(), EngineError> {
        if matches!(command, PlaybackCommand::Shutdown) {
            self.shutdown.store(true, Ordering::Release);
            return Ok(());
        }
        self.commands
            .as_ref()
            .ok_or(EngineError::NotConnected)?
            .try_send(command)
            .map_err(|e| EngineError::Failed(format!("播放命令暂未提交：{e}")))
    }
}

struct Core {
    api: Arc<Api>,
    handle: Handle,
    wake: Box<Sender<()>>,
    bridge: Arc<RenderBridge>,
    request: u64,
    audio: audio::AudioState,
}
impl Drop for Core {
    fn drop(&mut self) {
        self.bridge.state.fetch_or(4, Ordering::AcqRel);
        while self.bridge.state.load(Ordering::Acquire) & 2 != 0 {
            std::thread::sleep(Duration::from_millis(10));
        }
        // SAFETY: callback context outlives unregistration and destruction; all
        // render leases were released, ordinary calls exclusively belonged to this worker.
        unsafe {
            (self.api.wakeup)(self.handle, None, std::ptr::null_mut());
            (self.api.destroy)(self.handle);
        }
    }
}
unsafe extern "C" fn wakeup(context: *mut c_void) {
    // SAFETY: Box<Sender> stays alive until callback removal and core destruction.
    let _ = unsafe { (&*(context as *const Sender<()>)).try_send(()) };
}
impl Core {
    fn check(&self, code: i32) -> Result<(), String> {
        if code < 0 {
            Err(self.api.error(code))
        } else {
            Ok(())
        }
    }
    fn set(&self, name: &str, value: &str) -> Result<(), String> {
        let name = CString::new(name).map_err(|e| e.to_string())?;
        let value = CString::new(value).map_err(|e| e.to_string())?;
        // SAFETY: initialized live core, NUL strings valid for synchronous worker call.
        self.check(unsafe { (self.api.set)(self.handle, name.as_ptr(), value.as_ptr()) })
    }
    fn get(&self, name: &str) -> String {
        let name = CString::new(name).unwrap();
        // SAFETY: copy and free mpv-owned property before any other call invalidates data.
        unsafe {
            let result = (self.api.get_string)(self.handle, name.as_ptr());
            if result.is_null() {
                return String::new();
            }
            let value = CStr::from_ptr(result).to_string_lossy().into_owned();
            (self.api.free)(result.cast());
            value
        }
    }
    fn node(&self, name: &str) -> Value {
        let name = CString::new(name).unwrap();
        let mut node = ffi::Node {
            format: 0,
            data: ffi::NodeData { integer: 0 },
        };
        // SAFETY: node is initialized, only access after successful get, and free once.
        unsafe {
            if (self.api.get)(
                self.handle,
                name.as_ptr(),
                6,
                (&mut node as *mut ffi::Node).cast(),
            ) < 0
            {
                return Value::Null;
            }
            let value = ffi::node_value(&node, 0);
            (self.api.free_node)(&mut node);
            value
        }
    }
    fn command(&mut self, args: &[String]) -> Result<(), String> {
        let strings = args
            .iter()
            .map(|arg| CString::new(arg.as_str()))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;
        let mut pointers: Vec<_> = strings.iter().map(|arg| arg.as_ptr()).collect();
        pointers.push(std::ptr::null());
        self.request = self.request.wrapping_add(1).max(1);
        // SAFETY: async API copies this NULL-terminated argument vector before return.
        self.check(unsafe { (self.api.command)(self.handle, self.request, pointers.as_ptr()) })
    }
    fn snapshot(&self, snapshot: &mut PlaybackSnapshot) {
        let number = |name| self.get(name).parse::<f64>().ok().filter(|v| v.is_finite());
        snapshot.position = number("time-pos")
            .filter(|v| *v >= 0.0)
            .map(Duration::from_secs_f64);
        snapshot.duration = number("duration")
            .filter(|v| *v >= 0.0)
            .map(Duration::from_secs_f64);
        snapshot.speed = number("speed").unwrap_or(1.0);
        snapshot.volume = number("volume").unwrap_or(70.0) as f32;
        snapshot.muted = self.get("mute") == "yes";
        snapshot.title = self.get("media-title");
        snapshot.audio_output = Some(self.get("audio-device"));
        snapshot.hwdec = self.get("hwdec-current");
        let video = self.get("video-codec");
        snapshot.video = self.node("track-list").as_array().is_some_and(|tracks| {
            tracks
                .iter()
                .any(|t| t["type"] == "video" && t["albumart"] != true)
        });
        if matches!(
            snapshot.phase,
            PlaybackPhase::Playing | PlaybackPhase::Paused
        ) {
            snapshot.phase = if self.get("pause") == "yes" {
                PlaybackPhase::Paused
            } else {
                PlaybackPhase::Playing
            };
        }
        snapshot.devices = self
            .node("audio-device-list")
            .as_array()
            .into_iter()
            .flatten()
            .filter(|item| {
                !cfg!(windows)
                    || text(item, "name") == "auto"
                    || text(item, "name").starts_with("wasapi/")
            })
            .map(|item| AudioDevice {
                id: text(item, "name"),
                name: if text(item, "name") == "auto" {
                    "系统默认".into()
                } else {
                    text(item, "description")
                },
            })
            .collect();
        snapshot.tracks = self
            .node("track-list")
            .as_array()
            .into_iter()
            .flatten()
            .map(|item| {
                let id = item["id"].as_i64().unwrap_or(0);
                let kind = text(item, "type");
                MediaTrack {
                    id,
                    kind: kind.clone(),
                    selected: item["selected"].as_bool().unwrap_or(false),
                    label: format!(
                        "#{id} {} {} {}",
                        text(item, "title"),
                        text(item, "lang"),
                        text(item, "codec")
                    )
                    .trim()
                    .to_owned(),
                }
            })
            .collect();
        snapshot.chapters = self
            .node("chapter-list")
            .as_array()
            .into_iter()
            .flatten()
            .enumerate()
            .map(|(i, item)| Chapter {
                title: if text(item, "title").is_empty() {
                    format!("章节 {}", i + 1)
                } else {
                    text(item, "title")
                },
                seconds: item["time"].as_f64().unwrap_or(0.0),
            })
            .collect();
        let decoded = if snapshot.hwdec.is_empty() {
            "未知 / 未初始化"
        } else if snapshot.hwdec == "no" {
            "软件解码"
        } else {
            "硬件解码（具体路径见下方）"
        };
        snapshot.info = format!(
            "文件：{}\n容器：{}\n时长：{} 秒\n视频 codec：{}\n解码器：{}\n分辨率：{} × {}\n帧率：{}\n像素格式：{}\n色彩：{} / {} / {}\n当前解码：{}\nhwdec-current：{}\n请求 hwdec：{}\n音频 codec：{}\n采样率 / 声道：{} / {}\n音频输出：{}\n掉帧：{}\n倍速：{:.2}x\n内核：{}\n呈现：libmpv OpenGL → RGBA8 GPU 纹理（SDR，不宣称 HDR 输出）",
            self.get("path"),
            self.get("file-format"),
            self.get("duration"),
            video,
            self.get("video-format"),
            self.get("width"),
            self.get("height"),
            self.get("container-fps"),
            self.get("video-params/pixelformat"),
            self.get("video-params/colormatrix"),
            self.get("video-params/primaries"),
            self.get("video-params/gamma"),
            decoded,
            snapshot.hwdec,
            self.get("hwdec"),
            self.get("audio-codec-name"),
            self.get("audio-params/samplerate"),
            self.get("audio-params/channel-count"),
            self.get("current-ao"),
            self.get("decoder-frame-drop-count"),
            snapshot.speed,
            snapshot.runtime
        );
    }
}
fn text(item: &Value, key: &str) -> String {
    item[key].as_str().unwrap_or("").into()
}
fn source_string(source: &MediaSource) -> Result<String, String> {
    match source {
        MediaSource::Local(path) => path
            .to_str()
            .map(str::to_owned)
            .ok_or("此内核适配尚不支持非 UTF-8 媒体路径".into()),
        MediaSource::Url(url) => Ok(url.clone()),
    }
}
fn run(
    commands: Receiver<PlaybackCommand>,
    latest: Arc<Mutex<PlaybackSnapshot>>,
    endpoint: Endpoint,
    shutdown: Arc<AtomicBool>,
) -> Result<(), String> {
    let path = crate::loader::runtime_path()?;
    let api = Arc::new(Api::load(&path)?);
    // SAFETY: no handle needed for ABI query; create returns a worker-owned opaque core.
    let version = unsafe { (api.version)() };
    if version >> 16 != 2 {
        return Err(format!("需要 libmpv client API 2，实际 {version}"));
    }
    let handle = unsafe { (api.create)() };
    if handle.is_null() {
        return Err("libmpv 创建失败".into());
    }
    let (wake_tx, wake_rx) = bounded(1);
    let bridge = Arc::new(RenderBridge {
        api: api.clone(),
        address: handle as usize,
        state: AtomicU8::new(1),
    });
    let mut core = Core {
        api,
        handle,
        bridge: bridge.clone(),
        wake: Box::new(wake_tx),
        request: 0,
        audio: Default::default(),
    };
    for (name, value) in [
        ("config", "no"),
        ("vo", "libmpv"),
        ("osc", "no"),
        ("input-default-bindings", "no"),
        ("input-vo-keyboard", "no"),
        ("terminal", "no"),
        ("idle", "yes"),
        ("keep-open", "yes"),
        ("volume-max", "100"),
        ("hwdec", "auto-safe"),
        ("audio-pitch-correction", "yes"),
        ("screenshot-format", "png"),
        ("replaygain", "no"),
        ("audio-channels", "auto"),
        ("audio-display", "no"),
        ("audio-samplerate", "0"),
        ("audio-fallback-to-null", "no"),
        ("stop-playback-on-init-failure", "yes"),
        ("msg-level", "all=warn,ao/wasapi=debug"),
    ] {
        let name = CString::new(name).unwrap();
        let value = CString::new(value).unwrap();
        // SAFETY: all initialization on worker; checked required options.
        core.check(unsafe { (core.api.option)(handle, name.as_ptr(), value.as_ptr()) })?;
    }
    #[cfg(windows)]
    core.check(unsafe { (core.api.option)(handle, c"ao".as_ptr(), c"wasapi".as_ptr()) })?;
    core.check(unsafe { (core.api.initialize)(handle) })?;
    // SAFETY: static NUL string, worker-owned initialized handle; log payloads copied below.
    core.check(unsafe { (core.api.request_logs)(handle, c"terminal-default".as_ptr()) })?;
    let wake_ptr = (&mut *core.wake as *mut Sender<()>).cast();
    unsafe {
        (core.api.wakeup)(handle, Some(wakeup), wake_ptr);
    }
    for (id, property) in [
        "pause",
        "time-pos",
        "duration",
        "volume",
        "speed",
        "track-list",
        "audio-device-list",
        "hwdec-current",
        "eof-reached",
    ]
    .iter()
    .enumerate()
    {
        let name = CString::new(*property).unwrap();
        core.check(unsafe { (core.api.observe)(handle, id as u64 + 1, name.as_ptr(), 0) })?;
    }
    *endpoint.lock().unwrap() = Some(bridge);
    let mut snapshot = PlaybackSnapshot {
        ready: true,
        runtime: core.get("mpv-version"),
        ..Default::default()
    };
    let mut pending_resume = None;
    let mut pending_load = None;
    let heartbeat = crossbeam_channel::tick(Duration::from_millis(250));
    while !shutdown.load(Ordering::Acquire) {
        crossbeam_channel::select! {
            recv(commands) -> command => match command { Ok(command) => {
                if matches!(command, PlaybackCommand::Load { .. } | PlaybackCommand::Open(_)) && core.bridge.state.load(Ordering::Acquire) & 8 == 0 { pending_load = Some(command); }
                else if let Err(error) = apply(&mut core, command, &mut snapshot, &mut pending_resume) { snapshot.error = error; }
            }, Err(_) => break },
            recv(wake_rx) -> _ => {},
            recv(heartbeat) -> _ => {},
        }
        if core.bridge.state.load(Ordering::Acquire) & 8 != 0
            && let Some(command) = pending_load.take()
            && let Err(error) = apply(&mut core, command, &mut snapshot, &mut pending_resume)
        {
            snapshot.error = error;
        }
        loop {
            // SAFETY: events are copied/used before the next wait_event, worker exclusive.
            let event = unsafe { &*(core.api.wait)(handle, 0.0) };
            if event.id == 0 {
                break;
            }
            match event.id {
                2 if !event.data.is_null() => {
                    // SAFETY: the event payload matches client.h and is copied before wait_event.
                    let log = unsafe { &*(event.data as *const ffi::LogMessage) };
                    let prefix = unsafe { CStr::from_ptr(log.prefix) }.to_string_lossy();
                    if prefix.starts_with("ao/wasapi") {
                        let message = unsafe { CStr::from_ptr(log.text) }.to_string_lossy();
                        core.audio.log(&message, log.log_level);
                    }
                }
                6 => {
                    snapshot.phase = PlaybackPhase::Loading;
                    snapshot.generation += 1;
                    snapshot.error.clear();
                }
                8 => {
                    let actual = core.get("path");
                    if snapshot
                        .source
                        .as_ref()
                        .and_then(|source| source_string(source).ok())
                        .is_some_and(|expected| !same_media_path(&expected, &actual))
                    {
                        continue;
                    }
                    snapshot.phase = PlaybackPhase::Playing;
                    if let Some((expected, position, paused)) = pending_resume.as_ref()
                        && same_media_path(expected, &actual)
                        && let Err(e) = core
                            .command(&[
                                "seek".into(),
                                position.to_string(),
                                "absolute+exact".into(),
                            ])
                            .and_then(|_| {
                                core.set(
                                    "pause",
                                    if core.audio.restore_pause.is_some() || *paused {
                                        "yes"
                                    } else {
                                        "no"
                                    },
                                )
                            })
                    {
                        snapshot.error = e;
                    }
                    if pending_resume
                        .as_ref()
                        .is_some_and(|(expected, _, _)| same_media_path(expected, &actual))
                    {
                        pending_resume = None;
                    }
                }
                7 => {
                    if !event.data.is_null() {
                        let end = unsafe { &*(event.data as *const ffi::EndFile) };
                        if end.reason == 4 {
                            snapshot.error = core.api.error(end.error);
                            snapshot.phase = PlaybackPhase::Error;
                        } else if end.reason == 0 {
                            snapshot.phase = PlaybackPhase::Ended;
                        }
                    }
                }
                5 if event.error < 0 => {
                    snapshot.error = format!(
                        "命令 #{} 失败：{}",
                        event.userdata,
                        core.api.error(event.error)
                    )
                }
                _ => {}
            }
        }
        if core.get("eof-reached") == "yes"
            && matches!(
                snapshot.phase,
                PlaybackPhase::Playing | PlaybackPhase::Paused
            )
        {
            snapshot.phase = PlaybackPhase::Ended;
        }
        core.snapshot(&mut snapshot);
        if let Err(error) = core.audio_tick(&mut snapshot, &mut pending_resume) {
            snapshot.error = error;
        }
        snapshot.revision += 1;
        *latest.lock().unwrap() = snapshot.clone();
    }
    *endpoint.lock().unwrap() = None;
    Ok(())
}
fn apply(
    core: &mut Core,
    command: PlaybackCommand,
    snapshot: &mut PlaybackSnapshot,
    resume: &mut Option<(String, f64, bool)>,
) -> Result<(), String> {
    use PlaybackCommand::*;
    let arguments: Vec<String> = match command {
        Load {
            source,
            decode,
            resume: restore,
        } => {
            decode.validate()?;
            if !cfg!(windows) && core.audio.exclusive_requested() {
                return Err("此平台独占输出尚未验证，请选择共享模式后重试".into());
            }
            core.audio.blocked = false;
            core.audio.started = Some(std::time::Instant::now());
            core.audio.resume = Some(restore.unwrap_or((0.0, false)));
            core.audio.restore_pause = if cfg!(windows) && core.audio.exclusive_requested() {
                Some(restore.is_some_and(|(_, paused)| paused))
            } else {
                None
            };
            core.set("hwdec", decode.hwdec())?;
            core.set("vd-lavc-threads", &decode.threads.to_string())?;
            core.set("deinterlace", if decode.deinterlace { "yes" } else { "no" })?;
            core.set("deband", if decode.deband { "yes" } else { "no" })?;
            core.set(
                "pause",
                if core.audio.restore_pause.is_some() || restore.is_some_and(|(_, paused)| paused) {
                    "yes"
                } else {
                    "no"
                },
            )?;
            *resume = restore
                .map(|(position, paused)| {
                    source_string(&source).map(|path| (path, position, paused))
                })
                .transpose()?;
            snapshot.source = Some(source.clone());
            snapshot.phase = PlaybackPhase::Loading;
            snapshot.error.clear();
            vec!["loadfile".into(), source_string(&source)?, "replace".into()]
        }
        Open(source) => {
            return apply(
                core,
                Load {
                    source,
                    decode: Default::default(),
                    resume: None,
                },
                snapshot,
                resume,
            );
        }
        Pause => {
            if core.audio.restore_pause.is_some() {
                core.audio.restore_pause = Some(true);
            }
            return core.set("pause", "yes");
        }
        Resume => {
            if core.audio.blocked {
                return Err("输出失败 / 格式策略已暂停，请重新选择输出模式或设备".into());
            }
            if core.audio.restore_pause.is_some() {
                core.audio.restore_pause = Some(false);
                return Ok(());
            }
            return core.set("pause", "no");
        }
        Stop => {
            core.audio.restore_pause = None;
            snapshot.phase = PlaybackPhase::Idle;
            snapshot.source = None;
            *resume = None;
            vec!["stop".into()]
        }
        Seek(position) => vec![
            "seek".into(),
            position.as_secs_f64().to_string(),
            "absolute+exact".into(),
        ],
        SeekRelative(seconds) if seconds.is_finite() => {
            vec!["seek".into(), seconds.to_string(), "relative+exact".into()]
        }
        SetSpeed(speed) if speed.is_finite() && (0.1..=8.0).contains(&speed) => {
            return core.set("speed", &speed.to_string());
        }
        SetVolume(volume) if volume.is_finite() && (0.0..=100.0).contains(&volume) => {
            return core.set("volume", &volume.to_string());
        }
        SetMute(muted) => return core.set("mute", if muted { "yes" } else { "no" }),
        SetDevice(id) => return core.set("audio-device", &id),
        ApplyAudioRequest(request) => return core.apply_audio(request, snapshot, resume),
        ApplyEq(preset) => {
            let filter = preset.filter()?;
            let old = snapshot.eq_filter.clone();
            if let Err(error) = core.set("af", &filter) {
                let rollback = core.set("af", &old);
                return Err(format!("EQ 未应用：{error}；恢复旧设置：{rollback:?}"));
            }
            snapshot.eq_filter = filter;
            return Ok(());
        }
        SetTrack { kind, id } => {
            if !["aid", "sid", "vid"].contains(&kind.as_str()) {
                return Err("非法轨道类型".into());
            }
            return core.set(&kind, &id);
        }
        AddSubtitle(path) => vec![
            "sub-add".into(),
            source_string(&MediaSource::Local(path))?,
            "select".into(),
        ],
        SetSubtitleDelay(delay) if delay.is_finite() && delay.abs() <= 600.0 => {
            return core.set("sub-delay", &delay.to_string());
        }
        SetAudioDelay(delay) if delay.is_finite() && delay.abs() <= 600.0 => {
            return core.set("audio-delay", &delay.to_string());
        }
        SetLoop(enabled) => return core.set("loop-file", if enabled { "inf" } else { "no" }),
        FrameStep(backward) => vec![
            if backward {
                "frame-back-step"
            } else {
                "frame-step"
            }
            .into(),
        ],
        Screenshot(path) => vec![
            "screenshot-to-file".into(),
            source_string(&MediaSource::Local(path))?,
            "subtitles".into(),
        ],
        SetAspect(aspect) => return core.set("video-aspect-override", &aspect),
        SetAbLoop(points) => {
            let (a, b) = points
                .map(|(a, b)| (a.to_string(), b.to_string()))
                .unwrap_or(("no".into(), "no".into()));
            core.set("ab-loop-a", &a)?;
            return core.set("ab-loop-b", &b);
        }
        Shutdown => return Ok(()),
        _ => return Err("命令参数不受支持或超出允许范围".into()),
    };
    core.command(&arguments)
}
fn same_media_path(expected: &str, actual: &str) -> bool {
    #[cfg(windows)]
    {
        let normalize = |path: &str| {
            path.replace(r"\\?\UNC\", r"\\")
                .trim_start_matches(r"\\?\")
                .replace('\\', "/")
                .to_lowercase()
        };
        normalize(expected) == normalize(actual)
    }
    #[cfg(not(windows))]
    {
        expected == actual
    }
}
