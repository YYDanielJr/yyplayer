use crate::settings::DecodeOptions;
use std::path::PathBuf;
use std::time::Duration;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MediaSource {
    Local(PathBuf),
    Url(String),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct QueueItemId(pub u64);

#[derive(Clone, Debug)]
pub struct QueueItem {
    pub id: QueueItemId,
    pub source: MediaSource,
    pub title: String,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AudioMode {
    #[default]
    Shared,
    PreferExclusive,
    StrictExclusive,
}

#[derive(Clone, Debug)]
pub struct AudioRequest {
    pub device_id: String,
    pub mode: AudioMode,
    pub volume_percent: f32,
    pub preserve_rate: bool,
}

impl Default for AudioRequest {
    fn default() -> Self {
        Self {
            device_id: "auto".into(),
            mode: AudioMode::Shared,
            volume_percent: 70.0,
            preserve_rate: false,
        }
    }
}

#[derive(Clone, Debug)]
pub enum PlaybackCommand {
    Open(MediaSource),
    Load {
        source: MediaSource,
        decode: DecodeOptions,
        resume: Option<(f64, bool)>,
        /// Video output requires the presenter's render lease; audio does not.
        video: bool,
    },
    SetVideoOutput(bool),
    SeekRelative(f64),
    SetSpeed(f64),
    SetMute(bool),
    SetDevice(String),
    SetTrack {
        kind: String,
        id: String,
    },
    AddSubtitle(PathBuf),
    SetSubtitleDelay(f64),
    SetSubtitleFont {
        family: String,
        override_ass: bool,
    },
    SetAudioDelay(f64),
    SetLoop(bool),
    FrameStep(bool),
    Screenshot(PathBuf),
    SetAspect(String),
    SetAbLoop(Option<(f64, f64)>),
    ReplaceQueue(Vec<QueueItem>),
    PlayItem(QueueItemId),
    Pause,
    Resume,
    Stop,
    Seek(Duration),
    SetVolume(f32),
    ApplyAudioRequest(AudioRequest),
    ApplyEq(crate::audio::EqPreset),
    Shutdown,
}
