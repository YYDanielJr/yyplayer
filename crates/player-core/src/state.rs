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
}
