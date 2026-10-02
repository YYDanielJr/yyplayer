//! Latest video intent waits for a render lease; audio and cancellation never do.
use player_core::PlaybackCommand;

#[derive(Default)]
pub(super) struct VideoGate {
    load: Option<PlaybackCommand>,
    enable: bool,
}
impl VideoGate {
    pub fn accept(&mut self, command: PlaybackCommand, ready: bool) -> Option<PlaybackCommand> {
        match &command {
            PlaybackCommand::Load { video, .. } => {
                self.load = None;
                self.enable = false;
                if *video && !ready {
                    self.load = Some(command);
                    return None;
                }
            }
            PlaybackCommand::SetVideoOutput(enabled) => {
                self.enable = false;
                if !enabled {
                    self.load = None;
                } else if !ready {
                    self.enable = true;
                    return None;
                }
            }
            PlaybackCommand::Stop => {
                self.load = None;
                self.enable = false;
            }
            _ => {}
        }
        Some(command)
    }
    pub fn ready(&mut self) -> Option<PlaybackCommand> {
        if let Some(load) = self.load.take() {
            self.enable = false;
            Some(load)
        } else if std::mem::take(&mut self.enable) {
            Some(PlaybackCommand::SetVideoOutput(true))
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use player_core::MediaSource;
    fn load(name: &str, video: bool) -> PlaybackCommand {
        PlaybackCommand::Load {
            source: MediaSource::Local(name.into()),
            decode: Default::default(),
            resume: None,
            video,
        }
    }
    #[test]
    fn audio_supersedes_video_waiting_for_renderer() {
        let mut gate = VideoGate::default();
        assert!(gate.accept(load("old.mp4", true), false).is_none());
        assert!(matches!(
            gate.accept(load("new.flac", false), false),
            Some(PlaybackCommand::Load { video: false, .. })
        ));
        assert!(gate.ready().is_none());
    }
    #[test]
    fn cancel_and_stop_prevent_late_video_start() {
        for cancel in [
            PlaybackCommand::Stop,
            PlaybackCommand::SetVideoOutput(false),
        ] {
            let mut gate = VideoGate::default();
            gate.accept(load("old.mp4", true), false);
            gate.accept(PlaybackCommand::SetVideoOutput(true), false);
            assert!(gate.accept(cancel, false).is_some());
            assert!(gate.ready().is_none());
        }
    }
    #[test]
    fn latest_load_and_reenable_are_consumed_once() {
        let mut gate = VideoGate::default();
        gate.accept(load("old.mp4", true), false);
        gate.accept(load("new.mp4", true), false);
        assert!(
            matches!(gate.ready(), Some(PlaybackCommand::Load { source: MediaSource::Local(p), .. }) if p.to_str()==Some("new.mp4"))
        );
        assert!(gate.ready().is_none());
        assert!(
            gate.accept(PlaybackCommand::SetVideoOutput(true), false)
                .is_none()
        );
        assert!(matches!(
            gate.ready(),
            Some(PlaybackCommand::SetVideoOutput(true))
        ));
        assert!(gate.ready().is_none());
    }
}
