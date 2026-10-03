//! WASAPI evidence and controlled same-device fallback. Ordinary calls stay on Engine.
use super::*;
use player_core::{AudioMode, AudioRequest};
use std::time::Instant;
#[cfg(target_os = "linux")]
#[path = "audio_linux.rs"]
mod linux;

#[derive(Default)]
pub(super) struct AudioState {
    #[cfg(target_os = "linux")]
    linux: linux::Evidence,
    request: AudioRequest,
    accepted: Option<bool>,
    initialized: bool,
    failed: bool,
    fallback: bool,
    pub(super) started: Option<Instant>,
    device: String,
    last_endpoint: String,
    endpoint: String,
    endpoint_changed: bool,
    pub(super) resume: Option<(f64, bool)>,
    pub(super) restore_pause: Option<bool>,
    pub(super) blocked: bool,
    log: std::collections::VecDeque<String>,
}
impl AudioState {
    pub(super) fn exclusive_requested(&self) -> bool {
        self.request.mode != AudioMode::Shared
    }
    fn confirmed(&self) -> Option<bool> {
        if !self.initialized || self.failed {
            return None;
        }
        let expected = self.exclusive_requested() && !self.fallback;
        if self.request.device_id != "auto" {
            let id = self
                .request
                .device_id
                .strip_prefix("wasapi/")?
                .trim_start_matches("{0.0.0.00000000}.");
            if !self
                .endpoint
                .starts_with(&format!("Selecting device '{id}'"))
            {
                return None;
            }
        }
        self.accepted.filter(|exclusive| *exclusive == expected)
    }
    pub(super) fn log(&mut self, message: &str, level: i32) {
        if self.log.len() == 32 {
            self.log.pop_front();
        }
        self.log.push_back(format!(
            "{level}: {}",
            message.trim().chars().take(1200).collect::<String>()
        ));
        if message.trim() == "Init wasapi" {
            self.accepted = None;
            self.initialized = false;
            self.failed = false;
            self.started = Some(Instant::now());
            self.device.clear();
        }
        if message.contains("Accepted as ") {
            self.accepted = if message.contains("(exclusive)") {
                Some(true)
            } else if message.contains("(shared)") {
                Some(false)
            } else {
                None
            };
        }
        if message.trim() == "Init wasapi done" {
            self.initialized = true;
            self.failed = false;
            if !self.last_endpoint.is_empty()
                && !self.endpoint.is_empty()
                && self.last_endpoint != self.endpoint
            {
                self.endpoint_changed = true;
            }
            if self.confirmed().is_some() {
                self.last_endpoint = self.endpoint.clone();
            }
        }
        if message.starts_with("Selecting device '") {
            self.endpoint = message.trim().to_owned();
            self.device = self.endpoint.clone();
        }
        if message.contains("Device name:")
            || message.contains("Device ID:")
            || message.contains("Selecting device by id:")
        {
            self.device.push_str(message.trim());
            self.device.push('\n');
        }
        if level <= 20
            && (!self.initialized
                || message.contains("INVALIDATED")
                || message.contains("Error getting device delay"))
            && (message.contains("Error")
                || message.contains("Failed")
                || message.contains("INVALIDATED"))
        {
            self.failed = true;
        }
    }
}
impl Core {
    #[cfg(target_os = "linux")]
    pub(super) fn apply_audio(
        &mut self,
        request: AudioRequest,
        snapshot: &mut PlaybackSnapshot,
        resume: &mut Option<(String, f64, bool)>,
    ) -> Result<(), String> {
        self.apply_linux_audio(request, snapshot, resume)
    }
    #[cfg(target_os = "linux")]
    pub(super) fn audio_tick(
        &mut self,
        snapshot: &mut PlaybackSnapshot,
        resume: &mut Option<(String, f64, bool)>,
    ) -> Result<(), String> {
        self.linux_audio_tick(snapshot, resume)
    }
}
impl Core {
    #[cfg(not(target_os = "linux"))]
    pub(super) fn apply_audio(
        &mut self,
        request: AudioRequest,
        snapshot: &mut PlaybackSnapshot,
        resume: &mut Option<(String, f64, bool)>,
    ) -> Result<(), String> {
        if !request.volume_percent.is_finite() || !(0.0..=100.0).contains(&request.volume_percent) {
            return Err("非法音量".into());
        }
        if request.device_id != "auto"
            && !snapshot.devices.iter().any(|d| d.id == request.device_id)
        {
            return Err("输出设备不可用，请重新选择设备".into());
        }
        if !cfg!(windows) && request.mode != AudioMode::Shared {
            self.set("pause", "yes")?;
            self.audio.request = request;
            self.audio.blocked = true;
            return Err("此平台独占输出尚未验证，当前仅启用共享输出".into());
        }
        let retry = self.audio.blocked || snapshot.phase == PlaybackPhase::Error;
        let paused = self
            .audio
            .restore_pause
            .unwrap_or_else(|| self.get("pause") == "yes");
        self.set("pause", "yes")?;
        let old_device = self.get("audio-device");
        let old_exclusive = self.get("audio-exclusive");
        let unchanged = !retry
            && self.audio.confirmed().is_some()
            && old_device == request.device_id
            && old_exclusive
                == if request.mode == AudioMode::Shared {
                    "no"
                } else {
                    "yes"
                };
        let result = self
            .set("audio-device", &request.device_id)
            .and_then(|_| {
                self.set(
                    "audio-exclusive",
                    if request.mode == AudioMode::Shared {
                        "no"
                    } else {
                        "yes"
                    },
                )
            })
            .and_then(|_| self.set("volume", &request.volume_percent.to_string()));
        if let Err(error) = result {
            let _ = self.set("audio-device", &old_device);
            let _ = self.set("audio-exclusive", &old_exclusive);
            let _ = self.set("pause", if paused { "yes" } else { "no" });
            return Err(format!("输出设置未应用：{error}"));
        }
        let previous = std::mem::take(&mut self.audio);
        self.audio = AudioState {
            request,
            resume: Some((snapshot.position.map_or(0.0, |p| p.as_secs_f64()), paused)),
            started: Some(Instant::now()),
            restore_pause: (cfg!(windows)
                && !unchanged
                && matches!(
                    snapshot.phase,
                    PlaybackPhase::Playing | PlaybackPhase::Paused
                ))
            .then_some(paused),
            ..Default::default()
        };
        if unchanged {
            self.audio.accepted = previous.accepted;
            self.audio.initialized = previous.initialized;
            self.audio.endpoint = previous.endpoint;
            self.audio.last_endpoint = previous.last_endpoint;
            self.audio.device = previous.device;
            self.audio.log = previous.log;
        }
        snapshot.exclusive_confirmed = None;
        snapshot.error.clear();
        snapshot.audio_status = "输出已请求，等待实际初始化".into();
        if retry && let Some(source) = &snapshot.source {
            self.audio.restore_pause = None;
            let path = source_string(source)?;
            *resume = Some((
                path.clone(),
                snapshot.position.map_or(0.0, |p| p.as_secs_f64()),
                true,
            ));
            self.command(&["loadfile".into(), path, "replace".into()])?;
            snapshot.phase = PlaybackPhase::Loading;
            return Ok(());
        }
        if self.audio.restore_pause.is_some() {
            Ok(())
        } else {
            self.set("pause", if paused { "yes" } else { "no" })
        }
    }
    #[cfg(not(target_os = "linux"))]
    pub(super) fn audio_tick(
        &mut self,
        snapshot: &mut PlaybackSnapshot,
        resume: &mut Option<(String, f64, bool)>,
    ) -> Result<(), String> {
        let ao = self.get("current-ao");
        snapshot.audio_log = self.audio.log.iter().cloned().collect();
        let source = self.node("audio-params");
        let output = self.node("audio-out-params");
        snapshot.audio_fallback = self.audio.fallback;
        snapshot.audio_source_rate = source["samplerate"].as_u64();
        snapshot.audio_output_rate = output["samplerate"].as_u64();
        let has_audio = !self.get("audio-codec-name").is_empty();
        if !self.audio.failed
            && !self.audio.blocked
            && self.audio.restore_pause.is_none()
            && matches!(
                snapshot.phase,
                PlaybackPhase::Playing | PlaybackPhase::Paused
            )
            && let Some(position) = snapshot.position
        {
            self.audio.resume = Some((
                position.as_secs_f64(),
                snapshot.phase == PlaybackPhase::Paused,
            ));
        }
        snapshot.exclusive_confirmed =
            if ao == "wasapi" && self.audio.initialized && !self.audio.failed {
                self.audio.confirmed()
            } else {
                None
            };
        snapshot.audio_info = format!(
            "音频 codec：{}\n源解码 PCM：{} Hz · {} · {}\nAPI 输出 PCM：{} Hz · {} · {}\n实际音频 API：{}\n请求设备：{}\n设备初始化记录：{}\n物理 DAC 格式：未知（不能由软件 PCM 格式推断）\n音量：{:.0}% · 倍速：{:.2}x\nDSP：{}",
            self.get("audio-codec-name"),
            source["samplerate"],
            source["channels"],
            source["format"],
            output["samplerate"],
            output["channels"],
            output["format"],
            ao,
            self.audio.request.device_id,
            if self.audio.device.is_empty() {
                "未知"
            } else {
                &self.audio.device
            },
            snapshot.volume,
            snapshot.speed,
            if snapshot.eq_filter.is_empty() {
                "EQ 关闭 / 平直，无 EQ 滤镜"
            } else {
                &snapshot.eq_filter
            }
        );
        snapshot
            .audio_info
            .push_str(&format!("\n实际 AF 链：{}", self.get("af")));
        if self.audio.blocked {
            return Ok(());
        }
        let disconnected = self.audio.request.device_id != "auto"
            && !snapshot
                .devices
                .iter()
                .any(|d| d.id == self.audio.request.device_id);
        if (disconnected || self.audio.endpoint_changed) && has_audio {
            self.set("pause", "yes")?;
            self.audio.blocked = true;
            snapshot.audio_status = "设备失联，已暂停；请重新选择设备".into();
            return Err(snapshot.audio_status.clone());
        }
        let timeout = cfg!(windows)
            && has_audio
            && self
                .audio
                .started
                .is_some_and(|t| t.elapsed() > Duration::from_secs(5))
            && snapshot.exclusive_confirmed.is_none();
        // An initialization error can be an intermediate buffer-alignment retry.
        // Only act after core failure, a runtime device error, or the confirmation deadline.
        let failed =
            self.audio.failed && (self.audio.initialized || snapshot.phase == PlaybackPhase::Error);
        let wrong_mode = snapshot.exclusive_confirmed == Some(false)
            && self.audio.request.mode != AudioMode::Shared
            && !self.audio.fallback;
        if failed || timeout || wrong_mode {
            if self.audio.request.mode == AudioMode::PreferExclusive
                && !self.audio.fallback
                && !disconnected
            {
                self.audio.fallback = true;
                self.audio.failed = false;
                self.audio.initialized = false;
                self.audio.accepted = None;
                self.audio.started = Some(Instant::now());
                self.set("audio-exclusive", "no")?;
                if let Some(media) = &snapshot.source {
                    let path = source_string(media)?;
                    let (position, paused) = self.audio.resume.unwrap_or((
                        snapshot.position.map_or(0.0, |p| p.as_secs_f64()),
                        snapshot.phase != PlaybackPhase::Playing,
                    ));
                    *resume = Some((path.clone(), position, paused));
                    self.command(&["loadfile".into(), path, "replace".into()])?;
                    snapshot.phase = PlaybackPhase::Loading;
                    snapshot.error.clear();
                }
            } else {
                self.set("pause", "yes")?;
                self.audio.blocked = true;
                snapshot.audio_status = "音频输出初始化失败；已暂停，未切换到其他设备".into();
                return Err(snapshot.audio_status.clone());
            }
        }
        snapshot.audio_status = match snapshot.exclusive_confirmed {
            Some(true) => "WASAPI 独占已确认".into(),
            Some(false) if self.audio.fallback => "独占失败，已回退到同一设备的共享输出".into(),
            Some(false) => "WASAPI 共享输出".into(),
            None if !has_audio => "等待音频 / 未初始化".into(),
            None => "实际独占状态未知 / 初始化中".into(),
        };
        if self.audio.request.preserve_rate
            && snapshot.exclusive_confirmed == Some(true)
            && source["samplerate"] != output["samplerate"]
        {
            self.set("pause", "yes")?;
            self.audio.blocked = true;
            snapshot.audio_status = "设备未接受源采样率；按严格源格式策略暂停".into();
            return Err(snapshot.audio_status.clone());
        }
        if snapshot.exclusive_confirmed.is_some()
            && let Some(paused) = self.audio.restore_pause.take()
        {
            self.set("pause", if paused { "yes" } else { "no" })?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn accepted_format_alone_does_not_confirm_initialization() {
        let mut audio = AudioState::default();
        audio.log("Init wasapi\n", 60);
        audio.log("Accepted as stereo s32 @ 48000hz -> PCM (exclusive)\n", 40);
        assert_eq!(audio.accepted, Some(true));
        assert!(!audio.initialized);
        audio.log(
            "Error initializing device: AUDCLNT_E_BUFFER_SIZE_NOT_ALIGNED",
            20,
        );
        assert!(audio.failed);
        audio.log("Init wasapi done\n", 60);
        assert!(audio.initialized);
        assert!(!audio.failed);
        audio.log("Init wasapi\n", 60);
        assert_eq!(audio.accepted, None);
    }
    #[test]
    fn unrequested_endpoint_change_is_not_a_transparent_fallback() {
        let mut audio = AudioState::default();
        for message in [
            "Init wasapi",
            "Selecting device 'A' (DAC)",
            "Accepted as stereo float @ 48000hz -> PCM (shared)",
            "Init wasapi done",
            "Init wasapi",
            "Selecting device 'B' (Speakers)",
            "Accepted as stereo float @ 48000hz -> PCM (shared)",
            "Init wasapi done",
        ] {
            audio.log(message, 60);
        }
        assert!(audio.endpoint_changed);
    }
    #[test]
    fn confirmation_matches_requested_mode_and_endpoint() {
        let mut audio = AudioState {
            request: AudioRequest {
                device_id: "wasapi/{A}".into(),
                mode: AudioMode::StrictExclusive,
                ..Default::default()
            },
            ..Default::default()
        };
        for message in [
            "Init wasapi",
            "Selecting device '{B}' (Other)",
            "Accepted as stereo s32 @ 44100hz -> PCM (exclusive)",
            "Init wasapi done",
        ] {
            audio.log(message, 60);
        }
        assert_eq!(audio.confirmed(), None);
        for message in [
            "Init wasapi",
            "Selecting device '{A}' (DAC)",
            "Accepted as stereo float @ 48000hz -> PCM (shared)",
            "Init wasapi done",
        ] {
            audio.log(message, 60);
        }
        assert_eq!(audio.confirmed(), None);
        for message in [
            "Init wasapi",
            "Selecting device '{A}' (DAC)",
            "Accepted as stereo s32 @ 44100hz -> PCM (exclusive)",
            "Init wasapi done",
        ] {
            audio.log(message, 60);
        }
        assert_eq!(audio.confirmed(), Some(true));
        assert!(!audio.endpoint_changed);
    }
}
