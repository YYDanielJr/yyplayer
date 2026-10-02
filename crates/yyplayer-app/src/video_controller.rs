use super::*;
use std::rc::Rc;
impl AppController {
    pub(super) fn video_request(&mut self, request: library_service::Request) {
        if let Some(service) = &self.video_service
            && let Err(e) = service.commands.try_send(request)
        {
            match e {
                std::sync::mpsc::TrySendError::Full(request) => {
                    if matches!(request, library_service::Request::Scan(..)) {
                        self.video_pending = Some(request);
                    } else {
                        self.status = "视频库服务忙，请稍后重试".into();
                    }
                }
                std::sync::mpsc::TrySendError::Disconnected(_) => {
                    self.status = "视频库服务已停止".into()
                }
            }
        }
    }
    pub(super) fn scan_video(&mut self) {
        self.video_scan += 1;
        self.video_busy = true;
        if let Some(s) = &self.video_service {
            s.generation
                .store(self.video_scan, std::sync::atomic::Ordering::Relaxed);
        }
        self.video_request(library_service::Request::Scan(
            self.video_scan,
            self.settings.video_library.clone(),
        ));
    }
    pub(super) fn poll_video(&mut self) {
        if let Some(request) = self.video_pending.take() {
            self.video_request(request);
        }
        let replies: Vec<_> = self
            .video_service
            .as_ref()
            .map(|s| s.replies.try_iter().collect())
            .unwrap_or_default();
        for reply in replies {
            match reply {
                library_service::Reply::Paths(folder, paths) => {
                    self.import_video_paths(folder, paths);
                }
                library_service::Reply::Indexed(id, config, songs, message) => {
                    if config == self.settings.video_library
                        && (id == self.video_scan || (id == 0 && self.video_songs.is_empty()))
                    {
                        self.video_songs = songs;
                        if id == self.video_scan {
                            self.video_busy = false;
                        }
                        self.video_message = message;
                        self.refresh_video();
                    }
                }
                library_service::Reply::Error(e) => {
                    self.video_message = e;
                }
            }
        }
    }
    pub(super) fn import_video_paths(&mut self, folder: bool, paths: Vec<PathBuf>) {
        let mut config = self.settings.video_library.clone();
        for path in paths {
            if folder {
                if !config.roots.contains(&path) {
                    config.roots.push(path);
                }
            } else {
                config.excluded.retain(|p| p != &path);
                if !config.files.contains(&path) {
                    config.files.push(path);
                }
            }
        }
        if let Err(e) = config.validate() {
            self.status = e;
            return;
        }
        self.settings.video_library = config;
        self.settings_dirty = Some(Instant::now());
        self.refresh_video();
        self.scan_video();
    }
    pub(super) fn refresh_video(&mut self) {
        let query = self.video_search.trim().to_lowercase();
        let folder = self
            .video_folder
            .checked_sub(1)
            .and_then(|i| self.settings.video_library.roots.get(i as usize));
        let loose = self.video_folder == self.settings.video_library.roots.len() as i32 + 1;
        let loose_files: std::collections::BTreeSet<_> =
            self.settings.video_library.files.iter().collect();
        let excluded: std::collections::BTreeSet<_> =
            self.settings.video_library.excluded.iter().collect();
        self.video_rows = Rc::new(
            self.video_songs
                .iter()
                .enumerate()
                .filter(|(_, s)| {
                    (loose_files.contains(&s.path)
                        || self
                            .settings
                            .video_library
                            .roots
                            .iter()
                            .any(|p| s.path.starts_with(p)))
                        && !excluded.contains(&s.path)
                        && (!loose || loose_files.contains(&s.path))
                        && folder.is_none_or(|p| s.path.starts_with(p))
                        && (query.is_empty()
                            || format!("{} {} {} {}", s.title, s.artist, s.album, s.path.display())
                                .to_lowercase()
                                .contains(&query))
                })
                .map(|(i, s)| MediaPreview {
                    id: i as i32,
                    title: s.title.clone(),
                    artist: s.artist.clone(),
                    collection: s.album.clone(),
                    duration: if s.seconds > 0 {
                        format_time(Some(Duration::from_secs(s.seconds)))
                    } else {
                        "".into()
                    },
                    cover: -1,
                    artwork: Default::default(),
                })
                .collect(),
        );
        self.video_revision += 1;
    }
    pub(super) fn video_control(&mut self, action: &str, value: &str) -> bool {
        let index = value.parse::<usize>().ok();
        match action {
            "video-folder-add" => self.video_request(library_service::Request::Folder),
            "video-files-add" => self.video_request(library_service::Request::Files),
            "video-rescan" => self.scan_video(),
            "video-cancel" => {
                self.video_scan += 1;
                if let Some(s) = &self.video_service {
                    s.generation
                        .store(self.video_scan, std::sync::atomic::Ordering::Relaxed);
                }
                self.video_pending = None;
                self.video_busy = false;
                self.video_message = "扫描已取消；已完成的索引保留".into();
            }
            "video-folder" => {
                self.video_folder = value.parse().unwrap_or(0);
                self.refresh_video();
            }
            "video-folder-remove" => {
                if self.video_folder > 0
                    && self.video_folder as usize <= self.settings.video_library.roots.len()
                {
                    self.settings
                        .video_library
                        .roots
                        .remove(self.video_folder as usize - 1);
                    self.video_folder = 0;
                    self.settings_dirty = Some(Instant::now());
                    self.refresh_video();
                    self.scan_video();
                }
            }
            "video-remove" => {
                if let Some(song) = index.and_then(|i| self.video_songs.get(i)).cloned() {
                    self.settings
                        .video_library
                        .files
                        .retain(|p| p != &song.path);
                    if !self.settings.video_library.excluded.contains(&song.path) {
                        if self.settings.video_library.excluded.len() >= 10000 {
                            self.status = "单个视频排除已达到 10000 个，请按目录管理".into();
                            return true;
                        }
                        self.settings.video_library.excluded.push(song.path.clone());
                    }
                    self.settings_dirty = Some(Instant::now());
                    self.video_songs.retain(|s| s.path != song.path);
                    self.refresh_video();
                    self.scan_video();
                }
            }
            "video-play" => {
                if let Some(row) = self
                    .video_rows
                    .iter()
                    .position(|r| Some(r.id as usize) == index)
                {
                    self.queue = self
                        .video_rows
                        .iter()
                        .filter_map(|r| self.video_songs.get(r.id as usize))
                        .map(|s| MediaSource::Local(s.path.clone()))
                        .collect();
                    self.queue_version += 1;
                    self.load_index(row as i32);
                }
            }
            _ => return false,
        }
        true
    }
    pub(super) fn video_view(&self) -> player_ui::view_model::LibraryViewModel {
        let mut folders = vec!["全部视频".into()];
        folders.extend(
            self.settings
                .video_library
                .roots
                .iter()
                .map(|p| library_service::display_path(p)),
        );
        folders.push("单独添加的视频".into());
        let current = self.queue.get(self.selected_id as usize);
        let selected = current
            .and_then(|src| {
                if let MediaSource::Local(p) = src {
                    self.video_songs.iter().position(|s| &s.path == p)
                } else {
                    None
                }
            })
            .map(|i| i as i32)
            .unwrap_or(-1);
        player_ui::view_model::LibraryViewModel {
            rows: self.video_rows.clone(),
            revision: self.video_revision,
            folders,
            folder: self.video_folder,
            busy: self.video_busy,
            message: self.video_message.clone(),
            selected,
        }
    }
}
