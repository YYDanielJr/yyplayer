use player_core::{AudioRequest, PlaybackCommand, PlaybackEngine};
use player_mpv::MpvEngine;
use player_platform::PlatformInfo;
use player_ui::{UiAction, view_model::ShellViewModel};

use crate::demo;

/// A single owner for application state. No decoder, timer or fake playback loop.
pub struct AppController {
    engine: MpvEngine,
    platform: PlatformInfo,
    page: i32,
    selected_id: i32,
    search: String,
    favorite: bool,
    audio_request: AudioRequest,
    status: String,
}

impl AppController {
    pub fn new(engine: MpvEngine, platform: PlatformInfo) -> Self {
        Self {
            engine,
            platform,
            page: 0,
            selected_id: 0,
            search: String::new(),
            favorite: false,
            audio_request: AudioRequest::default(),
            status: String::new(),
        }
    }

    pub fn dispatch(&mut self, action: UiAction) {
        self.status.clear();
        match action {
            UiAction::Navigate(page) => self.page = page.clamp(0, 3),
            UiAction::Search(query) => self.search = query,
            UiAction::SelectPreview(id) => {
                if demo::tracks().iter().any(|item| item.id == id) {
                    self.selected_id = id;
                    self.favorite = false;
                }
            }
            UiAction::SetVolume(volume) => {
                if volume.is_finite() {
                    self.audio_request.volume_percent = volume.clamp(0.0, 100.0);
                }
            }
            UiAction::ToggleFavorite => self.favorite = !self.favorite,
            UiAction::OpenFile => self.status = "文件打开将在后续版本开放".into(),
            UiAction::RequestPlayback => {
                if let Err(error) = self.engine.submit(PlaybackCommand::Resume) {
                    self.status = error.to_string();
                }
            }
            UiAction::Previous => self.select_adjacent(-1),
            UiAction::Next => self.select_adjacent(1),
        }
    }

    fn select_adjacent(&mut self, offset: i32) {
        let count = demo::tracks().len() as i32;
        self.selected_id = (self.selected_id + offset).rem_euclid(count);
        self.favorite = false;
    }

    pub fn view_model(&self) -> ShellViewModel {
        let queue = demo::tracks();
        let selected = &queue[self.selected_id as usize];
        let query = self.search.trim().to_lowercase();
        let tracks = queue
            .iter()
            .filter(|track| {
                query.is_empty()
                    || format!("{} {} {}", track.title, track.artist, track.collection)
                        .to_lowercase()
                        .contains(&query)
            })
            .cloned()
            .collect();

        ShellViewModel {
            page: self.page,
            selected_id: self.selected_id,
            selected_title: selected.title.clone(),
            selected_artist: selected.artist.clone(),
            selected_duration: selected.duration.clone(),
            selected_cover: selected.cover,
            volume_percent: self.audio_request.volume_percent,
            favorite: self.favorite,
            status: if self.status.is_empty() {
                format!("{} · 界面预览", self.platform.name)
            } else {
                self.status.clone()
            },
            tracks,
            queue,
        }
    }

    pub fn engine_mut(&mut self) -> &mut MpvEngine {
        &mut self.engine
    }
}
