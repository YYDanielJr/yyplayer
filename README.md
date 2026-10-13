# YYPlayer

一个用 Rust、Slint 和 libmpv 做的音视频播放器，主要用来播放和管理本地音乐、视频。

当前开发版本：**0.5.4**。Windows x64 是目前主要测试的平台；Ubuntu 26.04 amd64 已有运行记录，最近几次更新还需复测。macOS 尚未测试。

## 能做什么

- **听音乐、看视频**：调进度、音量和倍速，选择音轨、字幕；视频支持全屏、逐帧、截图和 A–B 循环。
- **整理媒体库**：添加文件或整个目录，搜索音乐和视频，按歌手、专辑浏览音乐。移出媒体库不会删除原文件。
- **建歌单、收藏歌曲**：支持多选、批量播放和“我喜欢”。音乐与视频各有自己的播放列表，可顺序播放、循环或随机播放。
- **看歌词**：自动读取同名 LRC 或内嵌歌词，也能手动导入 LRC / TXT，调整字体、行距和双语显示。
- **调整声音**：选择输出设备、设置均衡器，支持共享和独占输出。默认音量为 70%，不开额外音效。
- **调整界面**：浅色 / 深色、主题色、背景图片、字体、快捷键和减少动画。
- **联网搜歌**：配置兼容的 API 后，可以搜索、播放和下载音乐。歌曲和歌词的联网功能默认关闭，在“设置 → 联网服务”中开启。

目前不保证 HDR 显示输出、位准确或无缝播放，也未提供 ASIO / 原生 DSD 输出。独占输出不等于位准确输出。

## 怎么用

Windows 安装包的文件名以 `-setup.exe` 结尾，运行后按提示安装；已有旧版时会识别并升级。便携版需要完整解压 ZIP，再运行其中的 `yyplayer.exe`。

启动后：

1. 在“音乐库”或“视频库”里添加目录，也可以用“打开文件”直接播放。
2. 点击左下角封面打开歌词页；点击右下角播放列表按钮查看和调整队列。
3. 在“播放选项”里调整输出设备、均衡器、字幕和解码设置；在“设置”里调整外观、字体与快捷键。

默认快捷键：空格播放 / 暂停，左右方向键短按前后跳转 5 秒、长按临时 3 倍速，上下方向键调音量，`F` 切换全屏，`Ctrl+O` 打开文件。

歌单会保存，播放列表是本次使用的临时队列，重启后为空。视频返回库后，可以在本次使用期间继续观看。

Windows 设置保存在 `%APPDATA%\YYPlayer`，便携版也使用这里。音乐库和歌单保存在该目录下的 SQLite 数据库中，无需另外安装数据库。升级会保留这些数据；备份时先退出播放器，再备份整个设置目录。详细操作见[歌单指南](docs/MUSIC_PLAYLISTS.md)、[歌词指南](docs/LYRICS.md)和[联网搜索指南](docs/ONLINE_API.md)。

## 从源码运行

Windows 和 Linux 共用一份源码，开发分支是 `dev`。

### Windows

需要 Rust MSVC、Visual Studio C++ Build Tools 和 Windows SDK。在仓库根目录执行：

```powershell
powershell -ExecutionPolicy Bypass -File scripts/Get-Mpv.ps1
cargo run --locked --release -p yyplayer-app --bin yyplayer
```

首次获取播放内核需要联网，解压需要 7-Zip 或支持 7z 的 Windows `tar`。想自己打包时，准备好上述环境和 Git，双击根目录的 **`Package-Windows.cmd`**；它会编译当前代码，生成 ZIP 和安装包，并打开输出目录。更多选项见[打包说明](docs/PUBLISHING.md)。

### Ubuntu 26.04

先准备 Rust 和系统依赖。下面的检查脚本会列出缺少的依赖及安装命令：

```bash
python3 scripts/check-linux-deps.py
cargo run --locked --release -p yyplayer-app --bin yyplayer
```

Linux 安装、音频输出及 deb / AppImage 打包方法见[Linux 指南](docs/LINUX.md)。

## 截图

<details>
<summary>展开查看音乐库、歌词页和视频播放</summary>

![音乐库](images/musiclib.png)
![歌词页](images/musicdetail.png)
![视频播放](images/videoplay.png)

</details>

## 更多资料

[更新日志](CHANGELOG.md) · [参与开发](CONTRIBUTING.md) · [开发规则](AGENTS.md) · [开发规划](docs/DEVELOPMENT_PLAN.md) · [测试记录与待办](docs/STATUS.md) · [平台兼容性记录](docs/validation/platform-minimums.md)

项目源码和原创 UI 资源采用 **GPL-3.0-only**，见 [LICENSE](LICENSE)。第三方组件的许可见 [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md)。
