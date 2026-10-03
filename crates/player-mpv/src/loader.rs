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
    #[cfg(target_os = "linux")]
    return linux_runtime_path(&executable);
    #[cfg(not(target_os = "linux"))]
    {
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
}

#[cfg(target_os = "linux")]
fn linux_runtime_path(executable: &std::path::Path) -> Result<PathBuf, String> {
    let runtime = executable
        .parent()
        .ok_or("可执行文件目录不可用")?
        .join("runtime");
    let bundled = runtime.join("libmpv.so.2");
    // A present bundle must pass its own manifest; never silently bypass a damaged bundle.
    if bundled.exists() || runtime.join("manifest.json").exists() {
        let manifest: serde_json::Value = serde_json::from_slice(
            &std::fs::read(runtime.join("manifest.json"))
                .map_err(|e| format!("Linux 内核清单不可读：{e}"))?,
        )
        .map_err(|e| format!("Linux 内核清单损坏：{e}"))?;
        if manifest["platform"] != "linux-x86_64"
            || manifest["library"] != "libmpv.so.2"
            || manifest["client_api_major"] != 2
        {
            return Err("Linux 内核清单平台或 API 不匹配".into());
        }
        let digest = format!(
            "{:x}",
            Sha256::digest(std::fs::read(&bundled).map_err(|e| e.to_string())?)
        );
        if manifest["library_sha256"].as_str() != Some(&digest) {
            return Err("打包 libmpv 校验失败，请重新获取完整 AppImage".into());
        }
        return bundled.canonicalize().map_err(|e| e.to_string());
    }
    // Debian packages deliberately use the distribution's security-maintained runtime.
    // Absolute system locations only; no CWD, PATH or unqualified dlopen lookup.
    [
        "/usr/lib/x86_64-linux-gnu/libmpv.so.2",
        "/lib/x86_64-linux-gnu/libmpv.so.2",
        "/usr/lib64/libmpv.so.2",
        "/usr/lib/libmpv.so.2",
    ]
    .iter()
    .map(PathBuf::from)
    .find(|p| p.is_file())
    .ok_or("未找到 libmpv.so.2。Ubuntu 请安装 libmpv2（>= 0.41），或使用完整 AppImage。")?
    .canonicalize()
    .map_err(|e| e.to_string())
}
