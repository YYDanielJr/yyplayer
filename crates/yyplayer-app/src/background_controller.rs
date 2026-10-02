use super::{AppController, DialogRequest};
use player_core::appearance::Background;
use std::{path::PathBuf, time::Instant};

impl AppController {
    fn background(&self, kind: usize) -> &Background {
        if kind == 0 {
            &self.settings.appearance.library_background
        } else {
            &self.settings.appearance.lyrics_background
        }
    }
    fn background_mut(&mut self, kind: usize) -> &mut Background {
        if kind == 0 {
            &mut self.settings.appearance.library_background
        } else {
            &mut self.settings.appearance.lyrics_background
        }
    }
    pub(super) fn request_background(&mut self, kind: usize) {
        let background = self.background(kind);
        if background.style != 5 {
            return;
        }
        let Some(path) = background.file.clone() else {
            return;
        };
        let blur = background.blur;
        if let Some(service) = &mut self.backgrounds {
            service.request(kind, path, blur);
        }
    }
    fn background_changed(&mut self) {
        self.settings_revision += 1;
        self.settings_dirty = Some(Instant::now());
    }
    pub(super) fn set_background_file(&mut self, kind: usize, path: PathBuf) {
        if kind > 1 {
            return;
        }
        let background = self.background_mut(kind);
        background.file = Some(path);
        background.style = 5;
        self.request_background(kind);
        self.background_changed();
    }
    pub(super) fn background_control(&mut self, action: &str, value: &str) -> bool {
        let Some(rest) = action.strip_prefix("background-") else {
            return false;
        };
        let Some((target, field)) = rest.split_once('-') else {
            return false;
        };
        let kind = match target {
            "library" => 0,
            "lyrics" => 1,
            _ => return false,
        };
        match field {
            "pick" => self.dialog(DialogRequest::Background(kind)),
            "style" => {
                let Ok(style) = value.parse::<u8>() else {
                    return true;
                };
                if style > 5 {
                    return true;
                }
                if style == 5 && self.background(kind).file.is_none() {
                    self.dialog(DialogRequest::Background(kind));
                    return true;
                }
                if self.background(kind).style != style {
                    self.background_mut(kind).style = style;
                    if style == 5 {
                        self.request_background(kind);
                    }
                    self.background_changed();
                }
            }
            "opacity" => {
                let Ok(opacity) = value.parse::<u8>() else {
                    return true;
                };
                if opacity <= 100 && self.background(kind).opacity != opacity {
                    self.background_mut(kind).opacity = opacity;
                    self.background_changed();
                }
            }
            "blur" => {
                let Ok(blur) = value.parse::<u8>() else {
                    return true;
                };
                if blur <= 32 && self.background(kind).blur != blur {
                    self.background_mut(kind).blur = blur;
                    self.request_background(kind);
                    self.background_changed();
                }
            }
            _ => return false,
        }
        true
    }
}
