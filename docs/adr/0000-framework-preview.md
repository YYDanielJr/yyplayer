# ADR 0000：先交付可运行框架与界面预览

- 日期：2026-09-30
- 状态：框架范围采用；播放技术选型仍等待 S00–S02 实验

## 背景与证据

最初只有实施规划。用户随后明确要求本轮先搭前后端框架、制作美观简洁 UI，不要求功能性，也不进行过多检验。该要求覆盖规划中“先做内核样机，再做完整 UI”的默认执行顺序，但没有要求提前声称播放能力完成。

开发机已安装 Rust 1.97.0 MSVC stable；本地基础缓存包含 Slint 1.17.1。核对该版本源码与 Cargo 元数据后，固定 slint / slint-build 同为 1.17.1，而不把在线候选 1.18.1 API 直接套进工程。缺少的依赖在本轮构建时获取，锁文件保留。

## 决定

1. 建立 core / mpv / platform / ui / app 五个 crate。仅引入当前需要的 Slint；动态加载、数据库、日志、异步通道等在实际实现阶段增加。
2. 主程序使用 Winit + FemtoVG。原创 SVG 图标和封面随 UI 编译，不加载外部媒体，不依赖 webview。
3. 示例数据只走 `MediaPreview` / `ShellViewModel`。占位 `MpvEngine` 能力全部为 false，快照为 Idle，播放提交返回 NotConnected。按钮与示例时长不代表实际播放。
4. 使用统一 UiAction → AppController → ShellViewModel → UiShell 流程，弱窗口引用避免强引用环。当前没有阻塞任务，故 controller 在 UI 线程立即处理；接入内核前必须迁移至规划中的 worker / 有界通道。
5. Windows 的入口使用 GUI subsystem，平台名称按 cfg 分离；没有借此宣称 SMTC、设备独占或其他平台运行完成。
6. 开发例子 `ui-preview` 使用软件渲染器捕获实际组件。初次 GL 截图的交换后 back buffer 为空，改用软件截图，不把黑色截图误判成生产窗口的 GPU 失败。软件 renderer 与 PNG 编码为 dev-dependencies。

## 本轮验证与影响

仅执行必要的 workspace 编译、debug 应用构建、格式检查与音乐页实际组件截图检查。未做完整 tests / clippy / release、音视频场景、硬件或跨平台测试。详细命令与结果见 STATUS。

S00 仍缺具体工具链版本固定、受控 libmpv manifest / loader / 获取脚本；S01 / S02 尚未开始。已有类型、控制器与 UI 可继续复用；后续真实实现不能保留同步 mpv 调用、demo 进度或“请求即成功”的状态假设。
