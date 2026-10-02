# YYPlayer 进度与证据

更新：2026-10-02。正式源码分支已统一为 master，dev 保留最新开发版本，原 main 的全部提交已合入 master，重复分支已移除。当前交付为 **Windows 音视频播放开发预览**，原始框架于 2026-09-30 完成。Git 框架 `d10a4e8` 保留在历史，视频 `4eca722` 保留在 master 历史中；音频 `8467ca9` 已合入 master，目录音乐库 / 主题 / UI 修复 / 字体 / 窗口 / 列宽与发布准备均已合入 master。

## 2026-10-02：视频重播后无法暂停

状态：Done（本轮播放状态修复 / 本机回归）。继续 dev，未提交 / 合并 / 推送。旧 Release 自有 2 秒视频复现：EOF 后重播位置前进，但 snapshot 仍 Ended，每次点击均 Seek(0)+Resume，UI 仍播放图标。worker 在内核 active / 非 EOF 时恢复 Playing / Paused，Idle / Loading / Error 不被旧属性复活；保持 Engine / GL 边界。

- 改动 worker.rs 的真实状态投影及两个边界单测、video-library.rs 的实际 Slint 按钮回归、Test-VideoLibrary 提示、CHANGELOG / 接续规则。没有修改 GL / 解码默认 / 音频策略或添加 reload。
- fmt --check、严格 workspace all-targets clippy、workspace all-targets tests（28 个独立单测）、Debug yyplayer / video-library 和 Release 构建通过。Test-VideoLibrary 21 阶段及 10 个暂停 / 重播子步骤通过：普通和 EOF 重播后的暂停位置冻结、继续递增，generation / renderer 创建数保持不变；Test-Video 11 动作含 AAC 音轨、NVDEC、暂停及软件解码重载位置恢复通过。
- 最终 Release 同一 2 秒短视频复验通过：3 秒 Ended，4 秒 Playing，5 秒 Paused；引擎 / 渲染错误为空。自有媒体、独立配置；用户视频仅读取编码 / 时长，没有截图 / 复制或修改 APPDATA。原因、最小轨迹和截图见 [验收记录](validation/video-pause-replay.md)。未新增物理键鼠 / 多屏 / 4K 图片 / HDR / USB 资格。
- 保存证据后运行 Clean-Workspace -WhatIf 核对工作区目标与链接，再清理 12,408,954,418 字节（11.56 GiB），target 仅保留 22,159,360 字节新 Release exe，固定 runtime / 头文件保留。清理后独立配置启动 / 正常退出通过，Idle、render 帧 / 创建 0、错误为空；exe hash 未变、DLL hash 符合 manifest，没有终止用户播放器。
- 下一步：用户运行新 Release 确认实际媒体体验；后续回归保留普通暂停与 EOF 重播后暂停 / 继续场景，继续既有生命周期 / 多设备资格验证。

## 2026-10-02：Windows x64 Actions 构建包

状态：实现完成，等待用户推送到 GitHub 验证真实 runner。当前在 dev，未提交或推送；触发条件是远端 master 收到 push。

- 新增 `.github/workflows/windows-release.yml`、`scripts/Package-Windows.ps1`、`packaging/windows/yyplayer.nsi`。Windows 2022 x64 MSVC 稳定 Rust workflow 下载并校验固定 libmpv，编译 release exe，创建带 runtime DLL / 数据文件、许可记录、manifest / commit 源码链接的 portable ZIP 和 x64 Windows 专用 NSIS 安装器，两文件存为同一 30 天 Actions artifact；也可手动 dispatch。
- 选择 NSIS 而非 Inno Setup：NSIS 上游 zlib/libpng 等许可可以商用 / 非商用，runner 清单含 NSIS / 7-Zip，无需将 CI 构建锁定到商业安装器许可。NSIS setup 按架构检查，只装 64 位程序至 Program Files，快捷方式 / 注册卸载清理。项目设置保持原有 per-user `%APPDATA%`。
- `scripts/Get-Mpv.ps1` hash 验证之后优先用 7-Zip 解压经验证的 `.7z`，没有 7-Zip 才退到 tar；工作流不上传 DLL 到 Git 仓库或使用用户机 PATH。
- 输出是逐提交 Actions artifact，不创建 GitHub Releases 页；需 wait for workflow after master push。完整 libmpv 内部组件 notices / 对应源码安排 / DLL 闭包发行审计仍未完成，所以没有宣称正式二进制发行验收。没有在本机或 GitHub runner 执行 workflow；NSIS 安装/卸载、干净 Windows 首次运行、签名与真实 Actions 上传待后续 runner / 真机验收。
- 下一步：源码 push 至 GitHub master 后，从 Actions 下载两份资产并在干净 Windows 验证安装 / 卸载、便携目录启动与运行时加载；随后再补发行许可闭包与签名决策。

## 2026-10-02：修复 NSIS runner 许可路径假设

状态：已修正，等待将 dev 更新推送到 GitHub 后重跑 Actions。首次 hosted runner 在 `Package-Windows.ps1` 第 18 行失败，因为脚本假定 NSIS 安装器一定在编译器旁安装 `Docs/AppendixI.html`。

