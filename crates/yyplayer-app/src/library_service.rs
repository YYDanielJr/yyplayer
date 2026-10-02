//! Bounded, cancellable directory indexing. No Slint handles or playback calls.
use player_core::library::{LibrarySettings, Song, VIDEO_EXTENSIONS, is_audio, is_video};
use serde::{Deserialize, Serialize};
use std::{
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
        mpsc::{Receiver, SyncSender, sync_channel},
    },
};
#[derive(Serialize, Deserialize)]
struct Cache {
    version: u32,
    #[serde(default)]
    video: bool,
    config: LibrarySettings,
    songs: Vec<Song>,
}
pub enum Request {
    Scan(u64, LibrarySettings),
    Folder,
    Files,
    Paths(bool, Vec<PathBuf>),
}
pub enum Reply {
    Paths(bool, Vec<PathBuf>),
    Indexed(u64, LibrarySettings, Vec<Song>, String),
    Error(String),
}
pub struct Service {
    pub commands: SyncSender<Request>,
    pub replies: Receiver<Reply>,
    pub generation: Arc<AtomicU64>,
    worker: Option<std::thread::JoinHandle<()>>,
}
impl Service {
    pub fn start(cache: PathBuf, config: LibrarySettings) -> Self {
        Self::start_kind(cache, config, false)
    }
    pub fn start_video(cache: PathBuf, config: LibrarySettings) -> Self {
        Self::start_kind(cache, config, true)
    }
    fn start_kind(cache: PathBuf, config: LibrarySettings, video: bool) -> Self {
        let (tx, rx) = sync_channel::<Request>(2);
        let (out, done) = sync_channel(4);
        let generation = Arc::new(AtomicU64::new(0));
        let cancel = generation.clone();
        let worker = std::thread::spawn(move || {
            let loaded = std::fs::metadata(&cache)
                .ok()
                .filter(|m| m.len() <= 32_000_000)
                .and_then(|_| std::fs::read(&cache).ok())
                .and_then(|b| serde_json::from_slice::<Cache>(&b).ok())
                .filter(|c| {
                    c.version == 1
                        && c.video == video
                        && c.config == config
                        && c.songs.len() <= 20000
                });
            let mut prior = loaded.as_ref().map(|c| c.songs.clone()).unwrap_or_default();
            if let Some(c) = loaded
                && out
                    .send(Reply::Indexed(
                        0,
                        c.config,
                        c.songs,
                        if video {
                            "已恢复视频库"
                        } else {
                            "已恢复音乐库"
                        }
                        .into(),
                    ))
                    .is_err()
            {
                return;
            }
            while let Ok(request) = rx.recv() {
                let reply = match request {
                    Request::Paths(folder, paths) => Some(Reply::Paths(folder, paths)),
                    Request::Folder => rfd::FileDialog::new()
                        .set_title(if video {
                            "添加视频目录（包含子目录）"
                        } else {
                            "添加音乐目录（包含子目录）"
                        })
                        .pick_folder()
                        .map(|p| Reply::Paths(true, vec![p])),
                    Request::Files => rfd::FileDialog::new()
                        .set_title(if video {
                            "添加视频到视频库"
                        } else {
                            "添加歌曲到音乐库"
                        })
                        .add_filter(
                            if video { "视频" } else { "音频" },
                            if video {
                                VIDEO_EXTENSIONS
                            } else {
                                &[
                                    "mp3", "flac", "wav", "m4a", "ogg", "opus", "aac", "aif",
                                    "aiff", "ape", "wv", "caf", "dsf", "dff",
                                ]
                            },
                        )
                        .pick_files()
                        .map(|p| Reply::Paths(false, p)),
                    Request::Scan(id, config) => {
                        if cancel.load(Ordering::Relaxed) != id {
                            continue;
                        }
                        let result = scan_kind(&config, &cancel, id, video);
                        if cancel.load(Ordering::Relaxed) != id {
                            continue;
                        }
                        let (mut songs, mut message, unreadable) = result;
                        let mut retained = 0;
                        let mut indexed: std::collections::BTreeSet<_> =
                            songs.iter().map(|s| s.path.clone()).collect();
                        for song in &prior {
                            if cancel.load(Ordering::Relaxed) != id {
                                break;
                            }
                            if unreadable.iter().any(|p| song.path.starts_with(p))
                                && !config.excluded.contains(&song.path)
                                && indexed.insert(song.path.clone())
                                && songs.len() < 20000
                            {
                                songs.push(song.clone());
                                retained += 1;
                            }
                        }
                        if cancel.load(Ordering::Relaxed) != id {
                            continue;
                        }
                        songs.sort_by(|a, b| a.path.cmp(&b.path));
                        if retained > 0 {
                            message.push_str(&format!("；离线 / 不可读目录保留 {retained} 项缓存"));
                        }
                        prior = songs.clone();
                        let cache_value = Cache {
                            version: 1,
                            video,
                            config: config.clone(),
                            songs: songs.clone(),
                        };
                        match serde_json::to_vec(&cache_value)
                            .map_err(|e| e.to_string())
                            .and_then(|bytes| {
                                if bytes.len() > 32_000_000 {
                                    Err("媒体索引超过 32MB".into())
                                } else {
                                    crate::services::atomic_bytes(&cache, &bytes)
                                }
                            }) {
                            Ok(()) => {}
                            Err(e) => message.push_str(&format!("；索引保存失败：{e}")),
                        }
                        Some(Reply::Indexed(id, config, songs, message))
                    }
                };
                let reply = reply.map(|reply| match reply {
                    Reply::Paths(folder, paths) => {
                        let paths = paths
                            .into_iter()
                            .map(|p| {
                                p.canonicalize()
                                    .map_err(|e| format!("无法添加 {}：{e}", p.display()))
                            })
                            .collect::<Result<Vec<_>, _>>();
                        match paths {
                            Ok(p) => Reply::Paths(folder, p),
                            Err(e) => Reply::Error(e),
                        }
                    }
                    r => r,
                });
                if let Some(reply) = reply
                    && out.send(reply).is_err()
                {
                    break;
                }
            }
        });
        Self {
            commands: tx,
            replies: done,
            generation,
            worker: Some(worker),
        }
    }
    pub fn finish(mut self) {
        self.generation.store(u64::MAX, Ordering::Relaxed);
        drop(self.commands);
        drop(self.replies);
        if let Some(w) = self.worker.take() {
            let _ = w.join();
        }
    }
}
#[cfg(test)]
fn scan(
    config: &LibrarySettings,
    cancel: &AtomicU64,
    id: u64,
) -> (Vec<Song>, String, Vec<PathBuf>) {
    scan_kind(config, cancel, id, false)
}
fn scan_kind(
    config: &LibrarySettings,
    cancel: &AtomicU64,
    id: u64,
    video: bool,
) -> (Vec<Song>, String, Vec<PathBuf>) {
    use std::collections::BTreeSet;
    let accepts = |p: &std::path::Path| if video { is_video(p) } else { is_audio(p) };
    let mut paths = BTreeSet::new();
    let excluded: BTreeSet<_> = config.excluded.iter().cloned().collect();
    let mut problems = 0usize;
    let mut limited = false;
    let mut visited = 0usize;
    let mut unreadable = vec![];
    for root in &config.roots {
        let mut stack = vec![(root.clone(), 0usize)];
        while let Some((dir, depth)) = stack.pop() {
            if cancel.load(Ordering::Relaxed) != id {
                return (vec![], "已取消".into(), vec![]);
            }
            if depth > 64 || visited >= 200000 || paths.len() >= 20000 {
                limited = true;
                break;
            }
            let entries = match std::fs::read_dir(&dir) {
                Ok(v) => v,
                Err(_) => {
                    problems += 1;
                    unreadable.push(dir.clone());
                    continue;
                }
            };
            for entry in entries {
                visited += 1;
                if visited >= 200000 || paths.len() >= 20000 {
                    limited = true;
                    break;
                }
                let Ok(entry) = entry else {
                    problems += 1;
                    continue;
                };
                let p = entry.path();
                let Ok(meta) = std::fs::symlink_metadata(&p) else {
                    problems += 1;
                    continue;
                };
                if meta.file_type().is_symlink() || reparse(&meta) {
                    continue;
                }
                if meta.is_dir() {
                    stack.push((p, depth + 1));
                } else if meta.is_file() && accepts(&p) {
                    match p.canonicalize() {
                        Ok(p) if !excluded.contains(&p) => {
                            paths.insert(p);
                        }
                        Ok(_) => {}
                        Err(_) => problems += 1,
                    }
                }
            }
        }
    }
    for path in &config.files {
        if paths.len() >= 20000 {
            limited = true;
            break;
        }
        if accepts(path) && !excluded.contains(path) {
            paths.insert(path.clone());
        }
    }
    let mut songs = Vec::with_capacity(paths.len());
    use lofty::{
        config::ParseOptions,
        prelude::{Accessor, AudioFile, TaggedFileExt},
        probe::Probe,
    };
    for path in paths {
        if cancel.load(Ordering::Relaxed) != id {
            return (vec![], "已取消".into(), vec![]);
        }
        let mut song = Song {
            title: path
                .file_stem()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned(),
            path: path.clone(),
            artist: String::new(),
            album: String::new(),
            seconds: 0,
        };
        if video {
            song.artist = path.parent().map(display_path).unwrap_or_default();
            song.album = std::fs::metadata(&path)
                .map(|m| {
                    if m.len() < 1_000_000 {
                        format!("{:.1} KB", m.len() as f64 / 1_000.0)
                    } else {
                        format!("{:.1} MB", m.len() as f64 / 1_000_000.0)
                    }
                })
                .unwrap_or_else(|_| "文件失联".into());
            songs.push(song);
            continue;
        }
        match Probe::open(&path)
            .and_then(|p| p.options(ParseOptions::new().read_cover_art(false)).read())
        {
            Ok(file) => {
                song.seconds = file.properties().duration().as_secs();
                if let Some(tag) = file.primary_tag().or_else(|| file.first_tag()) {
                    if let Some(t) = tag.title() {
                        song.title = t.into_owned();
                    }
                    song.artist = tag.artist().map(|v| v.into_owned()).unwrap_or_default();
                    song.album = tag.album().map(|v| v.into_owned()).unwrap_or_default();
                }
            }
            Err(_) => problems += 1,
        }
        songs.push(song);
    }
    let mut message = format!(
        "已索引 {} {}",
        songs.len(),
        if video { "个视频" } else { "首歌曲" }
    );
    if problems > 0 {
        message.push_str(&format!(
            "；{problems} 项读取失败或标签不可识别，可重新扫描"
        ));
    }
    if limited {
        message.push_str("；达到 20000 个媒体 / 200000 项 / 64 层限制，请拆分目录");
    }
    (songs, message, unreadable)
}
pub(super) fn display_path(path: &std::path::Path) -> String {
    let text = path.to_string_lossy();
    if let Some(unc) = text.strip_prefix(r"\\?\UNC\") {
        format!(r"\\{unc}")
    } else {
        text.strip_prefix(r"\\?\").unwrap_or(&text).into()
    }
}
#[cfg(windows)]
fn reparse(m: &std::fs::Metadata) -> bool {
    use std::os::windows::fs::MetadataExt;
    m.file_attributes() & 0x400 != 0
}
#[cfg(not(windows))]
fn reparse(_: &std::fs::Metadata) -> bool {
    false
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cache_recovers_offline_folder_and_supersedes_old_scan() {
        let base =
            std::env::temp_dir().join(format!("yyplayer-library-cache-{}", std::process::id()));
        std::fs::create_dir_all(&base).unwrap();
        let missing = base.join("offline");
        let config = LibrarySettings {
            roots: vec![missing.clone()],
            ..Default::default()
        };
        let cache = base.join("index.json");
        let song = Song {
            path: missing.join("saved.flac"),
            title: "Cached".into(),
            artist: String::new(),
            album: String::new(),
            seconds: 12,
        };
        crate::services::atomic_bytes(
            &cache,
            &serde_json::to_vec(&Cache {
                version: 1,
                video: false,
                config: config.clone(),
                songs: vec![song],
            })
            .unwrap(),
        )
        .unwrap();
        let service = Service::start(cache, config.clone());
        assert!(
            matches!(service.replies.recv_timeout(std::time::Duration::from_secs(3)).unwrap(), Reply::Indexed(0, _, songs, _) if songs.len() == 1)
        );
        service.generation.store(3, Ordering::Relaxed);
        service
            .commands
            .send(Request::Scan(2, config.clone()))
            .unwrap();
        service.commands.send(Request::Scan(3, config)).unwrap();
        assert!(
            matches!(service.replies.recv_timeout(std::time::Duration::from_secs(3)).unwrap(), Reply::Indexed(3, _, songs, message) if songs.len() == 1 && message.contains("保留"))
        );
        service.finish();
        std::fs::remove_dir_all(base).unwrap();
    }
    #[test]
    fn scan_deduplicates_excludes_and_cancels() {
        let root = std::env::temp_dir().join(format!("yyplayer-library-{}", std::process::id()));
        std::fs::create_dir_all(root.join("sub")).unwrap();
        let file = root.join("sub/a.FLAC");
        std::fs::write(&file, b"invalid tag but visible").unwrap();
        std::fs::write(root.join("video.mp4"), b"").unwrap();
        let root = root.canonicalize().unwrap();
        let file = file.canonicalize().unwrap();
        let mut c = LibrarySettings {
            roots: vec![root.clone(), root.join("sub")],
            files: vec![file.clone()],
            excluded: vec![],
        };
        let cancel = AtomicU64::new(1);
        assert_eq!(scan(&c, &cancel, 1).0.len(), 1);
        c.excluded.push(file);
        assert!(scan(&c, &cancel, 1).0.is_empty());
        assert!(scan(&c, &cancel, 2).0.is_empty());
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn video_index_is_separate_bounded_and_preserves_raw_identity() {
        let base =
            std::env::temp_dir().join(format!("yyplayer-video-index-{}", std::process::id()));
        std::fs::create_dir_all(base.join("sub")).unwrap();
        let path = base.join("sub/湖边.MKV");
        std::fs::write(&path, b"video candidate, not decoded during scanning").unwrap();
        std::fs::write(base.join("tone.wav"), b"audio").unwrap();
        let base = base.canonicalize().unwrap();
        let path = path.canonicalize().unwrap();
        let mut config = LibrarySettings {
            roots: vec![base.clone(), base.join("sub")],
            files: vec![path.clone()],
            excluded: vec![],
        };
        let cancel = AtomicU64::new(1);
        let (items, _, _) = scan_kind(&config, &cancel, 1, true);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].path, path);
        assert_eq!(items[0].title, "湖边");
        assert_eq!(items[0].seconds, 0, "Unknown duration must not be invented");
        config.excluded.push(path.clone());
        assert!(scan_kind(&config, &cancel, 1, true).0.is_empty());
        assert!(scan_kind(&config, &cancel, 2, true).0.is_empty());
        assert!(path.exists(), "Removing a record never deletes the source");
        std::fs::remove_dir_all(base).unwrap();
    }
}
