# YYPlayer 进度与证据

## 当前状态

- 2026-09-30：完成规划，随后按用户要求交付 **v0.0.1 Rust / Slint 框架与 UI 预览**。最初目录为空，未发现 Git 仓库。
- 五个 crate 已建立：player-core、player-mpv、player-platform、player-ui、yyplayer-app。
- 音乐 / 视频 / 最近打开 / 设置四页、主题、SVG 资源、示例队列与底部控制栏已实现。导航 / 搜索 / 选择 / 前后切换 / 收藏图标 / 音量预览连接到统一 controller，仅为内存中的界面交互。
- 真实播放、媒体导入、输出设备、独占、EQ、字幕、硬解、系统集成、配置保存和媒体库未实现。占位 MpvEngine 返回 NotConnected，真实快照保持 Idle，能力全部 false。
- Slint / slint-build 固定 1.17.1，Cargo.lock 已生成；普通应用为 Winit / FemtoVG / accessibility。软件截图及 PNG 编码为开发依赖。
- 开发机：Windows，Rust 1.97.0 / Cargo 1.97.0，x86_64-pc-windows-msvc。工具链文件暂用已安装 stable；workspace MSRV 1.92 未单独验证。
- 启动命令见 README“当前框架与快速启动”；版本与范围决定见 [ADR 0000](adr/0000-framework-preview.md)。

## 本轮改动与检查

改动：workspace / 锁文件 / 工具链文件 / .gitignore、五个 crate 的前后端代码、Slint 组件与页面、原创 SVG 图标和封面、开发截图例子、third_party/mpv/README、README / AGENTS / STATUS / ADR。

| 实际执行 | 结果与适用范围 |
| --- | --- |
| `cargo check --workspace` | 首次受网络限制失败；获取依赖后定位并修复 Slint 编译错误。不是播放验收。 |
| `cargo check --workspace --offline` | 通过，五个 crate 可编译。 |
| `cargo fmt --all`、`cargo fmt --all -- --check` | 最终格式检查通过。 |
| `cargo build --locked -p yyplayer-app --offline` | 最终 debug 构建通过，生成 target/debug/yyplayer.exe。 |
| `cargo run --locked -p yyplayer-app --example ui-preview --offline -- --output docs/ui-preview.png` | 运行实际 Slint 组件、保存 PNG 并正常退出；人工查看音乐页，修复控件对齐、布局循环和装饰线位置。 |

[音乐页截图](ui-preview.png) 为 2170 × 1575 物理像素（1240 × 900 逻辑像素，环境缩放 175%）。截图使用 Winit 软件渲染器；普通 FemtoVG 应用已经编译，生产 GPU 显示、其他页面交互与多屏 DPI 尚未做完整运行验收。

遵照用户“只要框架、不要过多检验”的范围，未执行完整 clippy / tests / release，也未执行音视频播放、USB DAC、HDR、性能、长时运行或跨平台验收。没有 libmpv runtime build ID 或媒体 fixture hash；本轮没有下载播放内核。

## 任务状态

| 任务 | 状态 | 证据 / 待办 |
| --- | --- | --- |
| 文档规划 | Done | README 完整规格 / S00–S14 / 37 个验收场景；AGENTS 开发规则。 |
| 本轮：前后端框架与 UI | Done（限定范围） | 五个 crate、四页 Slint UI、controller 绑定、锁文件、debug 构建与音乐页截图；不包含真实播放。 |
| S00 | In progress | 工程 / 基本窗口 / 同版本 UI 编译器已建立；具体工具链固定、libmpv manifest / loader / 获取脚本、release 运行待完成。 |
| S01 | Not started | 音频 probe；需内置声卡与实际 USB DAC。 |
| S02A / S02B | Not started | GL / Windows 原生视频 probe 与 ADR。 |
| S03 / S04 | In progress（仅框架） | 基本类型 / controller / 页面已建；真实 engine worker、异步状态、队列语义、导入与持久化未实现。 |
| S05–S08 | Not started | Windows v0.1 实现与发行验收。 |
| S09–S12 | Not started | Windows 日用功能、HDR、性能定型。 |
| S13 | Not started | macOS / Wayland / X11 仅有 cfg 边界，未做跨平台编译与真机适配。 |
| S14 | Not started | Windows v1.0 稳定性与正式发行。 |

## 下一具体步骤

1. 补齐 S00：选择可追溯 Windows x64 libmpv 构建，建立 manifest / checksum / 获取脚本 / 受控 loader，固定具体 Rust 版本并验证。
2. 建立独立 Engine worker、有界命令通道和快照投递，再允许真实引擎接入 controller；UI 回调不得直接同步调用 libmpv。
3. 完成 S01 / S02 的真实音频和呈现 probe。通过后逐步使用真实状态替换 demo，不把示例数据写入媒体库或设备设置。

暂无阻塞本轮交付的问题。Windows 最低系统版本、字体回退、可访问性、多屏布局、音视频硬件及跨平台能力仍需后续证据。

## 后续每轮记录模板

```text
日期：
任务 / 子步骤：
前置检查：
改动文件：
检查命令 / 结果（实际执行）：
运行环境 / runtime build ID：
验收 ID / 素材 hash / 结果 / 证据路径：
限制 / 未验证环境：
已知缺陷 / 阻塞：
下一具体步骤：
```

状态允许 Not started、In progress、Partially verified、Done、Blocked；硬件条件暂缺时记录条件，不把未测结果写成 Pass。阻塞不妨碍推进不依赖该条件的其他明确任务。