- 移除对 runner 私有 NSIS 文档目录的硬依赖；改为从仓库打包固定的 NSIS attribution 文件，含官方许可链接和本项目安装器源码位置。NSIS 版本仍从 `makensis.exe` 文件元数据读取并写入 `BUILD-INFO.txt`。
- 改动 `scripts/Package-Windows.ps1` 与新增 `packaging/windows/NSIS-BUILD-TOOL.md`。尚未在本地复跑完整打包，也未重跑 GitHub Actions；下次 workflow 会验证 hosted runner 路径，并继续检查其余 packaging 步骤。

## 2026-10-02：外观设置控件高度与缓存清理

状态：Done（本轮 UI / 本机布局及缓存清理）。继续 dev 原工作区，未提交 / 合并 / 推送。appearance-options.slint 将用户框出的设计 / 明暗 / 主题色来源下拉、颜色输入及应用按钮固定 34px；标签行 60px，不再按面板剩余高度拉伸。

- fmt --check、严格 workspace all-targets clippy、Debug yyplayer / library-theme / navigation-ui、Release 构建通过。Test-LibraryTheme 13 阶段、navigation-ui 10 点通过，已查看 1240×900 / 1460×920 设置截图；文字 / 控件高度正常，无回调或主题策略变更。低风险布局未新增单测，未重跑解码 / 位准确 / 内存专项；物理键鼠 / 多屏 DPI 尚未新增资格。
- 保存自有截图 / 证据到 docs，并归档此前内存原始样本与测量脚本。Clean-Workspace -WhatIf 核对工作区绝对目标 / 无链接，再清理，移除 18,717,681,722 字节（17.43 GiB）；target 仅留 22,158,848 字节 Release exe，固定 DLL / headers 保留，未处理 APPDATA / 用户媒体或终止用户播放器。
- 清理后独立配置 Release 启动 / 正常退出再验通过，Idle、mpv v0.41.0-1087-ge470f8986、render 帧 / 创建 0，无引擎 / 渲染错误；exe 清理前后 hash 一致，DLL 与 manifest 一致。预览 / 清理输出 / 最小诊断及图片见 [验收记录](validation/appearance-controls-cleanup.md)。git diff --check 与证据链接检查通过。
- 下一步：用户体验紧凑外观控件；后续构建会完整重编，继续此前大库 / 多屏 / USB 资格验证。

## 2026-10-02：精简媒体库界面

状态：Done（本轮 UI / 本机布局与流程验收）。按用户框选截图删除音乐宣传卡片、音乐 / 视频库重复标题和索引说明，数量并入筛选工具栏；侧栏、主标题与空闲播放栏统一命名“视频库”。保留未框出的空库引导和库操作。继续在 dev 原工作区，未提交 / 合并 / 推送。

- 改动 music.slint / video-library.slint / app.slint / shell.rs 的布局与可见属性、controller.rs 空闲标题；更新 ui-customization / video-library 新行位置点击坐标、CHANGELOG 与接续规则。没有改 Engine / presenter / GL / 解码或音频默认。
- fmt --check、严格 workspace all-targets clippy、workspace all-targets tests（26 个独立测试）、Debug bins / examples、Release 与 git diff --check 通过；Test-UiCustomization 22 阶段（新列宽拖动 / 移除）、Test-LibraryTheme 13、Test-VideoLibrary 20、navigation-ui 10、ui-feedback 17、audio-ui 五图 / 实际点击通过。
- 已审查浅 / 深音乐库、视频空页 / 列表和 1000×640 选项面板截图，按钮 / 数量无重叠、列表上移、删除内容无空白占位。使用自有素材 / 独立配置；报告与图片见 [媒体库 UI 精简验收](validation/media-library-ui.md)。未新增物理键鼠 / 对话框 / 多屏 DPI 资格，没有重复 4K 像素 / 位准确 / 内存专项。
- 下一步：在用户真实媒体库下体验精简布局，继续补此前大库 / 长时资源 / 多屏 / USB 验收；库与播放资源生命周期保持上一轮实现。

## 2026-10-02：视频库与按需视频渲染

状态：Done（本轮 Windows 开发预览功能 / 本机验收）。用户要求在 dev 修改；dev 原落后 master 一个文档提交，先快进至 1406840，本轮实现留在 dev 工作区，未提交、未合入 master、未推送。

