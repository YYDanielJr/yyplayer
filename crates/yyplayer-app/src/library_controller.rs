use super::*;
use player_core::appearance::{Design, Scheme, ink, parse_color};
impl AppController {
    pub(super) fn library_request(&mut self, request: library_service::Request) {
        if let Some(service) = &self.library_service
            && let Err(e) = service.commands.try_send(request)
        {
            match e {
                std::sync::mpsc::TrySendError::Full(request) => {
                    if matches!(request, library_service::Request::Scan(..)) {
                        self.library_pending = Some(request);
                    } else {
                        self.status = "音乐库服务忙，请稍后重试".into();
                    }
                }
                std::sync::mpsc::TrySendError::Disconnected(_) => {
                    self.status = "音乐库服务已停止".into()
                }
            }
        }
    }
    pub(super) fn scan_library(&mut self) {
        self.library_scan += 1;
        self.library_busy = true;
        if let Some(s) = &self.library_service {
            s.generation
                .store(self.library_scan, std::sync::atomic::Ordering::Relaxed);
        }
        self.library_request(library_service::Request::Scan(
            self.library_scan,
            self.settings.library.clone(),
        ));
    }
    fn restore_music_collections(&mut self, value: services::music_store::Collections) {
        if !self.music_ready {
            self.settings.library = value.library;
            self.settings.playlists = value.playlists;
        } else if self.settings.library != value.library
            || self.settings.playlists != value.playlists
        {
            // A retry must retain edits already accepted by this session. The
            // persistence worker will save them to the reopened connection.
            self.settings_dirty = Some(Instant::now());
        }
        self.music_ready = true;
        self.music_retry = false;
    }
    pub(super) fn poll_library(&mut self) {
        let initialized: Vec<_> = self
            .persistence
            .as_ref()
            .map(|p| p.music.try_iter().collect())
            .unwrap_or_default();
        for result in initialized {
            match result {
                Ok(collections) => {
                    if let Some(old) = self.library_service.take() {
                        old.generation
                            .store(u64::MAX, std::sync::atomic::Ordering::Relaxed);
                    }
                    self.restore_music_collections(collections);
                    self.library_busy = false;
                    self.library_service = Some(library_service::Service::start(
                        services::settings_path().with_extension("library.json"),
                        self.settings.library.clone(),
                    ));
                    if self.status == self.library_message {
                        self.status.clear();
                    }
                    self.refresh_library();
                    let before_import = self.library_scan;
                    let pending = std::mem::take(&mut self.pending_music_import);
                    if !pending.roots.is_empty() {
                        self.import_library_paths(true, pending.roots);
                    }
                    if !pending.files.is_empty() {
                        self.import_library_paths(false, pending.files);
                    }
                    if self.library_scan == before_import
                        && (!self.settings.library.roots.is_empty()
                            || !self.settings.library.files.is_empty())
                    {
                        self.scan_library();
                    }
                }
                Err(error) => {
                    self.music_retry = true;
                    self.library_busy = false;
                    self.library_message = error.clone();
                    self.status = error;
                    self.notice_revision += 1;
                }
            }
        }
        if let Some(request) = self.library_pending.take() {
            self.library_request(request);
        }
        let replies: Vec<_> = self
            .library_service
            .as_ref()
            .map(|s| s.replies.try_iter().collect())
            .unwrap_or_default();
        for reply in replies {
            match reply {
                library_service::Reply::Paths(folder, paths) => {
                    self.import_library_paths(folder, paths);
                }
                library_service::Reply::Indexed(id, config, songs, message) => {
                    if config == self.settings.library
                        && (id == self.library_scan || (id == 0 && self.library_songs.is_empty()))
                    {
                        if let Some(t) = &mut self.thumbnails {
                            t.invalidate();
                        }
                        self.library_songs = songs;
                        for song in &mut self.library_songs {
                            if let Some(metadata) = self.settings.online.downloads.get(&song.path) {
                                song.title = metadata.title.clone();
                                song.artist = metadata.artist.clone();
                                song.album = metadata.album.clone();
                            }
                        }
                        if id == self.library_scan {
                            self.library_busy = false;
                        }
                        self.library_message = message;
                        self.refresh_library();
                    }
                }
                library_service::Reply::StorageError(e) => {
                    // The last valid collections remain authoritative even if
                    // an index write fails. Do not merge an older DB snapshot
                    // over a pending playlist edit during the next save/exit.
                    self.music_retry = true;
                    self.library_busy = false;
                    self.library_message = e.clone();
                    self.status = e;
                    self.notice_revision += 1;
                }
                library_service::Reply::Error(e) => {
                    self.library_busy = false;
                    self.library_message = e;
                }
            }
        }
    }
    pub(super) fn import_library_paths(&mut self, folder: bool, paths: Vec<PathBuf>) {
        if !self.music_ready {
            let mut pending = self.pending_music_import.clone();
            let target = if folder {
                &mut pending.roots
            } else {
                &mut pending.files
            };
            for path in paths {
                if !target.contains(&path) {
                    target.push(path);
                }
            }
            if let Err(error) = pending.validate() {
                self.status = error;
                return;
            }
            self.pending_music_import = pending;
            self.status = "音乐导入已排队，将在数据库就绪后处理；初始化失败时可恢复后重扫".into();
            return;
        }
        let mut config = self.settings.library.clone();
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
        self.settings.library = config;
        self.settings_dirty = Some(Instant::now());
        self.refresh_library();
        self.scan_library();
    }
    pub(super) fn refresh_library(&mut self) {
        let query = self.search.trim().to_lowercase();
        let folder = self
            .library_folder
            .checked_sub(1)
            .and_then(|i| self.settings.library.roots.get(i as usize));
        let loose = self.library_folder == self.settings.library.roots.len() as i32 + 1;
        let loose_files: std::collections::BTreeSet<_> =
            self.settings.library.files.iter().collect();
        let excluded: std::collections::BTreeSet<_> =
            self.settings.library.excluded.iter().collect();
        let indices = self
            .library_songs
            .iter()
            .enumerate()
            .filter(|(_, s)| {
                (loose_files.contains(&s.path)
                    || self
                        .settings
                        .library
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
            .map(|(i, _)| i)
            .collect();
        self.build_collection(indices);
    }
    pub(super) fn library_control(&mut self, action: &str, value: &str) -> bool {
        if action == "library-rescan" && (self.music_retry || !self.music_ready) {
            self.library_pending = None;
            if let Some(old) = self.library_service.take() {
                old.generation
                    .store(u64::MAX, std::sync::atomic::Ordering::Relaxed);
            }
            if let Some(p) = &self.persistence {
                let _ = p.retry_music.try_send(());
            }
            self.library_busy = true;
            self.library_message = "正在重试音乐数据库…".into();
            self.status = self.library_message.clone();
            return true;
        }
        if action.starts_with("library-") && !self.music_ready {
            self.status = "音乐数据库尚未就绪，原库保留；请稍后重试或点击重扫".into();
            return true;
        }
        let index = value.parse::<usize>().ok();
        match action {
            "library-folder-add" => self.library_request(library_service::Request::Folder),
            "library-files-add" => self.library_request(library_service::Request::Files),
            "library-rescan" => self.scan_library(),
            "library-cancel" => {
                self.library_scan += 1;
                if let Some(s) = &self.library_service {
                    s.generation
                        .store(self.library_scan, std::sync::atomic::Ordering::Relaxed);
                }
                self.library_pending = None;
                self.library_busy = false;
                self.library_message = "扫描已取消；已完成的索引保留".into();
            }
            "library-folder" => {
                self.library_folder = value.parse().unwrap_or(0);
                self.refresh_library();
            }
            "library-folder-remove" => {
                if self.library_folder > 0
                    && self.library_folder as usize <= self.settings.library.roots.len()
                {
                    self.settings
                        .library
                        .roots
                        .remove(self.library_folder as usize - 1);
                    self.library_folder = 0;
                    self.settings_dirty = Some(Instant::now());
                    self.refresh_library();
                    self.scan_library();
                }
            }
            "library-song-remove" => {
                if let Some(path) = index.and_then(|i| self.browser.paths.get(i)).cloned() {
                    self.settings.library.files.retain(|p| p != &path);
                    if !self.settings.library.excluded.contains(&path) {
                        if self.settings.library.excluded.len() >= 10000 {
                            self.status = "单曲排除已达到 10000 首，请按目录管理".into();
                            return true;
                        }
                        self.settings.library.excluded.push(path.clone());
                    }
                    self.settings_dirty = Some(Instant::now());
                    self.library_songs.retain(|s| s.path != path);
                    self.refresh_library();
                    self.scan_library();
                }
            }
            "library-play" => {
                if let Some(row) = self
                    .library_rows
                    .iter()
                    .position(|r| Some(r.id as usize) == index)
                {
                    let sources = self
                        .browser
                        .paths
                        .iter()
                        .cloned()
                        .map(MediaSource::Local)
                        .collect();
                    if self.replace_queue(QueueKind::Music, sources) {
                        self.preview = false;
                        self.load_index(row as i32);
                    }
                }
            }
            "appearance-design" => {
                self.settings.appearance.design = if value == "0" {
                    Design::Simple
                } else {
                    Design::Fashion
                }
            }
            "appearance-scheme" => {
                self.settings.appearance.scheme = match value {
                    "1" => Scheme::Light,
                    "2" => Scheme::Dark,
                    _ => Scheme::System,
                }
            }
            "appearance-accent-source" => self.settings.appearance.system_accent = value == "0",
            "appearance-accent" => {
                if parse_color(value).is_none() {
                    self.status = "请输入 #RRGGBB 主题色，再点击应用".into();
                    return true;
                }
                self.settings.appearance.accent = value.into();
                self.settings.appearance.system_accent = false;
            }
            "appearance-motion" => self.settings.appearance.reduce_motion = value == "yes",
            _ => return false,
        }
        if action.starts_with("appearance-") {
            self.settings_revision += 1;
            self.settings_dirty = Some(Instant::now());
        }
        true
    }
    pub(super) fn library_view(&self) -> player_ui::view_model::LibraryViewModel {
        let mut folders = vec!["全部音乐".into()];
        folders.extend(
            self.settings
                .library
                .roots
                .iter()
                .map(|p| p.display().to_string()),
        );
        folders.push("单独添加的歌曲".into());
        let current = self.queue.get(self.selected_id as usize);
        let selected = current
            .and_then(|src| {
                if let MediaSource::Local(p) = src {
                    self.browser.paths.iter().position(|path| path == p)
                } else {
                    None
                }
            })
            .map(|i| i as i32)
            .unwrap_or(-1);
        player_ui::view_model::LibraryViewModel {
            rows: self.library_rows.clone(),
            revision: self.library_revision,
            folders,
            folder: self.library_folder,
            busy: self.library_busy,
            message: self.library_message.clone(),
            selected,
        }
    }
    pub(super) fn appearance_view(&self) -> player_ui::view_model::AppearanceViewModel {
        let a = &self.settings.appearance;
        let os = self
            .appearance_observer
            .as_ref()
            .map(|o| o.snapshot())
            .unwrap_or_default();
        let color = if a.system_accent {
            os.accent.unwrap_or(0x6875e8)
        } else {
            parse_color(&a.accent).unwrap_or(0x6875e8)
        };
        player_ui::view_model::AppearanceViewModel {
            design: if a.design == Design::Simple { 0 } else { 1 },
            scheme: match a.scheme {
                Scheme::System => 0,
                Scheme::Light => 1,
                Scheme::Dark => 2,
            },
            dark: match a.scheme {
                Scheme::System => os.dark.unwrap_or(true),
                Scheme::Light => false,
                Scheme::Dark => true,
            },
            accent: color,
            ink: ink(color),
            accent_text: player_core::appearance::text_accent(
                color,
                match a.scheme {
                    Scheme::System => os.dark.unwrap_or(true),
                    Scheme::Light => false,
                    Scheme::Dark => true,
                },
            ),
            system_accent: a.system_accent,
            custom_accent: a.accent.clone(),
            reduce_motion: a.reduce_motion,
            os_reduce_motion: os.reduce_motion,
            system_available: os.dark.is_some(),
            library_background_style: a.library_background.style as i32,
            library_background_opacity: a.library_background.opacity as i32,
            library_background_blur: a.library_background.blur as i32,
            library_background_name: a
                .library_background
                .file
                .as_ref()
                .and_then(|p| p.file_name())
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_default(),
            lyrics_background_style: a.lyrics_background.style as i32,
            lyrics_background_opacity: a.lyrics_background.opacity as i32,
            lyrics_background_blur: a.lyrics_background.blur as i32,
            lyrics_background_name: a
                .lyrics_background
                .file
                .as_ref()
                .and_then(|p| p.file_name())
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_default(),
            custom_library_background: self
                .backgrounds
                .as_ref()
                .map(|b| b.image(0))
                .unwrap_or_default(),
            custom_lyrics_background: self
                .backgrounds
                .as_ref()
                .map(|b| b.image(1))
                .unwrap_or_default(),
            background_image_revision: self.backgrounds.as_ref().map(|b| b.revision()).unwrap_or(0),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn retry_keeps_accepted_edits_instead_of_restoring_an_older_snapshot() {
        let mut c = AppController::new(MpvEngine::default(), player_platform::current());
        c.music_ready = false;
        let mut saved = services::music_store::Collections {
            library: Default::default(),
            playlists: Default::default(),
        };
        saved.library.roots.push("music".into());
        saved.playlists.add(1, vec!["old.flac".into()]).unwrap();
        c.restore_music_collections(saved.clone());
        assert_eq!(c.settings.playlists, saved.playlists);
        c.settings
            .playlists
            .add(1, vec!["new.flac".into()])
            .unwrap();
        c.settings.library.files.push("new.flac".into());
        c.music_retry = true;
        c.restore_music_collections(saved);
        assert!(
            c.settings
                .playlists
                .contains(1, std::path::Path::new("new.flac"))
        );
        assert_eq!(c.settings.library.files, [PathBuf::from("new.flac")]);
        assert!(c.music_ready && !c.music_retry && c.settings_dirty.is_some());
    }
    #[test]
    fn startup_import_is_bounded_deduplicated_and_applied_after_ready() {
        let mut c = AppController::new(MpvEngine::default(), player_platform::current());
        c.music_ready = false;
        c.import_library_paths(true, vec!["music".into(), "music".into()]);
        c.import_library_paths(false, vec!["single.flac".into()]);
        assert!(c.settings.library.roots.is_empty() && c.settings.library.files.is_empty());
        assert_eq!(c.pending_music_import.roots, [PathBuf::from("music")]);
        let prior = c.pending_music_import.clone();
        c.import_library_paths(
            true,
            (0..129)
                .map(|i| PathBuf::from(format!("extra{i}")))
                .collect(),
        );
        assert_eq!(c.pending_music_import, prior);
        c.music_ready = true;
        let pending = std::mem::take(&mut c.pending_music_import);
        c.import_library_paths(true, pending.roots);
        c.import_library_paths(false, pending.files);
        assert_eq!(c.settings.library, prior);
        assert!(c.settings_dirty.is_some());
    }
}
