use sha2::{Digest, Sha256};
use std::path::PathBuf;

pub fn runtime_path() -> Result<PathBuf, String> {
    if let Some(path) = std::env::var_os("YYPLAYER_MPV_LIBRARY") {
        let path = PathBuf::from(path)
            .canonicalize()
            .map_err(|e| format!("自定义 libmpv 路径无效：{e}"))?;
        return Ok(path); // Explicit developer override; never search ambient PATH.
    }
    let executable = std::env::current_exe().map_err(|e| e.to_string())?;
    let manifest: serde_json::Value =
        serde_json::from_str(include_str!("../../../third_party/mpv/manifest.json"))
            .map_err(|e| e.to_string())?;
    #[cfg(windows)]
    let name = "libmpv-2.dll";
    #[cfg(target_os = "macos")]
    let name = "libmpv.2.dylib";
    #[cfg(not(any(windows, target_os = "macos")))]
    let name = "libmpv.so.2";
    let candidates = [
        executable
            .parent()
            .unwrap_or(std::path::Path::new("."))
            .join("runtime")
            .join(name),
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../third_party/mpv/windows-x64")
            .join(name),
    ];
    let path = candidates.into_iter().find(|path| path.is_file()).ok_or("未找到播放内核。Windows 请先运行 scripts/Get-Mpv.ps1；其他平台可设置 YYPLAYER_MPV_LIBRARY 为绝对路径。")?.canonicalize().map_err(|e| e.to_string())?;
    if cfg!(windows) {
        let bytes = std::fs::read(&path).map_err(|e| e.to_string())?;
        let digest = format!("{:x}", Sha256::digest(bytes));
        if Some(digest.as_str()) != manifest["library_sha256"].as_str() {
            return Err("libmpv 校验不符，请重新获取固定运行时".into());
        }
    }
    Ok(path)
}