- 视频剧场改为独立库页，目录 / 单文件导入、递归去重、搜索、筛选、重扫 / 取消、移除和重启恢复；video_library 配置 / videos.json 索引与音乐分开，有界后台规范化 / 扫描，移除不删原文件、不改变已建立队列。只索引名称 / 目录 / 文件大小，不为库扫描解码或虚构时长。
- 同窗口 VideoPlayer / VideoPlayerToolbar 抽为独立组件；进入播放才创建 libmpv render context，音频不等待 render。返回库保存本次位置后 Stop；current-vo 实际关闭后释放 render / FBO / texture，继续观看重建并恢复。修正无音轨视频仅关 vid 会 Ended、旧 VO 标志残留、过期错误取消新加载；VideoGate 合并 / 取消旧等待请求。64 项普通命令 + 1 个 Stop 保留槽，不丢弃排队设置，Shutdown atomic；主线程 / Engine / GL 与 prepare_gl 边界保留。
- 涉及 core 配置 / 协议 / 快照、mpv worker / video_gate、app library_service / video_controller / controller / bootstrap、UI 库页 / 播放器组件 / presenter / 投影；新增 Test-VideoLibrary / 真实流程例子，更新导航与歌词点击测试坐标。更新 README / CHANGELOG / ADR 0006 / 接续规则，无新依赖。
- fmt --check、严格 workspace all-targets clippy、workspace all-targets tests（26 个独立测试）、Debug bins / examples 与 Release 构建通过。视频库 20 阶段含 10 次释放 / 重建、快速取消、损坏视频后音频恢复、索引恢复 / 最小布局通过。Test-Video 11 动作、Test-Render 4K HEVC NVDEC / 软件 / 去色带实际 GPU 图片比较通过，RGB 误差 0 / 0.205388、明显偏差比例均 0；Test-UiCustomization 22 阶段、navigation-ui 10、ui-feedback 17、audio-ui 五图 / 实际点击、Test-LibraryTheme 13、Release Test-Audio 通过。
- 同素材 / 默认设置 / 独立配置的串行 Release 短样本：音乐 Working Set 中位数 227.65 → 110.46 MiB，Private Bytes 384.91 → 163.40 MiB；新版 render 帧 / 创建数均 0。这是自有静音 WAV 的观察，不外推为用户 4K 素材 / PotPlayer 对比或 GPU 专用显存。
- 详细命令、硬件 / 驱动 / 素材 hash、生命周期 / 内存 / GPU 指标与图片见 [视频库验收](validation/video-library.md)，设计见 [ADR 0006](adr/0006-video-library-and-lazy-rendering.md)。使用自有素材 / 独立配置，未读取用户锦城湖视频或改 APPDATA。
- 已知边界：当前运行会话续播，未跨重启保存位置；未做视频缩略图 / 时长索引 / 实时监听 / DB。物理文件对话框 / 长时大库 / 网络盘 / 睡眠 / 多屏 DPI / 其他驱动与跨平台仍待验；USB / HDR / 位准确资格未改变。下一步以自有大库和长时间循环补内存 / 响应 / 退出证据，随后补原音频计划的 USB / 数字捕获。

## 2026-10-02：精简项目首页与整理更新日志

状态：Done（文档整理）。目标为 README 只保留功能、编译、运行与必要入口，产品更新集中到 CHANGELOG；保留完整规划供后续开发。

- 改动 README / CHANGELOG / docs/DEVELOPMENT_PLAN / AGENTS / CONTRIBUTING / THIRD_PARTY_NOTICES / STATUS。完整选型、架构、S00–S14 与验收规格从旧 README 转移，规划目标和实际交付明确区分，开发规则改为引用新位置。
- 验收：内联 Python 检查通过（本地 Markdown 链接、原规划 16 节逐字保留、UTF-8、代码围栏、无 emoji、实际包名与启动路径）；git diff --check 通过。首轮检查误用 player-app 目录，按实际 yyplayer-app 修正后全部通过；仅文档变化，不重复编译、Clippy、播放或硬件验收。
- 未改变程序行为或硬件资格，真机范围沿用此前验收。本轮更改保留在 master 工作区，未提交或推送；下一步由用户通过 VS Code 审查、提交并推送文档。

## 2026-10-02：统一 main / master 主分支

状态：Done（本地分支整理）。用户明确要求合并两个主分支，统一保留 master。

- 操作前工作区干净，没有远端，只有当前一个工作树。main=4eca722、master=dev=f01498f；main 是 master 的祖先，main 没有独有提交，master 比 main 多 6 个提交。
- 在 master 执行 git merge --ff-only main，确认已全部包含；随后 git branch -d main 安全删除已合并的重复分支。提交本轮说明后 dev 快进同步，最终仅 master / dev，当前停留 master。
- 更新 README / AGENTS / PUBLISHING / STATUS 和分支检查记录。检查提交可达性、两个保留分支一致、git diff --check 和最终工作区状态；无代码变化，本轮不重跑编译 / 播放测试。
- 所有原 main 提交（包括 4eca722）仍可从 master 访问，没有改写历史。没有远端分支或 GitHub 默认分支可修改，没有推送。
- 下一步：用户在 VS Code 发布 master；目录仍为 E:\SourceFiles\rust\yyplayer。此前章节描述的是当时的分支状态，当前以本节为准。

## 2026-10-02：GitHub 公开源码发布准备

状态：Done（本地源码仓库准备）。用户明确选择 GPL-3.0-only，GitHub 发布由用户在 VS Code 完成。

- LICENSE 使用锁定 Slint 1.17.1 随附的完整 GPLv3 标准文本；Cargo workspace 与五个 crate 声明 GPL-3.0-only，README 明确源码 / 原创资源授权。新增第三方记录和 383 个 Windows normal / build 可达依赖的许可元数据清单，未把它当作完整二进制许可审计。
- 增加 .gitattributes 源码 LF / PNG binary，补忽略库缓存、损坏配置备份与 VS Code 本地设置；新增 CONTRIBUTING 和具体 VS Code 发布步骤。
- 无冲突快进合并 dev 到 master，准备文档一并提交，dev 保留相同发布准备版本，当前停留 master；旧 main 保留历史视频基线。未创建远端 / GitHub 仓库，未推送。
- 检查：Cargo metadata --locked --offline（五个许可字段及 Windows 依赖图）、fmt --check、git diff --check、git fsck、忽略规则、已跟踪文件 / 历史 blob 体积和常见敏感标记扫描。仅许可 / 元数据 / 文档 / Git 配置变更，本轮不重复编译、Clippy、播放或硬件测试；沿用上轮验收。
- 发布根目录：E:\SourceFiles\rust\yyplayer。具体步骤见 docs/PUBLISHING.md；检查证据见 docs/validation/source-publication.md。
- 下一步：用户在 VS Code Publish to GitHub、选择公开仓库，确认默认分支 master。正式二进制发行仍补 runtime notices / 对应源码 / 依赖闭包，S00 / USB / HDR / 跨平台资格不因公开源码标 Done。

