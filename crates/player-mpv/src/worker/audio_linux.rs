//! mpv 0.41 PipeWire stream and ALSA hw evidence. No UI thread or render API calls.
use super::*;
#[derive(Default)]
pub(super) struct Evidence {
    backend: String,
    opened: String,
    connected: bool,
    hw_params: bool,
    lost: bool,
}
impl AudioState {
    pub(in crate::worker) fn linux_log(&mut self, prefix: &str, message: &str, level: i32) {
        if self.log.len() == 32 {
            self.log.pop_front();
        }
        self.log.push_back(format!(
            "{prefix} {level}: {}",
            message.trim().chars().take(1200).collect::<String>()
        ));
        let backend = prefix.trim_start_matches("ao/");
        if message.starts_with("Headers version:") || message.starts_with("using ALSA version:") {
            self.linux = Evidence {
                backend: backend.into(),
                ..Default::default()
            };
            self.failed = false;
        }
        if backend == "pipewire" {
            if message
                .split_whitespace()
                .any(|f| matches!(f, "state=paused" | "state=streaming"))
            {
                self.linux.backend = backend.into();
                self.linux.connected = true;
                self.failed = false;
            }
            if self.linux.connected
                && (message.contains("Stream disconnected")
                    || message.contains("Stream in error state"))
            {
                self.linux.lost = true;
            }
        }
        if backend == "alsa" {
            if let Some(device) = message
                .strip_prefix("opening device '")
                .and_then(|s| s.split_once('\''))
            {
                self.linux.opened = device.0.into();
                self.linux.backend = backend.into();
            }
            if message.starts_with("Final HW params:") {
                self.linux.hw_params = true;
                self.failed = false;
            }
            if message.contains("Device or resource busy")
                || message.contains("No such device")
                || message.contains("device disconnected")
            {
                self.failed = true;
            }
        }
        if level <= 20
            && (message.contains("Error")
                || message.contains("error")
                || message.contains("failed"))
        {
            self.failed = true;
        }
    }
}
fn exclusive_device(device: &str) -> bool {
    device
        .strip_prefix("pipewire/")
        .is_some_and(|d| !d.is_empty())
        || device.starts_with("alsa/hw:")
}
impl Core {
    pub(in crate::worker) fn validate_linux_audio_load(
        &mut self,
        snapshot: &PlaybackSnapshot,
    ) -> Result<(), String> {
        let request = &self.audio.request;
        if (request.mode != AudioMode::Shared
            && !self.audio.fallback
            && !exclusive_device(&request.device_id))
            || (request.device_id != "auto"
                && !snapshot.devices.iter().any(|d| d.id == request.device_id))
        {
            self.set("pause", "yes")?;
            self.audio.blocked = true;
            return Err("当前输出设备或独占策略不可用；请先选择可用设备 / 共享模式再重试".into());
        }
        Ok(())
    }
    pub(super) fn apply_linux_audio(
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
            self.set("pause", "yes")?;
            self.audio = AudioState {
                request,
                blocked: true,
                ..Default::default()
            };
            snapshot.exclusive_confirmed = None;
            return Err("输出设备不可用，已暂停；请重新选择设备".into());
        }
        let unsupported =
            request.mode != AudioMode::Shared && !exclusive_device(&request.device_id);
        if unsupported && request.mode == AudioMode::StrictExclusive {
            self.set("pause", "yes")?;
            self.audio = AudioState {
                request,
                blocked: true,
                ..Default::default()
            };
            snapshot.exclusive_confirmed = None;
            return Err("Linux 严格独占需要具体 PipeWire 设备或 ALSA 直连硬件；已暂停".into());
        }
        if unsupported && request.device_id == "auto" {
            self.set("pause", "yes")?;
            self.audio = AudioState {
                request,
                blocked: true,
                ..Default::default()
            };
            snapshot.exclusive_confirmed = None;
            return Err("Linux 独占请先选择具体设备，才能保证回退不切换到其他输出".into());
        }
        let retry = self.audio.blocked || snapshot.phase == PlaybackPhase::Error;
        let paused = self
            .audio
            .restore_pause
            .unwrap_or_else(|| self.get("pause") == "yes");
        let position = snapshot.position.map_or(0., |p| p.as_secs_f64());
        self.set("pause", "yes")?;
        let old_device = self.get("audio-device");
        let old_exclusive = self.get("audio-exclusive");
        let exclusive = request.mode != AudioMode::Shared && !unsupported;
        let result = self
            .set("audio-device", &request.device_id)
            .and_then(|_| self.set("audio-exclusive", if exclusive { "yes" } else { "no" }))
            .and_then(|_| self.set("volume", &request.volume_percent.to_string()));
        if let Err(error) = result {
            let _ = self.set("audio-device", &old_device);
            let _ = self.set("audio-exclusive", &old_exclusive);
            let _ = self.set("pause", if paused { "yes" } else { "no" });
            return Err(format!("输出设置未应用：{error}"));
        }
        self.audio = AudioState {
            request,
            fallback: unsupported,
            started: Some(Instant::now()),
            resume: Some((position, paused)),
            ..Default::default()
        };
        snapshot.exclusive_confirmed = None;
        snapshot.error.clear();
        snapshot.audio_status = "输出已请求，等待实际初始化".into();
        // Reload once at the saved position: all backend evidence belongs to this AO
        // initialization, including changes between two modes on the same device.
        if (matches!(
            snapshot.phase,
            PlaybackPhase::Playing | PlaybackPhase::Paused
        ) || retry)
            && let Some(source) = snapshot.source.as_ref()
        {
            let path = source_string(source)?;
            let paused = paused || retry;
            *resume = Some((path.clone(), position, paused));
            self.audio.restore_pause = Some(paused);
            self.command(&["loadfile".into(), path, "replace".into()])?;
            snapshot.phase = PlaybackPhase::Loading;
            return Ok(());
        }
        self.set("pause", if paused { "yes" } else { "no" })
    }
    pub(super) fn linux_audio_tick(
        &mut self,
        snapshot: &mut PlaybackSnapshot,
        resume: &mut Option<(String, f64, bool)>,
    ) -> Result<(), String> {
        let ao = self.get("current-ao");
        let source = self.node("audio-params");
        let output = self.node("audio-out-params");
        let has_audio = !self.get("audio-codec-name").is_empty();
        let initialized = !ao.is_empty() && output["samplerate"].as_u64().is_some();
        let request = &self.audio.request;
        let expected = request.mode != AudioMode::Shared && !self.audio.fallback;
        let hw = request.device_id.starts_with("alsa/hw:");
        let backend_mismatch = request
            .device_id
            .split_once('/')
            .is_some_and(|(backend, _)| backend != ao);
        let confirmed =
            if !initialized || self.audio.failed || self.audio.linux.lost || backend_mismatch {
                None
            } else if ao == "pipewire" && self.audio.linux.connected {
                // The pinned AO passes PW_STREAM_FLAG_EXCLUSIVE and waits for successful
                // connection. This confirms a PipeWire exclusive stream, not DAC ownership.
                Some(expected && self.get("audio-exclusive") == "yes")
            } else if ao == "alsa"
                && hw
                && self.audio.linux.hw_params
                && request.device_id.strip_prefix("alsa/") == Some(self.audio.linux.opened.as_str())
            {
                Some(true)
            } else if !expected && matches!(ao.as_str(), "pulse" | "alsa" | "pipewire") {
                Some(false)
            } else {
                None
            };
        snapshot.exclusive_confirmed = confirmed;
        snapshot.audio_source_rate = source["samplerate"].as_u64();
        snapshot.audio_output_rate = output["samplerate"].as_u64();
        snapshot.audio_fallback = self.audio.fallback;
        snapshot.audio_log = self.audio.log.iter().cloned().collect();
        snapshot.audio_info = format!(
            "音频 codec：{}\n源解码 PCM：{} Hz · {} · {}\nAPI 输出 PCM：{} Hz · {} · {}\n实际音频 API：{ao}\n请求设备：{}\nALSA 打开设备：{}\n物理 DAC 格式：未知（需要硬件 / 数字捕获验证）\n音量：{:.0}% · 倍速：{:.2}x\n实际 AF 链：{}\nPipeWire 独占流与 ALSA hw 硬件占用分别确认；均不证明 bit-perfect",
            self.get("audio-codec-name"),
            source["samplerate"],
            source["channels"],
            source["format"],
            output["samplerate"],
            output["channels"],
            output["format"],
            self.audio.request.device_id,
            self.audio.linux.opened,
            snapshot.volume,
            snapshot.speed,
            self.get("af")
        );
        if self.audio.blocked {
            return Ok(());
        }
        let disconnected = self.audio.request.device_id != "auto"
            && !snapshot
                .devices
                .iter()
                .any(|d| d.id == self.audio.request.device_id);
        if has_audio && (disconnected || self.audio.linux.lost) {
            self.set("pause", "yes")?;
            self.audio.blocked = true;
            snapshot.audio_status = "设备失联，已暂停；请重新选择设备".into();
            return Err(snapshot.audio_status.clone());
        }
        let timeout = has_audio
            && self
                .audio
                .started
                .is_some_and(|t| t.elapsed() > Duration::from_secs(5))
            && confirmed.is_none();
        if (self.audio.failed && snapshot.phase == PlaybackPhase::Error) || timeout {
            if self.audio.request.mode == AudioMode::PreferExclusive
                && !self.audio.fallback
                && self.audio.request.device_id.starts_with("pipewire/")
            {
                self.audio.fallback = true;
                self.audio.failed = false;
                self.audio.linux = Default::default();
                self.audio.started = Some(Instant::now());
                self.set("audio-exclusive", "no")?;
                if let Some(source) = snapshot.source.as_ref() {
                    let path = source_string(source)?;
                    let (position, paused) = self.audio.resume.unwrap_or((0., true));
                    *resume = Some((path.clone(), position, paused));
                    self.audio.restore_pause = Some(paused);
                    self.command(&["loadfile".into(), path, "replace".into()])?;
                    snapshot.phase = PlaybackPhase::Loading;
                    snapshot.error.clear();
                }
            } else {
                self.set("pause", "yes")?;
                self.audio.blocked = true;
                snapshot.audio_status =
                    "音频输出未确认，已暂停；ALSA 直连不会自动改走其他设备".into();
                return Err(snapshot.audio_status.clone());
            }
        }
        snapshot.audio_status = match confirmed {
            Some(true) if ao == "alsa" => "ALSA hw 直连硬件已打开（不等于位准确）".into(),
            Some(true) => "PipeWire 独占流已连接（不等于硬件独占 / 位准确）".into(),
            Some(false) if self.audio.fallback => "独占不可用，已回退到同一设备共享输出".into(),
            Some(false) => format!("{ao} 共享输出"),
            None if !has_audio => "等待音频 / 未初始化".into(),
            None => "实际输出状态未知 / 初始化中".into(),
        };
        if self.audio.request.preserve_rate
            && confirmed == Some(true)
            && source["samplerate"] != output["samplerate"]
        {
            self.set("pause", "yes")?;
            self.audio.blocked = true;
            snapshot.audio_status = "设备未接受源采样率；按严格源格式策略暂停".into();
            return Err(snapshot.audio_status.clone());
        }
        if confirmed.is_some()
            && let Some(paused) = self.audio.restore_pause.take()
        {
            self.set("pause", if paused { "yes" } else { "no" })?;
        }
        if confirmed.is_some()
            && matches!(
                snapshot.phase,
                PlaybackPhase::Playing | PlaybackPhase::Paused
            )
        {
            self.audio.resume = Some((
                snapshot.position.map_or(0., |p| p.as_secs_f64()),
                snapshot.phase == PlaybackPhase::Paused,
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn explicit_exclusive_backends_only() {
        assert!(exclusive_device("pipewire/alsa_output.usb-DAC"));
        assert!(exclusive_device("alsa/hw:CARD=DAC,DEV=0"));
        for id in [
            "auto",
            "pulse/speakers",
            "alsa/default",
            "alsa/plughw:0",
            "pipewire/",
        ] {
            assert!(!exclusive_device(id));
        }
    }
    #[test]
    fn pipewire_connection_and_disconnect_are_actual_events() {
        let mut audio = AudioState::default();
        audio.linux_log("ao/pipewire", "Headers version: 1.6.2", 60);
        audio.linux_log(
            "ao/pipewire",
            "Stream state changed: old_state=connecting state=paused error=(null)",
            60,
        );
        assert!(audio.linux.connected);
        audio.linux_log(
            "ao/pipewire",
            "Stream disconnected, trying to reload...",
            30,
        );
        assert!(audio.linux.lost);
        audio.linux_log("ao/pipewire", "Headers version: 1.6.2", 60);
        assert!(!audio.linux.connected);
    }
    #[test]
    fn alsa_open_request_is_not_negotiation() {
        let mut audio = AudioState::default();
        audio.linux_log("ao/alsa", "opening device 'hw:CARD=DAC,DEV=0'", 40);
        assert!(!audio.linux.hw_params);
        audio.linux_log(
            "ao/alsa",
            "Playback open error: Device or resource busy",
            20,
        );
        assert!(audio.failed);
        audio.linux_log("ao/alsa", "using ALSA version: 1.2.15", 40);
        audio.linux_log("ao/alsa", "Final HW params:", 40);
        assert!(audio.linux.hw_params);
        assert!(!audio.failed);
    }
}
