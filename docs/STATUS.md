# YYPlayer 进度与证据

更新：2026-10-01。当前交付为 **Windows 视频播放开发预览**，原始框架于 2026-09-30 完成。本轮先记录目标并初始化 Git，旧框架基线 `d10a4e8` 位于 main，开发位于 dev。

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
- 视频页紧凑、普通 / 最大化 / 全屏；音乐与视频共用 PlayerBar。音乐库专辑 / 封面仍为标注的示例，没有虚构实际文件 / 进度 / 设备状态。
- 快捷键可配置；上下音量、左右短按默认 5 秒、长按默认 3x 后恢复；焦点 / 对话框 / 页面 / 媒体变化取消临时加速。实际运行信息包含请求与实际硬解路径。
- 全局 → 最深祖先文件夹 → 单文件完整解码规则（后者优先）；自动 / 软件 / 硬件优先、线程 / 去隔行 / 去色带，保存并重载保留暂停 / 位置。设置与最近 30 个文件后台原子保存。
- 正常视频 libmpv OpenGL → RGBA8 GPU FBO → Slint 借用纹理，不每帧 CPU 回读。开发预览路径决定见 [ADR 0001](adr/0001-libmpv-video.md)；原框架决定见 [ADR 0000](adr/0000-framework-preview.md)。

## 实际验证

本机 Windows 11 / Rust 1.97.0 / Slint 1.17.1，固定 mpv v0.41.0-1087-ge470f8986，RTX 5060 Laptop。详细 runtime / 驱动 / fixture hash 与未测场景见 [Windows 视频报告](validation/video-windows.md)。

| 实际执行 | 结果 |
| --- | --- |
| 获取固定 runtime、archive / DLL SHA256、ABI 核对 | 通过，本地 DLL 可实际加载；二进制不提交。 |
| cargo test --locked --workspace --all-targets --offline | 通过，5 个独立边界测试，services 在例子 target 复用，总 6 次执行。 |
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
| S01 | In progress（共享音频输出） | 观察实际 WASAPI；独占 / USB DAC / bit-perfect / EQ 未实现与验证。 |
| S02A | Partially verified | GL 合成本机 NVDEC / 软件播放、方向 / 帧数 / 退出通过；广泛格式、性能、跨平台待验。 |
| S02B | Not started | Windows HWND / D3D11 / HDR 专项仍是候选。 |
| S03 / S04 | In progress（真实实现） | worker、异步 UI、会话队列、导入、设置 / 最近保存已有；复杂竞态、DB、正式队列语义、库待补齐。 |
| S05–S08 | In progress（部分功能） | 日常视频功能及控制已接；独占 / EQ / 发行 / 设备丢失策略 / 全素材矩阵未完成。 |
| S09–S12 | Not started（专项） | 完整音乐库、gapless、续播、系统媒体键、HDR、性能定型未完成。 |
| S13 | Not started | 仅 cfg / 库入口 / 设置路径预留，macOS / Wayland / X11 未编译 / 实机验证。 |
| S14 | Not started | 安装 / 卸载、许可、长时稳定性与正式发布未完成。 |

## 接续步骤

1. 按验证报告人工用例补齐 OS 快捷键 / 对话框、多轨字幕、解码优先级、快速连续切文件 / EOF / 重播、窗口与设备变化。
2. 固定 Rust release toolchain 并核对锁定依赖的 MSRV；选择应用 / Slint 许可和合格 runtime 对应源码 / notices，验证 portable 包依赖闭包。
3. S01 的实际 USB DAC 独占、设备失联安全策略与 EQ；设备选择不能代替这些功能。
4. 性能基准和原生 / HDR 资格，再执行跨平台编译及真机矩阵。

无阻塞本轮开发预览的已知问题。尚未完成的资格条件不能写“支持所有平台 / 格式”“零拷贝”“HDR 输出”“独占已确认”。

## 后续每轮记录

记录日期、具体任务、修改范围、真实执行的检查、runtime / 驱动 / fixture hash、证据路径、未验条件、已知问题、下一具体动作。状态允许 Not started、In progress、Partially verified、Done、Blocked；用户范围优先，但不得把没跑过的检查写为通过。