## 2026-10-02：原生窗口外观、音乐库封面与歌词页精简

状态：Done（本轮 Windows 开发预览实现 / 本机验证）。目标在 README 先记录；继续在 dev，未合入主分支、未发布。

- platform / app 新增可选 WindowSurface adapter：窗口化 DWM 圆角 + Winit 原生阴影，最大化 / 全屏方角；借用 HWND，仅状态变化更新。原生角部截图已看到真实圆角与阴影；不改变 GL prepare_gl 或 presenter。
- app 新增后台可见行缩略图服务，UI TrackRow / Cover 显示真实内嵌 / 同名 / 目录封面，损坏 / 无封面回退；64px、8 项队列 / 在途、256 项 LRU、重扫 generation 取消，结果按资源批次更新。
- Persistence 排除 recent 比较实际设置变化，history dirty 与 settings dirty、通知 revision 与磁盘 revision 分离；播放静默保存历史，同值不提示，实际成功修改才显示保存浮窗，失败仍可见。
- 大页面删除收起 / 自动跟随 / 更换歌词；自动歌词与 seek 保留，手动导入在播放选项。左下封面切换浏览 / 大页面，大封面返回音乐库；两种真实点击验证通过。
- fmt / 严格 all-targets clippy / all-targets tests / Release 通过，20 个独立单测。Test-UiCustomization 22 阶段、ui-feedback 17 点、navigation-ui 10 点、Debug 视频 11 动作 / 265 次 render 通过。原生 Release 窗口 result=0、preference=2；最大化 / 还原回读验证通过。
- `.gitignore` 已创建且补充本地 .env / 配置、PDB / dump / 原子临时文件 / 系统杂项。已跟踪文件 / 历史体积 / 常见密钥标记检查未发现不应上传的二进制或匹配；没有 Git 远端、根目录没有 LICENSE。GitHub 准备见 docs/PUBLISHING.md，未自动选择许可或推送。
- 安全清理已移除 5,433,771,005 字节（约 5.06 GiB）构建 / 测试缓存，保留 Release exe 与固定 DLL；自有视频临时素材另清理。提交前工作区约 151 MiB，target 仅余 21,758,976 字节 exe。清理后独立配置启动、原生圆角回读、runtime 与正常退出再次通过。
- 已更新 README / ADR 0005 / 本报告 / 接续规则。详细命令、图片与修正记录见 [窗口与封面验收](validation/window-artwork.md)。Windows 10 / 远程 / 贴靠、多屏 DPI 与 macOS / Linux 仍待验；未追加数字捕获或 HDR 资格。
- 下一步：选定应用许可证和正式主分支、合并 dev，再配置仓库并推送；发行包另补 runtime notices / 对应源码 / 依赖闭包。

## 2026-10-01：字体、集成窗口控制栏与音乐库列宽

状态：本轮 Windows 开发预览实现及验收已完成；物理窗体操作与跨平台资格待验。README 在实现前记录目标，继续在 dev，未合入主分支。

- core 新增 Fonts / LibraryColumns 配置默认与校验；platform 新增后台 Windows GDI 字体族枚举，app 新增有界字体服务 / controller，UI 新增字体设置页。全局 / 歌词 / 字幕分别选择，默认继承与缺失回退可见；列比例和字体原子保存。
- Engine 线程应用 sub-font / ASS 字体覆盖，检查返回并尝试全部旧值回滚；默认尊重作者字体。snapshot 回读实际属性。图片 / 烧录字幕不可更换，复杂 ASS 行内排版待专项。
- Window no-frame，自绘主题控制栏与原生 Winit 最小化 / 最大化 / 还原 / 拖动 / 缩放动作；关闭正常清理。Windows 右置、macOS 左置红黄绿布局边界，视频随顶部自动隐藏，全屏不显示。真实最大化 / 还原 / 最小化 / 关闭断言通过；物理拖动 / 边缘缩放、多屏 DPI 和 macOS / Linux 真机待验，不支持原生 Snap 悬停菜单。
- 音乐库歌曲 / 歌手 / 专辑 / 时长列共用表头与行公式，两条分隔线拖动和双击恢复，比例保存；窄窗口简化封面。移除与提示关闭改为居中 SVG 图标，24px 滚动条留空。实际拖动 0.46 → 0.56、保存读回、80 → 79 行移除通过。
- Test-UiCustomization 20 阶段通过：278 字体、真实 Slint 全局下拉、歌词 / SRT / ASS 字体、持久化、两尺寸布局、左置布局 / 点击及 Windows 窗口动作。已审查自有 GPU 图片；纠正隐式位置、下拉重建、过短测试音频及移除后的索引变化，旧失败不作通过证据。
- fmt / 严格 all-targets clippy / all-targets tests 通过，17 个独立单测。ui-feedback 17 点、navigation-ui 10 点复验通过。最终 Release 构建通过；独立配置重启恢复字体 / 列宽、默认 / 缺失字体回退、原生装饰关闭通过。
- 成品自有视频 11 动作 / 266 次 render 回归通过，真实最大化 / 全屏 / 退出、暂停解码重载保留位置、字体保持和正常退出。清理移除约 4.67 GiB 编译 / 测试缓存，提交前工作区约 161.7 MiB；target 只留 21,572,608 字节新版 exe，固定 DLL 哈希一致，清理后三配置启动再次通过。
- 未改变音频默认、WASAPI / EQ / 解码策略或 GL prepare_gl；未重复数字捕获、4K HEVC / HDR / 性能专项。设计见 [ADR 0004](adr/0004-fonts-window-chrome-and-columns.md)，命令 / 图片 / 限制见 [验收报告](validation/ui-customization.md)。
- 下一步：物理窗口拖动 / 八方向缩放 / 双击 / Snap、多屏 DPI，字体安装 / 卸载 / 缺字 / 复杂 ASS，之后按音频记录补 USB DAC 资格。

