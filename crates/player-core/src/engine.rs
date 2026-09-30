use std::fmt;

use crate::{PlaybackCommand, PlaybackSnapshot};

#[derive(Clone, Copy, Debug, Default)]
pub struct EngineCapabilities {
    pub playback: bool,
    pub video: bool,
    pub audio_device_selection: bool,
    pub exclusive_audio: bool,
    pub equalizer: bool,
}

#[derive(Clone, Debug)]
pub enum EngineError {
    NotConnected,
    Unsupported(&'static str),
}

impl fmt::Display for EngineError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotConnected => formatter.write_str("播放功能尚未开放"),
            Self::Unsupported(feature) => write!(formatter, "尚不支持：{feature}"),
        }
    }
}

impl std::error::Error for EngineError {}

/// The future real implementation will receive commands on its engine thread.
/// The scaffold implementation is immediate and performs no IO or decoding.
pub trait PlaybackEngine {
    fn capabilities(&self) -> EngineCapabilities;
    fn snapshot(&self) -> &PlaybackSnapshot;
    fn submit(&mut self, command: PlaybackCommand) -> Result<(), EngineError>;
}
