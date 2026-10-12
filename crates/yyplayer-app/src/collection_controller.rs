use super::*;
use player_core::{library::Song, playlists::FAVORITES};
use std::{
    collections::{BTreeMap, BTreeSet},
    rc::Rc,
};

#[derive(Default)]
pub(super) struct Browser {
    pub category: i32,
    pub group: Option<(String, String)>,
    pub keys: Vec<(String, String)>,
    pub groups: Rc<Vec<MediaPreview>>,
    pub playlist: usize,
    pub paths: Rc<Vec<PathBuf>>,
    pub checked: Rc<Vec<bool>>,
    pub selection: BTreeSet<PathBuf>,
    pub revision: u64,
}
fn artist(s: &Song) -> String {
    if s.artist.trim().is_empty() {
        "未知歌手".into()
    } else {
        s.artist.trim().into()
    }
}
fn album(s: &Song) -> String {
    if s.album.trim().is_empty() {
        "未知专辑".into()
    } else {
        s.album.trim().into()
    }
}
fn key(s: &Song, category: i32) -> (String, String) {
    (
        artist(s).to_lowercase(),
        if category == 2 {
            album(s).to_lowercase()
        } else {
            String::new()
        },
    )
}
impl AppController {
    pub(super) fn build_collection(&mut self, indices: Vec<usize>) {
        let category = self.browser.category;
        let mut groups = BTreeMap::<(String, String), (String, String, usize)>::new();
        if matches!(category, 1 | 2) {
            for &i in &indices {
                let s = &self.library_songs[i];
                let g = groups.entry(key(s, category)).or_insert_with(|| {
                    (
                        if category == 1 { artist(s) } else { album(s) },
                        artist(s),
                        0,
                    )
                });
                g.2 += 1;
            }
        }
        self.browser.keys = groups.keys().cloned().collect();
        self.browser.groups = Rc::new(
            groups
                .into_iter()
                .enumerate()
                .map(|(i, (_, (name, artist, n)))| MediaPreview {
                    id: i as i32,
                    title: name,
                    artist: if category == 2 {
                        artist
                    } else {
                        format!("{n} 首歌曲")
                    },
                    collection: format!("{n} 首歌曲"),
                    duration: String::new(),
                    cover: -1,
                    artwork: Default::default(),
                })
                .collect(),
        );
        let songs: BTreeMap<_, _> = self.library_songs.iter().map(|s| (&s.path, s)).collect();
        let paths: Vec<_> = if category == 3 {
            let query = self.search.trim().to_lowercase();
            self.settings
                .playlists
                .lists
                .get(self.browser.playlist)
                .into_iter()
                .flat_map(|p| &p.tracks)
                .filter(|p| {
                    query.is_empty()
                        || songs.get(p).is_some_and(|s| {
                            format!("{} {} {}", s.title, s.artist, s.album)
                                .to_lowercase()
                                .contains(&query)
                        })
                        || p.to_string_lossy().to_lowercase().contains(&query)
                })
                .cloned()
                .collect()
        } else {
            indices
                .into_iter()
                .filter(|i| {
                    self.browser
                        .group
                        .as_ref()
                        .is_none_or(|g| &key(&self.library_songs[*i], category) == g)
                })
                .map(|i| self.library_songs[i].path.clone())
                .collect()
        };
        self.library_rows = Rc::new(
            paths
                .iter()
                .enumerate()
                .map(|(i, path)| {
                    let s = songs.get(path);
                    MediaPreview {
                        id: i as i32,
                        title: s
                            .map(|s| s.title.clone())
                            .unwrap_or_else(|| source_title(&MediaSource::Local(path.clone()))),
                        artist: s.map(|s| artist(s)).unwrap_or_else(|| "未索引歌曲".into()),
                        collection: s
                            .map(|s| album(s))
                            .unwrap_or_else(|| "保留的歌单记录".into()),
                        duration: s
                            .filter(|s| s.seconds > 0)
                            .map(|s| format_time(Some(Duration::from_secs(s.seconds))))
                            .unwrap_or_default(),
                        cover: -1,
                        artwork: self
                            .thumbnails
                            .as_ref()
                            .map(|t| t.image(path))
                            .unwrap_or_default(),
                    }
                })
                .collect(),
        );
        self.browser.paths = Rc::new(paths);
        self.refresh_checks();
        self.library_revision += 1;
    }
    fn refresh_checks(&mut self) {
        let visible: BTreeSet<_> = self.browser.paths.iter().cloned().collect();
        self.browser.selection.retain(|p| visible.contains(p));
        self.browser.checked = Rc::new(
            self.browser
                .paths
                .iter()
                .map(|p| self.browser.selection.contains(p))
                .collect(),
        );
        self.browser.revision += 1;
    }
    fn batch_paths(&self) -> Vec<PathBuf> {
        self.browser
            .paths
            .iter()
            .filter(|p| self.browser.selection.is_empty() || self.browser.selection.contains(*p))
            .cloned()
            .collect()
    }
    pub(super) fn toggle_favorite(&mut self) {
        let snapshot = self.engine.snapshot();
        let Some(MediaSource::Local(path)) = snapshot
            .source
            .as_ref()
            .or_else(|| self.queue.get(self.selected_id as usize))
        else {
            self.status = "本地歌曲或已下载歌曲可以加入我喜欢".into();
            return;
        };
        if self.queue_kind != QueueKind::Music {
            self.status = "我喜欢用于音乐歌单".into();
            return;
        }
        let path = path.clone();
        if self.settings.playlists.contains(FAVORITES, &path) {
            self.settings
                .playlists
                .remove(FAVORITES, &BTreeSet::from([path]));
        } else if let Err(e) = self.settings.playlists.add(FAVORITES, vec![path]) {
            self.status = e;
            return;
        }
        self.settings_dirty = Some(Instant::now());
        self.refresh_library();
    }
    pub(super) fn collection_control(&mut self, action: &str, value: &str) -> bool {
        let index = value.parse::<usize>().ok();
        match action {
            "library-category" => {
                self.browser.category = value.parse::<i32>().unwrap_or(0).clamp(0, 3);
                self.browser.group = None;
                self.browser.selection.clear();
                self.refresh_library();
            }
            "library-group" => {
                self.browser.group = index.and_then(|i| self.browser.keys.get(i)).cloned();
                self.browser.selection.clear();
                self.refresh_library();
            }
            "library-group-back" => {
                self.browser.group = None;
                self.browser.selection.clear();
                self.refresh_library();
            }
            "library-check" => {
                if let Some(p) = index.and_then(|i| self.browser.paths.get(i)).cloned() {
                    if !self.browser.selection.remove(&p) {
                        self.browser.selection.insert(p);
                    }
                    self.refresh_checks();
                }
            }
            "library-check-all" => {
                if self.browser.selection.len() == self.browser.paths.len() {
                    self.browser.selection.clear();
                } else {
                    self.browser.selection = self.browser.paths.iter().cloned().collect();
                }
                self.refresh_checks();
            }
            "library-batch-play" => {
                let sources = self
                    .batch_paths()
                    .into_iter()
                    .map(MediaSource::Local)
                    .collect::<Vec<_>>();
                if !sources.is_empty() && self.replace_queue(QueueKind::Music, sources) {
                    self.preview = false;
                    self.load_index(0);
                }
            }
            "library-enqueue" => {
                let paths = self.batch_paths();
                self.enqueue(
                    QueueKind::Music,
                    paths.into_iter().map(MediaSource::Local).collect(),
                );
            }
            "playlist-select" => {
                if let Some(i) = index
                    && i < self.settings.playlists.lists.len()
                {
                    self.browser.playlist = i;
                    self.browser.selection.clear();
                    self.refresh_library();
                }
            }
            "playlist-create" => match self.settings.playlists.create(value) {
                Ok(id) => {
                    self.browser.playlist = self
                        .settings
                        .playlists
                        .lists
                        .iter()
                        .position(|p| p.id == id)
                        .unwrap_or(0);
                    self.browser.category = 3;
                    self.browser.selection.clear();
                    self.settings_dirty = Some(Instant::now());
                    self.refresh_library();
                }
                Err(e) => self.status = e,
            },
            "playlist-rename" => {
                if let Some(id) = self
                    .settings
                    .playlists
                    .lists
                    .get(self.browser.playlist)
                    .map(|p| p.id)
                {
                    match self.settings.playlists.rename(id, value) {
                        Ok(()) => {
                            self.settings_dirty = Some(Instant::now());
                            self.refresh_library();
                        }
                        Err(e) => self.status = e,
                    }
                }
            }
            "playlist-delete" => {
                if let Some(id) = self
                    .settings
                    .playlists
                    .lists
                    .get(self.browser.playlist)
                    .map(|p| p.id)
                {
                    match self.settings.playlists.delete(id) {
                        Ok(()) => {
                            self.browser.playlist = 0;
                            self.settings_dirty = Some(Instant::now());
                            self.refresh_library();
                        }
                        Err(e) => self.status = e,
                    }
                }
            }
            "playlist-add" => {
                let paths = self.batch_paths();
                if let Some(id) = index
                    .and_then(|i| self.settings.playlists.lists.get(i))
                    .map(|p| p.id)
                {
                    match self.settings.playlists.add(id, paths) {
                        Ok(n) => {
                            self.status = format!("已添加 {n} 首到歌单");
                            self.settings_dirty = Some(Instant::now());
                            self.refresh_library();
                        }
                        Err(e) => self.status = e,
                    }
                }
            }
            "playlist-remove" => {
                if let Some(id) = self
                    .settings
                    .playlists
                    .lists
                    .get(self.browser.playlist)
                    .map(|p| p.id)
                {
                    let paths = self.batch_paths().into_iter().collect();
                    self.settings.playlists.remove(id, &paths);
                    self.settings_dirty = Some(Instant::now());
                    self.refresh_library();
                }
            }
            "video-enqueue-all" => {
                let paths = self
                    .video_rows
                    .iter()
                    .filter_map(|r| self.video_songs.get(r.id as usize))
                    .map(|s| MediaSource::Local(s.path.clone()))
                    .collect();
                self.enqueue(QueueKind::Video, paths);
            }
            "video-batch-play" => {
                let sources = self
                    .video_rows
                    .iter()
                    .filter_map(|r| self.video_songs.get(r.id as usize))
                    .map(|s| MediaSource::Local(s.path.clone()))
                    .collect::<Vec<_>>();
                if !sources.is_empty() && self.replace_queue(QueueKind::Video, sources) {
                    self.preview = false;
                    self.load_index(0);
                }
            }
            _ => return false,
        }
        true
    }
    pub(super) fn collection_view(&self) -> player_ui::view_model::CollectionViewModel {
        player_ui::view_model::CollectionViewModel {
            revision: self.browser.revision,
            category: self.browser.category,
            groups: self.browser.groups.clone(),
            group_open: self.browser.group.is_some(),
            title: if self.browser.category == 3 {
                self.settings
                    .playlists
                    .lists
                    .get(self.browser.playlist)
                    .map(|p| p.name.clone())
                    .unwrap_or_default()
            } else {
                self.browser
                    .group
                    .as_ref()
                    .and_then(|g| self.browser.keys.iter().position(|k| k == g))
                    .and_then(|i| self.browser.groups.get(i))
                    .map(|r| r.title.clone())
                    .unwrap_or_default()
            },
            playlists: self
                .settings
                .playlists
                .lists
                .iter()
                .map(|p| p.name.clone())
                .collect(),
            playlist: self.browser.playlist as i32,
            checked: self.browser.checked.clone(),
            selected_count: self.browser.selection.len() as i32,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn classification_selection_playlist_retention_and_queue_are_independent() {
        let mut c = AppController::new(MpvEngine::default(), player_platform::current());
        c.preview = false;
        c.settings.library.roots.push("music".into());
        c.library_songs = (0..4)
            .map(|i| Song {
                path: format!("music/{i}.flac").into(),
                title: format!("song{i}"),
                artist: if i < 2 { "A" } else { "B" }.into(),
                album: "Live".into(),
                seconds: 60,
            })
            .collect();
        c.refresh_library();
        assert_eq!(c.library_rows.len(), 4);
        c.control("library-category", "1");
        assert_eq!(c.browser.groups.len(), 2);
        c.control("library-group", "0");
        assert_eq!(c.library_rows.len(), 2);
        c.control("library-check", "1");
        c.control("library-enqueue", "");
        assert_eq!(c.queue.len(), 1);
        assert_eq!(c.queue[0], MediaSource::Local("music/1.flac".into()));
        c.control("playlist-create", "Night");
        c.control("library-category", "2");
        assert_eq!(c.browser.groups.len(), 2);
        c.control("library-group", "1");
        c.control("playlist-add", "1");
        assert_eq!(c.settings.playlists.lists[1].tracks.len(), 2);
        c.control("playlist-add", "1");
        assert_eq!(c.settings.playlists.lists[1].tracks.len(), 2);
        c.control("library-category", "3");
        c.control("playlist-select", "1");
        assert_eq!(c.library_rows.len(), 2);
        c.library_songs.clear();
        c.settings.library.roots.clear();
        c.refresh_library();
        assert_eq!(c.library_rows.len(), 2);
        assert_eq!(c.queue.len(), 1);
        assert_eq!(c.library_rows[0].artist, "未索引歌曲");
        c.control("library-check", "0");
        c.control("playlist-remove", "");
        assert_eq!(c.library_rows.len(), 1);
        c.control("playlist-delete", "");
        assert_eq!(c.queue.len(), 1);
        assert_eq!(c.settings.playlists.lists.len(), 1);
        c.settings.validate().unwrap();
    }
    #[test]
    fn album_keys_disambiguate_artists_and_handle_empty_tags() {
        let s = Song {
            path: "a".into(),
            title: "song".into(),
            artist: "A".into(),
            album: "Live".into(),
            seconds: 0,
        };
        let mut b = s.clone();
        b.artist = "B".into();
        assert_ne!(key(&s, 2), key(&b, 2));
        b.artist = " a ".into();
        b.album = " LIVE ".into();
        assert_eq!(key(&s, 2), key(&b, 2));
        b.artist = String::new();
        b.album = String::new();
        assert_eq!(artist(&b), "未知歌手");
        assert_eq!(album(&b), "未知专辑");
    }
}