## 2026-10-01：主界面空白 / 侧栏修复与工程清理

状态：Done（本轮 Windows 开发预览修复 / 清理范围）。继续在 dev，未合入主分支。

- README 先记录目标。app.slint 显式启动页面计时器；内容改为可伸展 Rectangle，阻断子控件最大宽度向上传递。真实复现旧版音乐 / 设置 opacity=0、1240px 中音乐内容仅 394px；修复后 opacity=1、内容 1038px，选项栏始终靠右。
- music.slint 新增居中空库引导、目录 / 单曲按钮和空库工具精简；controller 无启动文件默认音乐库。删除导航版本小字，时尚上下各 10px 留白。
- 新增 navigation-ui：10 点实际 Slint 鼠标事件 / 三尺寸 / 透明度 / 几何 / 减少动态效果 / 简洁主题通过；已查看空库、最小面板、浅色设置和生产库截图。首版回归例子借用未释放的故障已修正，不计作通过证据。
- fmt、严格 all-targets clippy、16 个独立 tests 通过；ui-feedback 17 点、Test-LibraryTheme 13 阶段真实目录导入 / 播放 / 保存 / 多主题复验通过。未修改普通 mpv / WASAPI / EQ / GL 状态边界，未重复 4K 像素或数字捕获专项。
- Clean-Workspace.ps1 先 WhatIf 核对，限定工作区并拒绝链接；保留 Release exe、runtime DLL / 头文件、源码、Git、docs，不处理 APPDATA / 媒体。编译与验收缓存清理后可通过脚本再生成。
- 最终 Release 构建通过；清理前后独立配置空库启动均 page=0、固定 mpv 成功加载、无渲染 / 引擎错误、正常退出。清理移除 28,873,858,504 字节（26.89 GiB）；工程由 29,040,684,385 缩至约 166,825,881 字节（159.1 MiB，含 Git / docs / exe / DLL）。target 只留 20,774,912 字节的 Release exe，固定 DLL 哈希仍一致。后续文档 / Git 提交会带来小幅体积变化。
- 验收报告和保留图片见 [主界面修复记录](validation/navigation-cleanup.md)。上轮 Timer / 页面正常的结论被本次用户反馈纠正；旧测试漏了进入页面的透明度 / 几何断言。
- 下一步：真实 OS 对话框与多屏 DPI 人工验收，以及此前 USB 格式 / 拔插 / 数字捕获资格。

## 2026-10-01：目录音乐库、简洁 / 时尚主题和动效

状态：Done（本轮本机开发预览范围），全平台 / 大库 / 位准确资格未完成。音频基线 8467ca9 已快进合入 master，main 保留视频 4eca722，新功能在 dev。用户魅蓝 DSP 小尾巴 WASAPI 独占确认记录为用户实测，未推断 bit-perfect。

- core 新增 Appearance / LibrarySettings / Song 与默认兼容；platform 新增只读系统外观后台观察；app 新增目录 / 标签 / 缓存服务、取消 generation、目录和单曲管理及队列 / 搜索 Rc 投影缓存；ui 新增主题令牌、GlassSurface、外观设置、虚拟化音乐库和短时过渡。
- 目录添加 / 单曲添加、递归 / 重叠去重、搜索 / 按目录筛选、更新 / 取消 / 移除，独立于播放队列，不删除磁盘文件。缓存原子保存，离线目录保留记录。当前 20000 首 / 200000 项 / 64 层，未完成完整数据库 / 增量监听音乐库。
- 简洁 / 时尚、系统 / 浅 / 深、系统 / 自选主题色独立保存。Windows 实际读到当前 dark=true 和 DWM 色 316da1；手动切换与四种主题截图通过。时尚为轻量透明 / 高光 / 阴影，未实现 Apple 原生折射或实时模糊；视频 GPU 路径不变。
- Test-LibraryTheme 13 阶段实际生产 controller / Engine / GL presenter：目录导入、筛选、真实播放、移除后继续播放、重扫 / 取消、保存和四主题通过。已查看常规 / 设置 / 最小面板图；首轮布局溢出与软件例子的 GL 前置条件遗漏已纠正，未拿旧失败作通过证据。
- ui-feedback 17 点通过；截图耗时不再压缩下一超时间隔，首帧后显式开启计时。页面 Timer 的首次启动缺陷已在上方本轮修复；减少动态效果停持续 Timer / 短动画时长归零。不宣称已测量功耗或性能提升。
- fmt、严格 all-targets clippy、16 个独立测试、Debug / Release 构建通过；新增颜色 / 旧设置、扫描取消 / 排除和离线缓存 / 过期扫描测试。音乐大页面 audio-ui 五图 / 实际 Slint 点击、Test-Video 11 动作与暂停位置重载通过；4K HEVC 图像回归硬解差 0、去色带差 0.205388、明显偏差均 0%。
- 最终 Release 再次通过 Test-Audio：真实 FLAC / 封面 / LRC、共享 / 严格独占、全局 / 设备 / 单文件 EQ、保存 / 正常退出；768kHz 源协商 192kHz 时暂停。最新自有 FLAC SHA256 为 201ea717873fe7d586755daa5e8cb11a68c60330a1b108dbf1b03975a102875a。成功共享回退 / USB 数字捕获仍未增加证据。
- 当前改动和设计见 [ADR 0003](adr/0003-library-and-themes.md)，检查命令、实测与限制见 [验证报告](validation/library-themes.md)。
- 下一步：OS 对话框、真实系统偏好热切换、大目录 / 网络盘 / 长时性能，macOS / Linux 系统主题适配；USB 格式 / 拔插 / 数字捕获资格仍按音频报告。

