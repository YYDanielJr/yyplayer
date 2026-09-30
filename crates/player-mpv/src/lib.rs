//! libmpv adapter boundary. This scaffold deliberately does not load a DLL.

use player_core::{
    EngineCapabilities, EngineError, PlaybackCommand, PlaybackEngine, PlaybackSnapshot,
};

#[derive(Default)]
pub struct MpvEngine {
    snapshot: PlaybackSnapshot,
}

impl PlaybackEngine for MpvEngine {
    fn capabilities(&self) -> EngineCapabilities {
        EngineCapabilities::default()
    }

    fn snapshot(&self) -> &PlaybackSnapshot {
        &self.snapshot
    }

    fn submit(&mut self, command: PlaybackCommand) -> Result<(), EngineError> {
        match command {
            PlaybackCommand::Shutdown => Ok(()),
            _ => Err(EngineError::NotConnected),
        }
    }
}
