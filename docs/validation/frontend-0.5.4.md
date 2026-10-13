# 0.5.4 视频库、最近打开与控件统一

日期：2026-10-13。在 dev 保留既有 0.5.2 / 0.5.3 工作区改动，未提交、合入 master 或推送。GUI / GPU 验收串行，使用自有 fixture、target 独立配置，不修改用户 APPDATA、真实安装、媒体或系统偏好。

## 实现

- 视频库目录选择、重扫、移除目录和数量移到添加目录 / 添加视频之后，保持同一行；最小 1000×640 窗口也可容纳。继续观看位于批量操作行右侧，列表少占一行工具栏。
- 最近打开 QueueRow 标题 11→15px，路径 / 时长 9→12px，封面 42→48px、行高 68→76px；长标题继续省略，整行点击仍由原回调打开。
- SideDrawer 为播放选项 / 播放列表共用的覆盖组件：同一可逆横向平移、Theme.smooth、12px 外边距、18px 圆角、阴影和遮罩。选项卡保留 370px、队列卡 430px 宽度；内容显式使用卡片尺寸。抽屉不参与主布局，不改变库、歌词和视频宽度；遮罩点击仅关闭抽屉，减少动画时直接到目标位置。卡片在歌词封面之上。
- 所有 ComboBox 统一为 ThemeComboBox，包含音乐 / 视频目录、歌单对话框、联网来源、设计 / 明暗 / 主题色、背景、三类字体、视频窗口模式及已有音频 / EQ / 倍速 / 队列选择栏。受控选择使用 selected(value, choice) 的新 choice，避免读旧 current-index；浏览页 / 窗口模式焦点阻挡播放快捷键，换页清除旧焦点。
- 根 Cargo 版本、五个 workspace crate、Cargo.lock、EXE 资源和包版本为 0.5.4；第三方锁定版本、SQLite schema、设置版本和 runtime 保持原策略。NSIS 3.12 与原安装身份沿用 0.5.3 实现。

## 已执行回归

- ui-consistency 在 Winit / FemtoVG 和 Winit / software 各完成 18 阶段：导入按钮同高的目录选择、鼠标 / 方向键新值、快捷键焦点、1000px 最小宽度、最近打开点击、两种抽屉实际动画和几何、遮罩不穿透、隐藏选项不接收方向键、全屏视频尺寸、减少动画与浅色 / 简洁主题。GPU 弹层快照可能为黑色，交互断言仍通过；另跑软件视觉验收，保留 [GPU 摘要](frontend-0.5.4/ui-consistency-gpu.json)、[软件摘要](frontend-0.5.4/ui-consistency-software.json)和 [目录弹层](frontend-0.5.4/video-dropdown.png)。软件快照不代替 GPU 播放资格。
- navigation-ui 11 阶段、ui-feedback 17 阶段与 audio-ui 通过；导航检查抽屉打开时内容保持完整宽度，ui-feedback 新增选项覆盖全屏时视频尺寸不变。
- Test-UiCustomization -SkipBuild 通过：真实字体下拉、列宽拖动 / 保存、字幕、原生最大化 / 还原、滚动和歌词。
- Test-Collections -SkipBuild 29 阶段、SQLite 重启持久化通过；补最小窗口背景点击关闭选项而不改变分类，再次点击分类生效。实际队列、播放顺序与视频释放保持正常。
- Test-VideoLibrary -SkipBuild 通过：新的行坐标真实点击、目录筛选、音频懒渲染、十次释放 / 重建、恢复位置、取消请求、EOF 后暂停 / 继续与最小布局。
- Test-Render -SkipBuild 通过：自有静态 4K HEVC，硬解 / 软解 / 去色带真实 GPU 图片对照，145 / 151 / 154 帧；实际 d3d11va-copy，素材 SHA-256 `3cbbf6d3a5d62541a5574528b224802acbd586338625c4dcf889193ced65ecd2`。[摘要](frontend-0.5.4/render-summary.json)
- Test-Video -SkipBuild -EvidenceDirectory target/video-0.5.4 通过：生产 bootstrap / controller 控件、最大化 / 全屏、暂停重载、保存、软件解码和正常退出。[播放证据](frontend-0.5.4/video-controls.json)
- Windows PowerShell 5.1 Test-WindowsPackaging 8 项、Test-WindowsPackagingLauncher 10 项、Test-WindowsInstaller 8 项通过。真实 NSIS 代码在随机私有 HKCU / 私有快捷方式 / target fixture 测试，覆盖 0.5.3→0.5.4、0.5.1→0.5.4、保留原目录 / 用户文件、锁 DLL 失败回滚、同版重装、阻止降级、卸载保留用户数据及 Setup / 卸载器 DPI 声明。[安装摘要](frontend-0.5.4/installer.json)

## 最终构建与包

- `cargo fmt --all -- --check`、`git diff --check` 通过；最终 `cargo clippy --locked --offline --workspace --all-targets --config profile.dev.debug=0 --config profile.dev.incremental=false -- -D warnings` 通过。[日志](frontend-0.5.4/clippy.log)
- `cargo test --locked --offline --workspace --config profile.dev.debug=0 --config profile.dev.incremental=false`：73 项通过、0 失败、1 项既有 HTTPS localhost QA CA 测试 ignored。[日志](frontend-0.5.4/tests.log) `cargo metadata --locked --offline --no-deps` 确认五个 crate 均为 0.5.4。[版本](frontend-0.5.4/versions.json)
- `scripts/Package-Windows-Interactive.ps1 -Offline -NoOpen` 完成 locked x64 MSVC Release 与完整 ZIP / Setup。本次新目录 `dist/windows-20261013-100632-039a3f1b/`，ZIP 62,465,931 字节、Setup 60,851,035 字节；归档重新核验后提取 NSIS 3.12。包内 EXE 版本 0.5.4、35,876,864 字节，SHA-256 `0e9a0d2b128d7a1688f6f0cf61e311c2b63a0ab21f81ac979213a938bb15f637`，与本次 Cargo compiler-artifact 相同；BUILD-INFO / 固定 runtime hash / 生产 Setup DPI 声明核验通过。包记录工作区 modified=True。[包与 hash](frontend-0.5.4/release-package.json)、[打包日志](frontend-0.5.4/package.log)
- ZIP 解压后的 EXE、独立配置 8 秒空闲启动 / 正常退出通过：原生窗口查询 1 次、libmpv renderer 初始化 0 次、音乐数据库 ready。3 秒 CPU 时间采样为 0ms，受 Windows 时间粒度限制，不代表零 CPU 或 MacBook Air 资格。[证据](frontend-0.5.4/window-idle.json)
- 相同包内 EXE 使用 15 秒自有 WAV，5 秒检查为 Playing，位置 4.383749，无播放 / render 错误、数据库 ready、0 次视频 renderer、正常退出。[证据](frontend-0.5.4/release-audio.json)

## 适用范围与下一步

本机环境见 [记录](frontend-0.5.4/environment.json)。Windows UI 与现有 GPU 组合通过；macOS / Linux 本轮编译和真机未执行。实体键鼠 / 跨屏 DPI、真实管理员旧版本升级、USB DAC / 位准确 / HDR 及发行许可闭包沿用此前待验范围。下一步在 Linux / macOS 验证同一源码的下拉焦点、抽屉、视频尺寸与退出；Windows 补真实管理员升级和多 DPI 显示，不能把隔离 fixture 视为真实安装资格。
