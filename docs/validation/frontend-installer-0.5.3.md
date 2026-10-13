# 0.5.3 页面按钮、窗口查询与 Windows 升级安装

日期：2026-10-13。开发分支 dev，基于 5618134 和工作区已有的 0.5.2 SQLite 改动；未提交、合入 master 或推送。本轮自有测试素材与独立配置，不升级本机现有 YYPlayer、不修改用户 APPDATA 或系统缩放。

## 实现

- 所有页面文字 Button 不再导入 std-widgets 的系统按钮。PageButton 为统一实现，ThemeButton / ActionButton 复用它；音乐 / 视频库、搜索、最近打开、设置、歌词、播放选项、歌单弹窗和队列均使用主题色、同一字体 / 状态 / 焦点行为。时尚模式使用半高度胶囊，简洁模式使用 8px 圆角。专用图标播放键、导航项和窗口控制保留相应语义。
- bootstrap 保留业务 tick，但原生最大化 / 全屏查询由 resize / move / scale / focus 和明确模式请求触发，合并事件，去重相同尺寸 / 位置，并屏蔽查询自身同步产生的通知。查询前释放 controller 借用。窗口标题、controller 模式和 DWM 形状共用结果，诊断中的最大化也使用缓存。普通运行不再采集测试 checkpoints；只在请求视频 renderer 且尚未 ready 时重试 redraw。
- 锁定 Winit 0.30.13 的 `platform_impl/macos/window_delegate.rs::is_zoomed` 明确为无边框窗口临时切换 styleMask、查询 isZoomed 后恢复。该源码事实解释了高频查询的风险，不能代替 MacBook Air CPU 实测。
- Setup 读取既有 64 位 HKLM 卸载键 `{A858C6E1-6B63-4A1B-90EC-275C31F61D38}` 的 DisplayVersion / InstallLocation；升级固定到原目录，欢迎页和按钮显示升级。同版本重装、阻止降级。新文件先提取到插件临时目录，再逐个备份 / 替换；复制失败尝试回滚，并保持旧注册表版本。不会执行旧卸载器或删除配置 / SQLite。
- 打包根据本次 portable stage 生成并编译程序文件清单。卸载只删除清单内文件、空目录和固定快捷方式，不再递归删除安装根目录。旧包内已经过时的非冲突文件可能保留；本轮 0.5.1 → 0.5.3 布局保持兼容。
- 便携 NSIS 原本已经锁定 3.12。本轮要求自选编译器至少 3.12，Windows CI 也改为获取固定 hash 的 3.12；Setup / 卸载器明确 `ManifestDPIAware true`，解决缺声明造成的系统位图缩放模糊。它是系统 DPI 感知，不宣称多屏 Per-Monitor V2。官方依据：[ManifestDPIAware](https://nsis.sourceforge.io/Docs/Chapter4.html#4.8.1.29)、[NSIS 3.12 更新记录](https://nsis.sourceforge.io/Docs/AppendixF.html)。

## 已执行证据

- `cargo fmt --all -- --check`、`git diff --check`：通过。`cargo clippy --locked --offline --workspace --all-targets --config profile.dev.debug=0 --config profile.dev.incremental=false -- -D warnings`：最终通过。
- `cargo test --locked --offline --workspace --config profile.dev.debug=0 --config profile.dev.incremental=false`：73 项通过，0 失败；1 项已有的 HTTPS localhost QA CA 测试明确 ignored。本轮新增 WindowState 测试覆盖 1000 次空闲 tick、事件合并、缺失窗口重试、重复几何和查询产生的同步通知。
- `scripts/Test-WindowsPackaging.ps1 -NsisCompiler target/windows-packaging-tools/nsis-3.12/makensis.exe`：PowerShell 7 / Windows PowerShell 5.1 回归通过；最终 5.1 的 8 项还覆盖旧 NSIS 3.11 在编译前拒绝。固定 runtime hash、编译失败 / 旧 EXE、外部调用目录 / 自选 target、ZIP EXE hash / 0.5.3 版本、NSIS 失败 / 重试、已有包保护；保存的 [摘要](frontend-installer-0.5.3/packaging.json)。
- `scripts/Test-WindowsPackagingLauncher.ps1`：10 项通过，包含 PowerShell 5.1、固定归档校验 / 全新提取、独立目录 / 离线和失败边界；[摘要](frontend-installer-0.5.3/launcher.json)。
- `scripts/Test-WindowsInstaller.ps1`：7 项真实 NSIS 隔离测试通过。读取生成的 Setup 与卸载器 PE 验证 DPI 声明、首次安装、锁定 DLL 时恢复已替换 EXE / 旧版本、0.5.1 → 0.5.3 沿用原目录（即使 /D 指向其他目录）、同版重装、拒绝降级、卸载保留额外文件及独立数据。使用编译期私有 HKCU ID、私有快捷方式目录和 target 内 fixture，不触碰真实安装；[摘要](frontend-installer-0.5.3/installer.json)。只读检查本机真实卸载键为 0.5.1、`C:\Program Files\YYPlayer`，与生产检测逻辑匹配。
- `ui-feedback`、`navigation-ui`、`audio-ui`：全部通过。真实 Slint 点击、状态提示时序、最小 / 常规 / 宽窗口、深浅 / 简洁主题、播放选项 / 导航透明度、歌词展开和返回；[设置按钮截图](frontend-installer-0.5.3/settings-buttons.png)。
- `scripts/Test-UiCustomization.ps1 -SkipBuild`：字体、列宽拖动 / 保存、字幕、窗口最大化 / 还原 / 圆角与影子、歌词和滚动通过；[摘要](frontend-installer-0.5.3/ui-customization.json)。
- `scripts/Test-Collections.ps1 -SkipBuild`：实际歌曲 / 分类 / 歌单 / 队列、播放顺序、抽屉点击、SQLite 重启持久化、3 种尺寸和 GPU 生命周期通过；[摘要](frontend-installer-0.5.3/collections.json)、[页面按钮截图](frontend-installer-0.5.3/music-buttons.png)。
- `scripts/Test-VideoLibrary.ps1 -SkipBuild`：实际行 / 导航点击、音频懒渲染、释放 / 恢复、10 循环、快速取消、EOF 后暂停 / 继续通过。
- `scripts/Test-Render.ps1 -SkipBuild`：自有静态 4K HEVC，桌面 GL、`d3d11va-copy` / 软件解码 / 去色带 GPU 图片对照通过；146 / 142 / 149 帧。素材 SHA-256 `3cbbf6d3a5d62541a5574528b224802acbd586338625c4dcf889193ced65ecd2`；[摘要](frontend-installer-0.5.3/render-summary.json)。
- `scripts/Test-Video.ps1 -SkipBuild -EvidenceDirectory target/video-0.5.3`：实际生产 bootstrap / controller 的播放控制、最大化 / 全屏、暂停重载解码、保存、软解和正常退出通过，固定 runtime 为 `mpv v0.41.0-1087-ge470f8986`。

- 完整 Release：`scripts/Package-Windows-Interactive.ps1 -Offline -NoOpen` 最终构建 / 打包通过，本次归档重新核验后提取 NSIS 3.12。产物位于 `dist/windows-20261013-091954-d7f2bc7c/`，ZIP 62,544,812 字节，Setup 60,922,118 字节。EXE 36,114,432 字节，版本 0.5.3，SHA-256 `af34daacb5b9aba6997b23ee5c2ee81a32638c1140586d65890066fa630e8c9b`；ZIP 内 EXE 与本次 Cargo Release 完全一致，BUILD-INFO 记录工作区 modified=True 和 NSIS v3.12。最终生产 Setup 的 PE DPI 声明再次核验通过；包完整 hash 见 [摘要](frontend-installer-0.5.3/release-package.json)。
- 最终 ZIP 解压后独立启动：`scripts/Test-WindowIdle.ps1 -Player <包内 yyplayer.exe>` 8 秒仅查询原生窗口状态 **1 次**，音乐库数据库初始化 ready、0 次 libmpv renderer、正常退出。3 秒 CPU 采样读数为 0ms，受 Windows CPU 时间粒度限制，不据此宣称零 CPU，亦不推断 MacBook Air 表现；[证据](frontend-installer-0.5.3/window-idle.json)。
- 包内实际 WASAPI 共享播放 / 正常退出通过：4 秒自有 WAV 到 EOF 为 Ended、位置 3.999773；其后使用 15 秒自有 WAV，5 秒检查为 Playing、位置 4.477048，无错误、0 次视频 renderer、数据库 ready。第一次检查曾把短素材误当作应持续 Playing，核对实际时长后补长素材复验。未修改用户默认 APPDATA。
- 最后窄按钮审查补充：28px EQ 删除键明确使用 4px 内容内边距，普通按钮维持 14px；避免文字被内边距挤空。尝试按实际宽度计算 padding 时 Slint 检出布局绑定循环，已改为明确参数并通过编译。最终 `navigation-ui` / `audio-ui`、全部目标 Clippy、fmt 与 diff 检查再次通过，完整包包括此修正。
- 本机环境：[记录](frontend-installer-0.5.3/environment.json)。Windows 11 Pro build 26300、Ryzen AI 7 H 350（16 逻辑处理器）、Radeon 860M / 驱动 32.0.31041.1004 和 RTX 5060 Laptop / 驱动 32.0.16.1088，Rust 1.99.0、Slint 1.17.1。列出设备不代表每块 GPU 都完成独立矩阵。

## 限制与下一步

MacBook Air / macOS arm64 尚无本机环境，本轮消除应用高频查询与额外重绘路径，不报告 macOS 编译或 CPU 资格。Linux 受影响共用 UI / bootstrap 待原生回归。125% / 150% / 200% 实体安装界面与跨屏 DPI、管理员权限下真实旧安装升级、干净 Windows 安装 / 卸载继续待验。安装恢复针对可检测的复制失败，不是断电恢复事务；恢复文件仍被其他程序锁定时会记录失败，提示关闭程序重试。
