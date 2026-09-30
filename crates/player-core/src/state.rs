use std::time::Duration;

use crate::{MediaSource, QueueItemId};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PlaybackPhase {
    #[default]
    Idle,
    Loading,
    Playing,
    Paused,
    Ended,
    Error,
}

/// Confirmed engine state, kept separate from the UI's demonstration data.
#[derive(Clone, Debug, Default)]
pub struct PlaybackSnapshot {
    pub revision: u64,
    pub generation: u64,
    pub phase: PlaybackPhase,
    pub source: Option<MediaSource>,
    pub current_item: Option<QueueItemId>,
    pub position: Option<Duration>,
    pub duration: Option<Duration>,
    pub audio_output: Option<String>,
    pub exclusive_confirmed: Option<bool>,
    pub ready: bool,
    pub title: String,
    pub video: bool,
    pub speed: f64,
    pub volume: f32,
    pub muted: bool,
    pub devices: Vec<AudioDevice>,
    pub tracks: Vec<MediaTrack>,
    pub chapters: Vec<Chapter>,
    pub info: String,
    pub hwdec: String,
    pub error: String,
    pub runtime: String,
}

#[derive(Clone, Debug, Default)]
pub struct AudioDevice {
    pub id: String,
    pub name: String,
}
#[derive(Clone, Debug, Default)]
pub struct MediaTrack {
    pub id: i64,
    pub kind: String,
    pub label: String,
    pub selected: bool,
}
#[derive(Clone, Debug, Default)]
pub struct Chapter {
    pub title: String,
    pub seconds: f64,
}
