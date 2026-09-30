# ADR 0001：开发预览采用 libmpv OpenGL 合成

日期：2026-10-01。状态：接受用于 Windows 视频开发预览，正式发行 / HDR / 跨平台资格待验。

## 背景与决定

用户要求从框架推进真实视频播放、丰富控件、可配置快捷键和分级解码设置，并保留音乐布局与跨平台空间。先记录目标、Git 保存旧框架，再在 dev 实现。沿用 Rust + Slint + libmpv，不引入第二套解码或音频时钟。

当前使用 Winit / FemtoVG 的 NativeOpenGL notifier，将 mpv Render API 输出到 RGBA8 FBO，再交给 Slint 的借用 OpenGL 纹理。Windows 真机已播放 H.264 / AAC、实际 NVDEC 与软件解码均通过，Slint 控件可正常合成；因此开发预览不以尚未完成的 HWND / D3D11 关卡阻塞视频功能。原生方案保留为后续性能 / HDR 专项候选，不能据此认定 GL 为最终所有场景的最优路径。

业务命令和快照共用；设备、轨道、实际解码信息取自 mpv。自动策略 `auto-safe`，强制软件 `no`，硬件优先 `auto-copy`；硬件优先可能回退。分级规则是完整 DecodeOptions 对象，单文件 > 最深祖先文件夹 > 全局。保存后 loadfile 重载并在匹配 FileLoaded 时恢复位置 / pause，避免对已被替换的加载事件恢复错误媒体。

## 线程、呈现与退出

普通 mpv 调用集中在 worker，UI 使用 64 项有界命令和合并快照，不阻塞等待 mpv。文件对话框、路径规范化和 JSON 保存有独立后台服务。快照投影使用稳定模型与设置 revision，不在每帧覆写编辑框。

渲染器只在当前 GL 上下文调用 render API，以及官方允许的时钟函数。更新回调仅做合并唤醒；`ADVANCED_CONTROL=1`、`BLOCK_FOR_TARGET_TIME=0`，在主事件循环按目标时间请求重绘；没有更新且尺寸不变时不重复解码绘制。新纹理替换前清除 UI 引用，旧纹理在 Slint 完成本帧后回收。FBO 不启用 flip；与 Slint top-left 借用纹理组合已对照源帧验证方向。

render bridge 的原子租约保持 worker core / library 存活；RenderingTeardown 中先撤回 UI 引用、解绑回调、render_free 和销毁 GL 对象，再释放租约；随后 worker 才解除普通回调 / terminate_destroy。没有对原始 mpv / GL 指针添加 unsafe Send / Sync。关闭应用通过正常事件循环和显式清理，不调用 process::exit。

正常播放不从 GPU 向 CPU 回读视频。debug 可选择在 35 帧后一次读取完整 UI 到 PPM，用于开发证据，不在 release 或正式视频输出中运行。

## 固定构建的时钟差异

固定构建 `e470f8986e` 的 `render.h` 注释说 `target_time` 为微秒，但 [vo_libmpv.c](https://github.com/mpv-player/mpv/blob/e470f8986e/video/out/vo_libmpv.c) 实际直接赋 `vo_frame.pts` 纳秒值。`mpv_get_time_us` 和 `mpv_get_time_ns` 的 ABI 均需要 handle，见 [client.c](https://github.com/mpv-player/mpv/blob/e470f8986e/player/client.c)。仅按头文件注释使用 us 曾导致更新后不继续呈现，真机帧数暴露该问题。

当前以两种时钟基准的距离判断单位，再统一到微秒 delay；单测覆盖固定 ns、文档 us、过期目标和无目标，实机帧数 / 正常退出也验证。此兼容策略是固定版本适配，升级内核必须重新核对源码与实机时序，不能宣称所有未来版本自动兼容。

## 限制与后续

- RGBA8 为 SDR 输出；没有原生 HDR 显示资格，未专门验证 HDR → SDR 映射。
- 实际 NVDEC 不等于零拷贝；没有 GPU / CPU / 内存功耗与独立 mpv 的 release 基准。
- WASAPI 输出及设备枚举已观察；独占 / bit-perfect / EQ、热插拔安全回退未实现。
- macOS / Wayland / X11 与 Intel / AMD 硬解未验收；这些环境需各自合格运行时、GL 和 UI 验证。
- Windows client API 2 / checksum 加载和本机正常退出通过，不等于长时运行、快速切文件、多屏 / 睡眠 / 设备失联的资格完成。

证据和后续场景见 [Windows 视频报告](../validation/video-windows.md)。参考：[mpv Render API](https://mpv.io/manual/stable/#embedding-into-other-programs-(libmpv))、[Slint 借用纹理](https://docs.slint.dev/latest/docs/rust/slint/struct.BorrowedOpenGLTextureBuilder.html)。
