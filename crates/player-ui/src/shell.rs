use std::rc::Rc;

use slint::{ComponentHandle, ModelRc, VecModel};

use crate::{AppWindow, TrackRow, view_model::ShellViewModel};

#[derive(Clone, Debug)]
pub enum UiAction {
    Navigate(i32),
    Search(String),
    SelectPreview(i32),
    OpenFile,
    RequestPlayback,
    Previous,
    Next,
    SetVolume(f32),
    ToggleFavorite,
}

pub struct UiShell {
    window: AppWindow,
}

impl UiShell {
    pub fn new() -> Result<Self, slint::PlatformError> {
        Ok(Self {
            window: AppWindow::new()?,
        })
    }

    pub fn bind(&self, dispatch: Rc<dyn Fn(UiAction)>) {
        let handler = dispatch.clone();
        self.window
            .on_navigate(move |page| handler(UiAction::Navigate(page)));
        let handler = dispatch.clone();
        self.window
            .on_search_changed(move |text| handler(UiAction::Search(text.into())));
        let handler = dispatch.clone();
        self.window
            .on_select_preview(move |id| handler(UiAction::SelectPreview(id)));
        let handler = dispatch.clone();
        self.window
            .on_open_file(move || handler(UiAction::OpenFile));
        let handler = dispatch.clone();
        self.window
            .on_request_playback(move || handler(UiAction::RequestPlayback));
        let handler = dispatch.clone();
        self.window.on_previous(move || handler(UiAction::Previous));
        let handler = dispatch.clone();
        self.window.on_next(move || handler(UiAction::Next));
        let handler = dispatch.clone();
        self.window
            .on_volume_changed(move |volume| handler(UiAction::SetVolume(volume)));
        self.window
            .on_toggle_favorite(move || dispatch(UiAction::ToggleFavorite));
    }

    pub fn project(&self, state: &ShellViewModel) {
        project(&self.window, state);
    }

    /// The callback holds a weak window; UI callbacks never form a strong cycle.
    pub fn projector(&self) -> impl Fn(&ShellViewModel) + 'static {
        let weak = self.window.as_weak();
        move |state| {
            if let Some(window) = weak.upgrade() {
                project(&window, state);
            }
        }
    }

    pub fn run(&self) -> Result<(), slint::PlatformError> {
        self.window.run()
    }

    /// Window access for future native presenters and the developer preview example.
    /// Regular application callbacks should go through `bind` and `project`.
    pub fn component(&self) -> &AppWindow {
        &self.window
    }
}

fn rows(items: &[crate::view_model::MediaPreview]) -> ModelRc<TrackRow> {
    Rc::new(VecModel::from(
        items
            .iter()
            .map(|item| TrackRow {
                item_id: item.id,
                title: item.title.as_str().into(),
                artist: item.artist.as_str().into(),
                collection: item.collection.as_str().into(),
                duration: item.duration.as_str().into(),
                cover: item.cover,
            })
            .collect::<Vec<_>>(),
    ))
    .into()
}

fn project(window: &AppWindow, state: &ShellViewModel) {
    window.set_page(state.page);
    window.set_selected_id(state.selected_id);
    window.set_selected_title(state.selected_title.as_str().into());
    window.set_selected_artist(state.selected_artist.as_str().into());
    window.set_selected_duration(state.selected_duration.as_str().into());
    window.set_selected_cover(state.selected_cover);
    window.set_volume(state.volume_percent);
    window.set_favorite(state.favorite);
    window.set_status_message(state.status.as_str().into());
    window.set_tracks(rows(&state.tracks));
    window.set_queue(rows(&state.queue));
}
