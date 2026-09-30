//! UI- and platform-independent playback contracts. No playback implementation here.

pub mod engine;
pub mod state;
pub mod types;

pub use engine::{EngineCapabilities, EngineError, PlaybackEngine};
pub use state::{PlaybackPhase, PlaybackSnapshot};
pub use types::{AudioMode, AudioRequest, MediaSource, PlaybackCommand, QueueItem, QueueItemId};