## 2026-10-01：音频、WASAPI 独占、分级 EQ 与音乐大页面

状态：完成本轮实现与本机开发预览验证，S01 的完整设备 / 位准确资格尚未 Done。目标先记录于 README；视频版本 4eca722 已快进合入 main，音频代码在 dev。

- core 新增音频策略 / EQ 校验、JSON / APO 导入、规则优先级、LRC；mpv Engine 新增 WASAPI 日志证据、同设备回退、严格拒绝 / 源率检查、设备变化暂停、可恢复重试、命名 AF 链替换 / 回滚。正常视频 GL 边界保留。
- app 新增有界标签 / 封面 / 歌词 / 预设后台服务、带媒体身份的回复、共享音频设置与原子导出；ui 新增可折叠音频 / EQ 面板、大页面和动画。默认共享、无 EQ / ReplayGain、自动协商、70% 音量，不伪造 bit-perfect。
- 真实 FLAC / 内嵌 PNG / 同名 LRC、Realtek 共享 48kHz / 独占 44.1kHz、全局 / Waves SoundGrid 设备 / 单文件 EQ 覆盖及清空 / 保存通过。768kHz 源协商 192kHz 时严格源率策略实际暂停。
- Test-AudioBusy：真实 Realtek 独占持有者 + 严格竞争者 + 优先竞争者，严格拒绝，优先尝试同设备共享后也拒绝，未切换其他设备，正常退出。成功回退到共享的实际设备案例仍待验。
- audio-ui 五张截图已审查，封面展开 / 收起 / 歌词 seek 实际 Slint 点击通过；17 点 ui-feedback 复验通过。布局截图带示例标记；不代替播放或 OS 对话框人工证据。
- fmt、严格 workspace all-targets clippy、workspace all-targets tests、Debug / Release 构建通过；新增 8 个边界测试，含旧设置、极端 offset、输入格式、WASAPI 非致命重试、模式 / 设备身份确认与端点变化。总计 13 个独立测试；测试例子复用部分 services 测试，不重复计独立测试数。最终 Release 成品再次通过 Test-Audio / Test-AudioBusy，最终 Debug 成品再次通过 Test-Video / audio-ui。
- Test-Video 11 动作 / 暂停位置重载 / 保存复验通过；4K HEVC NVDEC / 软件 / 去色带 GPU 比较通过，均无明显偏差，平均 RGB 差 0 / 0.205388。不是音频效果或性能证据。
- 成品 target/release/yyplayer.exe。实现 / 设备边界见 [ADR 0002](adr/0002-audio-hifi-preview.md)、[音频报告](validation/audio-windows.md)。USB DAC、真实拔插、数字捕获对比、成功共享回退、广泛格式 / 多声道、跨平台和性能未验证。
- 下一具体动作：先做实际 USB DAC 格式 / 竞争 / 拔插 / 成功回退矩阵，然后数字捕获验证；并补 S00 工具链 / 发行许可。联网歌词、ASIO / DSD DoP / native DSD、gapless 和完整音乐库仍未实现。

## 2026-10-01：视频顶部浮层与控件居中

状态：Done（本机开发预览范围）。目标先记录于 README。视频顶部工具栏移出占位布局，从内容区 y=0 开始覆盖画面，52px 高、3 秒无操作后隐藏，指针 / 触摸 / 按键唤醒；顶部悬停、拖动、打开选项时保持可用。底部左侧媒体信息、整个窗口正中央播放按钮、右侧倍速 / 音量 / 选项；删除独立 1.00x 文字，实际非预设速度也统一在下拉框显示。

