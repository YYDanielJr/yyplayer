# YYPlayer

使用 Rust、Slint 和 libmpv 构建的本地音视频播放器。当前为 Windows x64 开发预览。

## 功能

- 音视频播放：队列、进度、倍速、音量、音轨与字幕选择；视频支持逐帧、截图、章节和 A–B 循环。
- 音频输出：设备选择、WASAPI 共享 / 优先独占 / 严格独占，查看实际输出状态；默认不强制升采样，不启用额外音效。
- 均衡器：手动调节、预设导入 / 导出，支持全局、设备和单文件设置，音视频共用。
- 音乐库：添加歌曲或递归扫描目录，搜索、筛选、重扫，显示标签和封面；移除记录不删除原文件。
- 歌曲大页面：点击左下封面展开，点击封面返回音乐库；自动加载同名 LRC 或内嵌歌词，也可手动导入 LRC / TXT。
- 视频界面：窗口、最大化与全屏，控件自动隐藏；可配置快捷键，方向键默认调音量、短按跳转 5 秒、长按临时 3 倍速。
- 播放设置：查看媒体与实际解码器信息，按全局、文件夹或单文件设置解码方式。
- 界面定制：简洁 / 时尚主题、浅色 / 深色与主题色，可跟随 Windows 系统；支持减少动效，以及界面、歌词、字幕字体设置。

macOS / Linux 尚未验收；HDR 显示输出、位准确输出、ASIO / DSD 和无缝播放仍待完善。独占输出不等于位准确输出。

## 编译

需要 Windows x64、Rust MSVC 工具链、Visual Studio C++ Build Tools 和 Windows SDK。已验证 Rust / Cargo 1.97.0，Slint / slint-build 固定为 1.17.1。

在仓库根目录用 PowerShell 执行：

```powershell
# 获取固定版本 libmpv，并校验下载文件和 DLL
powershell -ExecutionPolicy Bypass -File scripts/Get-Mpv.ps1
cargo build --locked --release -p yyplayer-app
```

首次获取依赖需要联网。运行时脚本需要能解压 7z 的 Windows `tar.exe`；解压失败时见 [运行时说明](third_party/mpv/README.md)。

## 运行

```powershell
.\target\release\yyplayer.exe
# 也可以直接打开本地媒体
.\target\release\yyplayer.exe "D:\Videos\example.mp4"
```

开发时使用 `cargo run --locked -p yyplayer-app`。保留 `third_party/mpv/windows-x64` 中的固定运行时，应用会自动查找并校验 DLL。

启动后在音乐库添加目录或歌曲，也可拖入文件。输出设备、独占、EQ 和歌词导入位于“播放选项”；外观、字体与快捷键位于“设置”。Windows 配置保存在 `%APPDATA%\YYPlayer\settings.json`。

## 文档与许可

[更新日志](CHANGELOG.md) · [开发指南](CONTRIBUTING.md) · [完整规划](docs/DEVELOPMENT_PLAN.md) · [实际状态与验收](docs/STATUS.md) · [GitHub 发布说明](docs/PUBLISHING.md)

项目自身源码与原创 UI 资源采用 **GPL-3.0-only**，见 [LICENSE](LICENSE)。第三方组件保留各自许可，见 [THIRD_PARTY_NOTICES](THIRD_PARTY_NOTICES.md)。当前源码主分支为 `master`，开发分支为 `dev`；二进制发行准备尚未完成。
