# YYPlayer 进度与证据

更新：2026-10-01。当前交付为 **Windows 音视频播放开发预览**，原始框架于 2026-09-30 完成。Git 框架 `d10a4e8` 保留在历史，视频 `4eca722` 保留 main；音频 `8467ca9` 已合入 master，本轮目录音乐库与主题继续在 dev。

## 2026-10-01：目录音乐库、简洁 / 时尚主题和动效

状态：Done（本轮本机开发预览范围），全平台 / 大库 / 位准确资格未完成。音频基线 8467ca9 已快进合入 master，main 保留视频 4eca722，新功能在 dev。用户魅蓝 DSP 小尾巴 WASAPI 独占确认记录为用户实测，未推断 bit-perfect。

- core 新增 Appearance / LibrarySettings / Song 与默认兼容；platform 新增只读系统外观后台观察；app 新增目录 / 标签 / 缓存服务、取消 generation、目录和单曲管理及队列 / 搜索 Rc 投影缓存；ui 新增主题令牌、GlassSurface、外观设置、虚拟化音乐库和短时过渡。
- 目录添加 / 单曲添加、递归 / 重叠去重、搜索 / 按目录筛选、更新 / 取消 / 移除，独立于播放队列，不删除磁盘文件。缓存原子保存，离线目录保留记录。当前 20000 首 / 200000 项 / 64 层，未完成完整数据库 / 增量监听音乐库。
- 简洁 / 时尚、系统 / 浅 / 深、系统 / 自选主题色独立保存。Windows 实际读到当前 dark=true 和 DWM 色 316da1；手动切换与四种主题截图通过。时尚为轻量透明 / 高光 / 阴影，未实现 Apple 原生折射或实时模糊；视频 GPU 路径不变。
- Test-LibraryTheme 13 阶段实际生产 controller / Engine / GL presenter：目录导入、筛选、真实播放、移除后继续播放、重扫 / 取消、保存和四主题通过。已查看常规 / 设置 / 最小面板图；首轮布局溢出与软件例子的 GL 前置条件遗漏已纠正，未拿旧失败作通过证据。
- ui-feedback 17 点通过；截图耗时不再压缩下一超时间隔，首帧后显式开启计时。页面 Timer 单次运行；减少动态效果停持续 Timer / 短动画时长归零。不宣称已测量功耗或性能提升。
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
| 完整规划 / 原 UI 框架 | Done（各自限定范围） | README / AGENTS / ADR 0000；旧音乐布局截图 ui-preview.png。 |
| 本轮：视频功能实现 | Done（开发预览范围） | 实际 engine / GPU / 控件 / 快捷键 / 规则 / 保存；目标见 README 顶部，资格限制见验证报告。 |
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
