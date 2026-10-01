//! Bounded background disk/dialog services; never run these in Slint callbacks.
use player_core::settings::Settings;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, SyncSender, sync_channel};

pub fn settings_path() -> PathBuf {
    if let Some(path) = std::env::var_os("YYPLAYER_CONFIG") {
        return PathBuf::from(path);
    }
    #[cfg(windows)]
    let base = std::env::var_os("APPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    #[cfg(target_os = "macos")]
    let base = std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_default()
        .join("Library/Application Support");
    #[cfg(not(any(windows, target_os = "macos")))]
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            std::env::var_os("HOME")
                .map(PathBuf::from)
                .unwrap_or_default()
                .join(".config")
        });
    base.join("YYPlayer/settings.json")
}
pub fn load(path: &Path) -> (Settings, String) {
    let result = || -> Result<Settings, String> {
        if !path.exists() {
            return Ok(Settings::default());
        }
        if std::fs::metadata(path).map_err(|e| e.to_string())?.len() > 2_000_000 {
            return Err("设置文件超过 2 MB".into());
        }
        let settings: Settings =
            serde_json::from_slice(&std::fs::read(path).map_err(|e| e.to_string())?)
                .map_err(|e| e.to_string())?;
        settings.validate()?;
        Ok(settings)
    };
    match result() {
        Ok(settings) => (settings, String::new()),
        Err(error) => {
            let backup = path.with_extension(format!(
                "corrupt-{}.json",
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs()
            ));
            let copied = std::fs::copy(path, &backup).is_ok();
            (
                Settings::default(),
                format!(
                    "设置读取失败：{error}；{}",
                    if copied {
                        format!("原始文件已备份至 {}", backup.display())
                    } else {
                        "原始文件保留；保存前请检查路径权限".into()
                    }
                ),
            )
        }
    }
}
pub fn atomic_save(path: &Path, settings: &Settings) -> Result<(), String> {
    settings.validate()?;
    let bytes = serde_json::to_vec_pretty(settings).map_err(|e| e.to_string())?;
    if bytes.len() > 2_000_000 {
        return Err("设置超过 2MB，原文件保留；请减少文件覆盖或预设数量".into());
    }
    atomic_bytes(path, &bytes)
}
pub fn atomic_bytes(path: &Path, bytes: &[u8]) -> Result<(), String> {
    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    static SAVE_ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let mut name = path.as_os_str().to_owned();
    name.push(format!(
        ".yyplayer-{}-{}.tmp",
        std::process::id(),
        SAVE_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    let temporary = PathBuf::from(name);
    use std::io::Write;
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)
        .map_err(|e| e.to_string())?;
    file.write_all(bytes)
        .and_then(|_| file.sync_all())
        .map_err(|e| e.to_string())?;
    drop(file);
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
        let from: Vec<_> = temporary.as_os_str().encode_wide().chain(Some(0)).collect();
        let to: Vec<_> = path.as_os_str().encode_wide().chain(Some(0)).collect();
        // SAFETY: nul-terminated UTF-16 paths live throughout atomic same-volume replace.
        if unsafe {
            windows_sys::Win32::Storage::FileSystem::MoveFileExW(
                from.as_ptr(),
                to.as_ptr(),
                windows_sys::Win32::Storage::FileSystem::MOVEFILE_REPLACE_EXISTING
                    | windows_sys::Win32::Storage::FileSystem::MOVEFILE_WRITE_THROUGH,
            )
        } == 0
        {
            return Err(std::io::Error::last_os_error().to_string());
        }
    }
    #[cfg(not(windows))]
    std::fs::rename(temporary, path).map_err(|e| e.to_string())?;
    Ok(())
}
pub struct Persistence {
    pub commands: SyncSender<(u64, Settings)>,
    pub replies: Receiver<(u64, Result<(), String>)>,
    worker: Option<std::thread::JoinHandle<()>>,
}
impl Persistence {
    pub fn start(path: PathBuf) -> Self {
        let (tx, rx) = sync_channel::<(u64, Settings)>(4);
        let (done_tx, done_rx) = sync_channel(4);
        let worker = std::thread::spawn(move || {
            while let Ok((revision, settings)) = rx.recv() {
                let result = atomic_save(&path, &settings);
                if done_tx.send((revision, result)).is_err() {
                    break;
                }
            }
        });
        Self {
            commands: tx,
            replies: done_rx,
            worker: Some(worker),
        }
    }
    pub fn finish(mut self) {
        drop(self.commands);
        for _ in self.replies {}
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}
pub enum DialogRequest {
    Open,
    Subtitle,
    Screenshot,
    Paths(Vec<PathBuf>),
}
pub enum DialogReply {
    Open(Vec<PathBuf>),
    Subtitle(PathBuf),
    Screenshot(PathBuf),
    Error(String),
}
pub struct Dialogs {
    pub commands: SyncSender<DialogRequest>,
    pub replies: Receiver<DialogReply>,
}
impl Dialogs {
    pub fn start() -> Self {
        let (tx, rx) = sync_channel(4);
        let (reply_tx, reply_rx) = sync_channel(4);
        std::thread::spawn(move || {
            while let Ok(request) = rx.recv() {
                let reply = match request {
                    DialogRequest::Open => rfd::FileDialog::new()
                        .set_title("打开音频 / 视频，可多选")
                        .add_filter(
                            "媒体",
                            &[
                                "mp4", "mkv", "mov", "webm", "avi", "ts", "m2ts", "wmv", "flv",
                                "mp3", "flac", "wav", "m4a", "ogg", "opus", "aac", "aif", "aiff",
                                "ape", "wv", "caf", "dsf", "dff",
                            ],
                        )
                        .add_filter("所有文件", &["*"])
                        .pick_files()
                        .map(DialogReply::Open),
                    DialogRequest::Subtitle => rfd::FileDialog::new()
                        .set_title("加载字幕")
                        .add_filter("字幕", &["srt", "ass", "ssa", "vtt", "sub", "idx"])
                        .pick_file()
                        .map(DialogReply::Subtitle),
                    DialogRequest::Screenshot => rfd::FileDialog::new()
                        .set_title("保存含字幕截图")
                        .set_file_name("YYPlayer.png")
                        .add_filter("PNG", &["png"])
                        .save_file()
                        .map(DialogReply::Screenshot),
                    DialogRequest::Paths(paths) => Some(DialogReply::Open(paths)),
                };
                let reply = match reply {
                    Some(DialogReply::Open(paths)) => {
                        let prepared = paths
                            .into_iter()
                            .map(|path| {
                                path.canonicalize()
                                    .map_err(|e| format!("无法打开 {}：{e}", path.display()))
                            })
                            .collect::<Result<Vec<_>, _>>();
                        Some(match prepared {
                            Ok(paths) => DialogReply::Open(paths),
                            Err(error) => DialogReply::Error(error),
                        })
                    }
                    other => other,
                };
                if let Some(reply) = reply
                    && reply_tx.send(reply).is_err()
                {
                    break;
                }
            }
        });
        Self {
            commands: tx,
            replies: reply_rx,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn atomic_replace_and_recover_bad_data() {
        let directory =
            std::env::temp_dir().join(format!("yyplayer-settings-test-{}", std::process::id()));
        std::fs::create_dir_all(&directory).unwrap();
        let path = directory.join("settings.json");
        let mut settings = Settings::default();
        atomic_save(&path, &settings).unwrap();
        settings.volume = 43.0;
        atomic_save(&path, &settings).unwrap();
        assert_eq!(load(&path).0.volume, 43.0);
        std::fs::write(&path, b"{bad").unwrap();
        let (_, warning) = load(&path);
        assert!(!warning.is_empty());
        assert_eq!(std::fs::read(&path).unwrap(), b"{bad");
        // This exact process-owned test directory is under temp; do not recursively delete.
        for file in std::fs::read_dir(&directory).unwrap() {
            std::fs::remove_file(file.unwrap().path()).unwrap();
        }
        std::fs::remove_dir(directory).unwrap();
    }
}
