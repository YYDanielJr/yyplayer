//! Paths retain platform identity; removing a record never deletes a media file.
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct LibrarySettings {
    #[serde(with = "crate::path_serde::vec")]
    pub roots: Vec<PathBuf>,
    #[serde(with = "crate::path_serde::vec")]
    pub files: Vec<PathBuf>,
    #[serde(with = "crate::path_serde::vec")]
    pub excluded: Vec<PathBuf>,
}
impl LibrarySettings {
    pub fn validate(&self) -> Result<(), String> {
        if self.roots.len() > 128 || self.files.len() > 10000 || self.excluded.len() > 10000 {
            Err("媒体目录最多 128 个，单独添加 / 排除文件各最多 10000 个".into())
        } else {
            Ok(())
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Song {
    #[serde(with = "crate::path_serde")]
    pub path: PathBuf,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub seconds: u64,
}
pub fn is_audio(path: &std::path::Path) -> bool {
    path.extension().and_then(|e| e.to_str()).is_some_and(|e| {
        matches!(
            e.to_ascii_lowercase().as_str(),
            "mp3"
                | "flac"
                | "wav"
                | "m4a"
                | "ogg"
                | "opus"
                | "aac"
                | "aif"
                | "aiff"
                | "ape"
                | "wv"
                | "caf"
                | "dsf"
                | "dff"
                | "alac"
        )
    })
}

/// Candidate extensions for directory indexing; the engine confirms actual tracks.
pub const VIDEO_EXTENSIONS: &[&str] = &[
    "mp4", "mkv", "mov", "webm", "avi", "ts", "m2ts", "mts", "wmv", "flv", "m4v", "mpeg", "mpg",
    "vob", "ogv", "3gp", "hevc", "h265", "h264",
];
pub fn is_video(path: &std::path::Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| VIDEO_EXTENSIONS.contains(&e.to_ascii_lowercase().as_str()))
}
