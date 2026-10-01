use std::rc::Rc;

use slint::{ComponentHandle, Model, ModelRc, VecModel};

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
    Control(String, String),
    ShortcutEdited(i32, String),
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

    pub fn bind_controls(&self, dispatch: Rc<dyn Fn(UiAction)>) {
        let control = dispatch.clone();
        self.window.on_control(move |action, value| {
            control(UiAction::Control(action.into(), value.into()))
        });
        self.window.on_shortcut_edited(move |index, value| {
            dispatch(UiAction::ShortcutEdited(index, value.into()))
        });
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

fn rows(
    current: ModelRc<TrackRow>,
    items: &[crate::view_model::MediaPreview],
) -> ModelRc<TrackRow> {
    let next = items
        .iter()
        .map(|item| TrackRow {
            item_id: item.id,
            title: item.title.as_str().into(),
            artist: item.artist.as_str().into(),
            collection: item.collection.as_str().into(),
            duration: item.duration.as_str().into(),
            cover: item.cover,
        })
        .collect::<Vec<_>>();
    if let Some(model) = current.as_any().downcast_ref::<VecModel<TrackRow>>()
        && model.row_count() == next.len()
    {
        for (index, row) in next.into_iter().enumerate() {
            if model.row_data(index).as_ref() != Some(&row) {
                model.set_row_data(index, row);
            }
        }
        current
    } else {
        Rc::new(VecModel::from(next)).into()
    }
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
    window.set_status_revision(state.status_revision as i32);
    window.set_tracks(rows(window.get_tracks(), &state.tracks));
    window.set_queue(rows(window.get_queue(), &state.queue));
    window.set_recent(rows(window.get_recent(), &state.recent));
    window.set_playing(state.playing);
    window.set_has_media(state.has_media);
    window.set_has_video(state.has_video);
    window.set_position_text(state.position_text.as_str().into());
    window.set_progress(state.progress);
    window.set_seekable(state.seekable);
    window.set_muted(state.muted);
    if !same_strings(&window.get_speed_choices(), &state.speed_choices) {
        window.set_speed_choices(strings(&state.speed_choices));
    }
    window.set_speed_index(state.speed_index);
    window.set_media_info(state.info.as_str().into());
    if !same_strings(&window.get_device_names(), &state.device_names) {
        window.set_device_names(strings(&state.device_names));
    }
    window.set_device_index(state.device_index);
    if !same_strings(&window.get_audio_tracks(), &state.audio_tracks) {
        window.set_audio_tracks(strings(&state.audio_tracks));
    }
    window.set_audio_index(state.audio_index);
    if !same_strings(&window.get_subtitle_tracks(), &state.subtitle_tracks) {
        window.set_subtitle_tracks(strings(&state.subtitle_tracks));
    }
    window.set_subtitle_index(state.subtitle_index);
    if !same_strings(&window.get_chapters(), &state.chapters) {
        window.set_chapters(strings(&state.chapters));
    }
    window.set_panel_open(state.panel_open);
    window.set_fullscreen_mode(state.fullscreen);
    window.set_window_mode(state.window_mode);
    if window.get_settings_revision() != state.settings_revision as i32 {
        window.set_settings_revision(state.settings_revision as i32);
        window.set_decode_mode(state.decode_mode);
        window.set_decode_threads(state.decode_threads);
        window.set_deinterlace(state.deinterlace);
        window.set_deband(state.deband);
        window.set_decode_scope(state.decode_scope);
        window.set_shortcut_rows(
            Rc::new(VecModel::from(
                state
                    .short_bindings
                    .iter()
                    .map(|(action, binding)| crate::ShortcutRow {
                        action: action.as_str().into(),
                        binding: binding.as_str().into(),
                    })
                    .collect::<Vec<_>>(),
            ))
            .into(),
        );
        window.set_seek_step(state.seek_step);
        window.set_volume_step(state.volume_step);
        window.set_hold_ms(state.hold_ms);
        window.set_hold_speed(state.hold_speed.as_str().into());
    }
    window.set_decode_origin(state.decode_origin.as_str().into());
}

fn strings(values: &[String]) -> ModelRc<slint::SharedString> {
    Rc::new(VecModel::from(
        values
            .iter()
            .map(|value| value.as_str().into())
            .collect::<Vec<_>>(),
    ))
    .into()
}
fn same_strings(model: &ModelRc<slint::SharedString>, values: &[String]) -> bool {
    model.row_count() == values.len()
        && values.iter().enumerate().all(|(index, value)| {
            model
                .row_data(index)
                .is_some_and(|row| row.as_str() == value)
        })
}
