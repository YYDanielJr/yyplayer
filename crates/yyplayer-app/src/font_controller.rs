use super::AppController;
use player_core::PlaybackEngine;
use player_core::{
    PlaybackCommand,
    typography::{Fonts, LibraryColumns},
};
use player_ui::view_model::FontViewModel;
use std::{rc::Rc, time::Instant};
impl AppController {
    pub(super) fn poll_fonts(&mut self) {
        let Some(result) = self
            .font_task
            .as_ref()
            .and_then(|t| t.receiver.try_recv().ok())
        else {
            return;
        };
        self.font_task = None;
        match result {
            Ok(names) => {
                let mut all = vec![String::new()];
                all.extend(names.into_iter().filter(|f| Fonts::valid_family(f)));
                self.font_message =
                    format!("已读取 {} 个系统字体；新安装字体后可刷新", all.len() - 1);
                self.font_names = Rc::new(all);
            }
            Err(e) => self.font_message = e,
        }
        self.font_revision += 1;
        self.apply_subtitle_font();
    }
    fn resolved_font(&self, requested: &str) -> String {
        if requested.is_empty() || self.font_names.iter().any(|f| f == requested) {
            requested.into()
        } else {
            String::new()
        }
    }
    pub(super) fn apply_subtitle_font(&mut self) {
        if !self.engine.snapshot().ready {
            return;
        }
        self.send(PlaybackCommand::SetSubtitleFont {
            family: self.resolved_font(&self.settings.fonts.subtitles),
            override_ass: self.settings.fonts.override_ass,
        });
    }
    pub(super) fn font_control(&mut self, action: &str, value: &str) -> bool {
        match action {
            "font-refresh" => {
                if self.font_task.is_none() {
                    self.font_task = Some(super::font_service::Task::start());
                    self.font_message = "正在读取系统字体…".into();
                    self.font_revision += 1;
                }
                return true;
            }
            "font-ui" | "font-lyrics" | "font-subtitles" => {
                let Some(name) = value
                    .parse::<usize>()
                    .ok()
                    .and_then(|i| self.font_names.get(i))
                    .cloned()
                else {
                    return true;
                };
                match action {
                    "font-ui" => self.settings.fonts.ui = name,
                    "font-lyrics" => self.settings.fonts.lyrics = name,
                    _ => self.settings.fonts.subtitles = name,
                }
            }
            "font-ass" => self.settings.fonts.override_ass = value == "yes",
            "library-columns" => {
                let Some((song, artist)) = value.split_once(',') else {
                    return true;
                };
                let (Ok(song), Ok(artist)) = (song.parse(), artist.parse()) else {
                    return true;
                };
                let columns = LibraryColumns { song, artist };
                if columns.validate().is_err() {
                    return true;
                }
                self.settings.library_columns = columns;
            }
            _ => return false,
        }
        self.font_revision += 1;
        self.settings_revision += 1;
        self.settings_dirty = Some(Instant::now());
        if action == "font-subtitles" || action == "font-ass" {
            self.apply_subtitle_font();
        }
        true
    }
    pub(super) fn font_view(&self) -> FontViewModel {
        let f = &self.settings.fonts;
        let index = |name: &str| self.font_names.iter().position(|n| n == name).unwrap_or(0) as i32;
        let missing: Vec<_> = [&f.ui, &f.lyrics, &f.subtitles]
            .into_iter()
            .filter(|n| !n.is_empty() && !self.font_names.iter().any(|a| a == *n))
            .cloned()
            .collect();
        let ui = self.resolved_font(&f.ui);
        let lyrics = self.resolved_font(&f.lyrics);
        FontViewModel {
            revision: self.font_revision,
            names: self.font_names.clone(),
            ui: ui.clone(),
            lyrics: if lyrics.is_empty() { ui } else { lyrics },
            ui_index: index(&f.ui),
            lyrics_index: index(&f.lyrics),
            subtitle_index: index(&f.subtitles),
            override_ass: f.override_ass,
            busy: self.font_task.is_some(),
            message: if !missing.is_empty() && self.font_task.is_none() {
                format!(
                    "{}；未安装：{}，当前使用默认字体",
                    self.font_message,
                    missing.join("、")
                )
            } else {
                self.font_message.clone()
            },
        }
    }
}