- 修改 app.slint / PlayerBar、指针顶部悬停处理、view_model / shell / controller 的倍速投影；未新增依赖、线程，未改变 GL 交接。
- ui-feedback 扩为 17 个检查点并通过：顶部在窗口模式超时 / 悬停保护、没有上方空白、顶部隐藏不改变视频高度；在 1000 / 1240 / 1460px 窗口正中央实际 Slint 点击均命中播放按钮。已查看最小 / 常规可见 / 隐藏、音乐页面截图，没有控件重叠。物理鼠标 / 触摸 / 多屏 DPI 验收仍未增加。
- fmt、严格 workspace all-targets clippy、workspace all-targets tests、Debug / Release 构建通过；成品位于 target/release/yyplayer.exe。
- Test-Video 11 动作、真实最大化 / 全屏 / 暂停解码重载 / 保存 / 退出再次通过；更新自有素材 GPU 图片与诊断数据。最新截图为 1550 × 1125，桌面为 125% 缩放，测试未改变系统设置。
- Test-Render 实际 NVDEC / 软件 / 去色带 4K HEVC 图片比较通过：119 / 144 / 146 次 render，平均 RGB 差 0 / 0.20539，明显偏差比例均 0%。与此前 175% 结果不做性能比较。
- 实现与验证范围见 [UI 记录](validation/ui-feedback.md)。下一步仍为下方接续计划，不扩大其他 GPU / 平台资格。

## 2026-10-01：浮空提示与全屏控制栏

状态：Done（本机开发预览范围）。目标先记录于 README。设置保存等状态移为浮空卡片，4 秒消失，可点击 × 关闭；文字变化 / 新保存 revision 重新显示，常规状态投影不重置计时。视频全屏控制栏覆盖底部，3 秒空闲后隐藏，指针 / 触摸 / 按键唤醒；悬停控制栏、按住拖动、打开选项时保持可见，失焦释放交互保护。退出全屏恢复常显，显示 / 隐藏不改变视频尺寸。

- 涉及 app.slint、shell / view_model、bootstrap / controller；新增开发例子 ui-feedback，未新增依赖、线程或改动 mpv GL 状态交接。
- ui-feedback 的 12 个检查点通过，包括实际 Slint 关闭按钮点击、相同文字再次提示、关闭后投影、超时和交互保护；已查看软件布局截图，不冒充 OS 物理鼠标 / 触摸 / 多屏 DPI 验收。
- fmt、严格 workspace all-targets clippy、workspace all-targets tests、Debug / Release 构建通过，成品位于 target/release/yyplayer.exe。
- Test-Video 真机复验通过 11 动作、窗口模式、暂停解码重载 / 持久化 / 安全退出，更新自有素材 UI 截图与诊断 JSON。
- Test-Render 再次通过实际 NVDEC / 软件 / 去色带 4K HEVC 图像比较：133 / 138 / 139 次 render，无去色带平均 RGB 差 0，去色带 0.24812，明显偏差像素均 0%。Test-Video / Test-Render 显式 UTF-8 读取 JSON，修复 Windows PowerShell 5.1 默认 ANSI 解码误报。
- 详细实现、检查范围与人工待验场景见 [交互验证记录](validation/ui-feedback.md)。下一步仍按下方接续计划；macOS / Wayland / X11 尚未验证。

## 2026-10-01：修复硬解横向噪点

状态：Done（本机问题修复 / 验证范围）。用户报告 4K HEVC 在关闭去色带时画面损坏。独立配置实际复现 NVDEC + 无去色带异常；软件与去色带正常。原因是 presenter 遗漏 libmpv 所需的共享 GL 默认状态交接。现对 create / update / render / free 恢复标准状态，缩放分配 FBO 后也再次恢复；保留默认硬解与去色带设置，不强制软解 / deband、不增加正常视频 CPU 回读。

- 原视频取样：修复前 25.66795% 像素明显偏离软件参考，修复后 NVDEC / deband=false 同区域误差为 0；原文件未修改，个人图片 / 视频不提交。
- 新 Test-Render / render-compare：自有静态 4K HEVC、实际 NVDEC / 软件 / 去色带 GPU 图片比较通过，补足上一轮只验状态 / 帧数所遗漏的画质检查。
- fmt、严格 all-targets clippy、workspace tests、debug / release 构建通过；Test-Video 的最大化 / 全屏 / 暂停解码重载复验通过，更新自有素材 UI 图片及控制数据。
- 旧 release 窗口保持运行；旧 executable 已在 target/release 同目录改名备份，新 yyplayer.exe 重新构建，用户需关闭旧窗口后重启。
- 详细原因、固定源文件依据、hash / 像素阈值 / 当前资格边界见 [渲染修复报告](validation/render-state-fix.md)。原视频报告里的状态检查仍有效，旧图像不再当作画质通过证据。

## 本轮实际实现

- 五个 crate 沿用：core 新增设置 / 快捷键 / 状态机；mpv 新增固定 runtime manifest、校验脚本、受控 loader、FFI、worker / render 租约；ui 新增 GPU 呈现和真实共用控件；app 新增事件 / controller / 文件对话框 / 原子持久化。
- 本地音视频实际播放，多文件打开 / 拖放 / CLI / 最近文件、队列、暂停 / 停止 / seek、音量 / 静音 / 倍速、输出设备、轨道 / 外挂字幕 / 延迟、章节 / 逐帧 / PNG / 循环 / 比例均已接入。
- 视频页紧凑、普通 / 最大化 / 全屏；音乐与视频共用 PlayerBar。真实运行显示本地队列与实际封面 / 标签 / 歌词；开发例子的样例播放状态有标记，不作为设备输出证据。
- 快捷键可配置；上下音量、左右短按默认 5 秒、长按默认 3x 后恢复；焦点 / 对话框 / 页面 / 媒体变化取消临时加速。实际运行信息包含请求与实际硬解路径。
- 全局 → 最深祖先文件夹 → 单文件完整解码规则（后者优先）；自动 / 软件 / 硬件优先、线程 / 去隔行 / 去色带，保存并重载保留暂停 / 位置。设置与最近 30 个文件后台原子保存。
- 正常视频 libmpv OpenGL → RGBA8 GPU FBO → Slint 借用纹理，不每帧 CPU 回读。开发预览路径决定见 [ADR 0001](adr/0001-libmpv-video.md)；原框架决定见 [ADR 0000](adr/0000-framework-preview.md)。

