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
}

impl Default for AudioRequest {
    fn default() -> Self {
        Self {
            device_id: "auto".into(),
            mode: AudioMode::Shared,
            volume_percent: 70.0,
        }
    }
}

#[derive(Clone, Debug)]
pub enum PlaybackCommand {
    Open(MediaSource),
    ReplaceQueue(Vec<QueueItem>),
    PlayItem(QueueItemId),
    Pause,
    Resume,
    Stop,
    Seek(Duration),
    SetVolume(f32),
    ApplyAudioRequest(AudioRequest),
    Shutdown,
}
