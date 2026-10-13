# 参与 YYPlayer 开发

当前开发版本为 **0.5.4**。所有源码、界面、脚本和文档修改都从 **dev** 开始；master 只合入已验证的 dev 改动，用于集成与发布。修改前运行 `git branch --show-current`。提交、合入 master、推送和发布分别按用户授权执行，不强制推送、不重写历史。

先看 [README](README.md) 了解使用方法，再看 [AGENTS](AGENTS.md) 的开发约束和 [STATUS](docs/STATUS.md) 的实际进度。架构背景与未来任务见[开发规划](docs/DEVELOPMENT_PLAN.md)和[架构决策](docs/adr/)。规划、布局样例和编译成功都不能当作功能验收记录。

## 准备环境

Windows x64 MSVC 和 Linux amd64 GNU 使用同一 Cargo workspace 与 Cargo.lock，平台 API 和依赖通过 cfg 隔离。当前代码固定 Slint / slint-build 1.17.1；Windows 和 Ubuntu 已有 Rust / Cargo 1.99.0 的构建记录，Cargo 声明的最低 Rust 1.92 尚未专项验证。macOS 尚未编译或真机验收；0.5.2–0.5.4 的最新改动还需 Linux 回归。

Windows 需要 Rust MSVC、Visual Studio C++ Build Tools、Windows SDK 和 Git。先获取固定 libmpv，再运行程序：

```powershell
powershell -ExecutionPolicy Bypass -File scripts/Get-Mpv.ps1
cargo run --locked -p yyplayer-app --bin yyplayer
```

Linux 依赖和输出策略见 [Linux 指南](docs/LINUX.md)：

```bash
python3 scripts/check-linux-deps.py
cargo run --locked --release -p yyplayer-app --bin yyplayer
```

Windows 运行时按 manifest / SHA-256 校验；Linux 开发和 deb 使用系统 libmpv，AppImage 校验包内运行时。音乐数据库已静态内置 SQLite，无需外部数据库服务。

## 代码放在哪里

| 目录 | 职责 |
| --- | --- |
| `crates/player-core` | 播放状态、命令、队列、歌单、设置和路径模型；不依赖 UI、mpv 或 OS API |
| `crates/player-mpv` | 动态加载、FFI、Engine worker、音频确认与视频加载等待 |
| `crates/player-platform` | 各平台设备、字体、外观、目录和窗口表面适配 |
| `crates/player-ui` | Slint 页面与控件、主题、view model、窗口操作和 GL presenter |
| `crates/yyplayer-app` | AppController、启动流程和后台服务；`music_store.rs` 管理 SQLite 音乐库 |
| `scripts` / `packaging` | 运行时获取、回归脚本、Windows 与 Linux 打包 |
| `docs` | 使用指南、规划、架构决策和验收证据 |

Slint 组件只在主线程修改，普通 mpv 命令只由 Engine worker 执行；UI 回调提交命令，不能等待 Engine。GL notifier 只调用允许的 render API，保留 `prepare_gl`、同上下文调用和先释放 renderer 后释放 core 的顺序。扫描、文件读写、数据库和字体枚举留在有界后台服务。完整要求见 [AGENTS](AGENTS.md)。

页面文字按钮使用 PageButton，下拉框使用 ThemeComboBox，播放选项与播放列表复用 SideDrawer。主题和减少动画设置应统一生效，抽屉覆盖内容，不能挤压视频或歌词页面。原生窗口状态按事件查询，不能恢复每 60ms 查询最大化的轮询。

## 检查改动

功能修改先明确目标和验收场景，完成以下检查及对应场景回归：

```powershell
cargo fmt --all -- --check
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo test --locked --workspace --all-targets
cargo build --locked --release -p yyplayer-app --bin yyplayer
```

依赖已缓存时，Cargo 命令可加 `--offline`。按改动选择回归范围：

| 改动 | 主要回归入口 |
| --- | --- |
| 按钮、下拉框、抽屉、导航 | `ui-consistency`、`navigation-ui`、`ui-feedback` 开发例子，`Test-UiCustomization.ps1` |
| 音乐库、歌单、SQLite | `Test-Collections.ps1`、迁移 / 损坏 / 重试单测及重启恢复 |
| 视频加载、渲染或生命周期 | `Test-VideoLibrary.ps1`、`Test-Video.ps1`、`Test-Render.ps1` 的真实 GPU 图像比较 |
| 音频设备和 EQ | `Test-Audio.ps1`、`Test-AudioBusy.ps1`，对应设备实测 |
| Windows 打包或安装器 | `Test-WindowsPackaging.ps1`、`Test-WindowsPackagingLauncher.ps1`、`Test-WindowsInstaller.ps1` |
| Linux 适配或包 | `test-linux.py`、`test-linux-packages.py` 及所改打包模块的测试 |

脚本参数与验收条件见各文件及 [STATUS](docs/STATUS.md) 链接的报告。GUI、GPU 和音频设备测试串行执行，使用独立配置与自有素材。`YYPLAYER_CONFIG` 指向测试设置时，数据库也跟随该路径；不要改默认 APPDATA / XDG 配置、系统主题或用户媒体。

把改动、实际检查结果、未验证条件和下一步写入 STATUS。仅文档修改检查事实、命令和链接即可，不需要为了文字改动重编译或运行硬件测试。README 面向使用者，保持简短；版本历史写 CHANGELOG，技术证据写验收报告。

## 打包和提交

Windows 本地打包可双击 `Package-Windows.cmd`，或使用 `scripts/Package-Windows.ps1`。脚本自动进行 locked x64 MSVC Release 编译，使用本次 Cargo 返回的 EXE；NSIS 固定 3.12。完整参数、升级行为与发行限制见[发布说明](docs/PUBLISHING.md)。

提交前检查 `git status --short` 和暂存 diff。保留 Cargo.lock、运行时 manifest、原创资源和验收文档；不提交 DLL / so、下载归档、target、发行包、用户设置、数据库、媒体或凭据。自定义 Cargo 产物目录放在 target 子目录或仓库外。

Windows 缓存清理先运行 `scripts/Clean-Workspace.ps1 -WhatIf` 核对范围；保存证据后再清理。不要为了删除锁定文件而终止用户播放器。这不是 Linux 构建前置步骤。

项目源码与原创 UI 资源采用 GPL-3.0-only，贡献使用相同许可；第三方代码保留原许可。PR 描述说明具体问题、修改后的行为和实际验证结果。