## 实际验证

本机 Windows 11 / Rust 1.97.0 / Slint 1.17.1，固定 mpv v0.41.0-1087-ge470f8986，RTX 5060 Laptop。详细 runtime / 驱动 / fixture hash 与未测场景见 [Windows 视频报告](validation/video-windows.md)。

| 实际执行 | 结果 |
| --- | --- |
| 获取固定 runtime、archive / DLL SHA256、ABI 核对 | 通过，本地 DLL 可实际加载；二进制不提交。 |
| cargo test --locked --workspace --all-targets --offline | 通过，当前 16 个独立测试；services 在例子 target 的复用执行不重复计数。 |
| cargo clippy --locked --workspace --all-targets --offline -- -D warnings | 通过。 |
| cargo build --locked -p yyplayer-app --offline | Debug 通过。 |
| cargo build --locked --release -p yyplayer-app --offline | Release 通过。 |
| cargo fmt --all -- --check / git diff --check | 通过。 |
| scripts/Test-Video.ps1 -SkipBuild | 真机通过 11 动作 / 暂停软解重载 / 规则持久化 / 独立软解 / 正常退出。 |
| release 成品 7 秒运行 | NVDEC、192 次 render、进度 6.333 秒、无错误、exit 0。 |
| 真实 GPU UI 截图与源帧对照 | 修复方向与 slider fill，对照确认；见 [截图](video-ui.png)。 |

脚本走生产 controller / 内核 / native window，不等于 OS 键盘 / 对话框逐项操作验收。所有功能广泛素材 / 设备矩阵和稳定性仍待补齐。此前探索阶段未通过的 render 日志不作为最终 Pass 证据；本轮固定构建的 ns/us 差异已写 ADR 和单测。

## 阶段状态

| 任务 | 状态 | 本轮证据 / 剩余 |
| --- | --- | --- |
| 完整规划 / 原 UI 框架 | Done（各自限定范围） | DEVELOPMENT_PLAN / AGENTS / ADR 0000；旧音乐布局截图 ui-preview.png。 |
| 本轮：视频功能实现 | Done（开发预览范围） | 实际 engine / GPU / 控件 / 快捷键 / 规则 / 保存；变更见 CHANGELOG，资格限制见验证报告。 |
| S00 | Partially verified | 可追溯 runtime、loader、锁文件、debug / release 已有；具体 toolchain pin、发行许可 / 依赖闭包未完成，MSRV 未测试。 |
| S01 | Partially verified（音频开发预览） | WASAPI 共享 / 独占、分级 EQ、源率拒绝与竞争已验；USB DAC、物理拔插、成功共享回退、数字捕获未验。 |
| S02A | Partially verified | GL 合成本机 NVDEC / 软件播放、方向 / 帧数 / 退出通过；广泛格式、性能、跨平台待验。 |
| S02B | Not started | Windows HWND / D3D11 / HDR 专项仍是候选。 |
| S03 / S04 | In progress（真实实现） | worker、异步 UI、会话队列、导入、设置 / 最近保存已有；复杂竞态、DB、正式队列语义、库待补齐。 |
| S05–S08 | In progress（部分功能） | 音视频控制、独占 / EQ / 本地歌词已接；发行、真实设备失联与全素材矩阵未完成。 |
| S09 | Partially implemented | 目录音乐库 / 标签索引 / 筛选 / 缓存已有；DB / 专辑 / 增量 / 大库资格未完成。 |
| S10–S12 | Not started（专项） | gapless、续播、系统媒体键、HDR、性能定型未完成。 |
| S13 | Not started | 仅 cfg / 库入口 / 设置路径预留，macOS / Wayland / X11 未编译 / 实机验证。 |
| S14 | Not started | 安装 / 卸载、许可、长时稳定性与正式发布未完成。 |

## 接续步骤

1. 按验证报告人工用例补齐 OS 快捷键 / 对话框、多轨字幕、解码优先级、快速连续切文件 / EOF / 重播、窗口与设备变化。
2. 固定 Rust release toolchain 并核对锁定依赖的 MSRV；选择应用 / Slint 许可和合格 runtime 对应源码 / notices，验证 portable 包依赖闭包。
3. S01 的实际 USB DAC 格式 / 独占 / 成功回退、物理设备失联和数字捕获；本机实现与资格边界见音频报告。
4. 性能基准和原生 / HDR 资格，再执行跨平台编译及真机矩阵。

无阻塞本轮开发预览的已知问题。尚未完成的资格条件不能写“支持所有平台 / 格式”“零拷贝”“HDR 输出”“独占已确认”。

## 后续每轮记录

记录日期、具体任务、修改范围、真实执行的检查、runtime / 驱动 / fixture hash、证据路径、未验条件、已知问题、下一具体动作。状态允许 Not started、In progress、Partially verified、Done、Blocked；用户范围优先，但不得把没跑过的检查写为通过。
