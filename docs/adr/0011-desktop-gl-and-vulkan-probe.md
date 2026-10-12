# ADR 0011：Windows 桌面 GL 与 Vulkan 去色带样机

日期：2026-10-12。接受用于 Windows 开发预览；AMD 独显、其他 GPU / 驱动、Linux / macOS 本轮新资格待验。

用户最终确认：继续使用 OpenGL，停止 Vulkan 探索。0.5.1 保留已验证的桌面 GL 路径，下列 Vulkan 结果仅作为已完成调查的历史证据。

## 证据

用户要求修复 AMD 去色带并探索 Vulkan。固定 Slint / slint-build 1.17.1、FemtoVG 0.25.1、mpv `e470f8986e` / DLL hash 保持。前轮自有 4K HEVC 的 deband=true 在 Radeon 860M 上为蓝色画面，0.4.1 对照同样失败。

增加仅 Debug、显式 `YYPLAYER_MPV_LOG` 启用的 Engine 初始化日志。实际上下文是 `OpenGL ES 3.2`，AMD shader 编译日志拒绝 mpv 内部的 `average` 函数名（redeclaration of existing name）；两个去色带 fragment shader 链接失败，普通 render 调用 / 帧数仍显示成功。没有证据把责任扩大为全部 AMD、某个硬解器或 OpenGL 家族本身。

固定 [Slint 默认 FemtoVG 上下文选择](https://github.com/slint-ui/slint/blob/v1.17.1/internal/backends/winit/renderer/femtovg/glcontext.rs)优先 GLES。官方 [BackendSelector 1.17.1](https://docs.rs/slint/1.17.1/slint/struct.BackendSelector.html#method.require_opengl_with_version)允许请求桌面 GL；实际请求 3.3 后，同一 Radeon / 驱动 / runtime 创建 `3.3.0 Core Profile`，shader 编译成功，4K 静态图像比较和暂停重载通过。完整结果见 [验证](../validation/compact-toolbar-amd-deband.md)。

## 决定

- 生产与真实 GPU 例子共用 `player_ui::select_gpu_backend`。Windows 明确请求桌面 OpenGL 3.3，presenter 在同上下文检查实际桌面类型 / 版本；若 Slint / glutin 降级为 GLES 或低版本，视频给出可见错误，不能继续显示成功。Linux / macOS 选择保持原样。
- 仍用 GL Render API → RGBA8 FBO → Slint 借用纹理。保留 prepare_gl、GL 串行、render / Engine 线程分离、先进 renderer 清理再退出 core；不添加每帧 CPU 回读、glFinish、第二套引擎或全局 runtime。
- 不关闭去色带，不更换固定 DLL，不强制软解，不更改图像阈值。默认去色带仍关闭，开启后也必须正确。Windows 的驱动最低能力明确为桌面 GL 3.3。
- Test-Render 补实际 GL 版本 / shader 日志检查；deband 例子使用真实 controller 验证暂停去色带 / 解码重载、位置和释放 / 继续观看，再做 GPU 像素比较。日志为开发选择，不进入普通 Release 播放路径。

## Vulkan 探索

同一固定 DLL 的原生 `vo=gpu` + Vulkan / OpenGL、`vo=gpu-next` + Vulkan，指定 Radeon 860M、自有 4K 输入，在 deband 开 / 关时均实际播放和 GPU 截图，像素对照通过。样机输出截图 480×270，不能宣称已验证 4K Vulkan 输出性能 / HDR / 零拷贝；实际硬解是 d3d11va-copy。

但固定 [render.h](https://github.com/mpv-player/mpv/blob/e470f8986e/include/mpv/render.h)只公开 OpenGL / 软件后端；直接创建 Vulkan render context 实测 `-19 / operation not implemented`。Slint 当前 [BorrowedOpenGLTextureBuilder](https://docs.rs/slint/1.17.1/slint/struct.BorrowedOpenGLTextureBuilder.html)也要求来自同一 GL 上下文的纹理。

因此 Vulkan 原生输出可以避开该 GLES shader 编译路径，生产集成不能只把 gpu-api 改为 vulkan。更换 Slint 的渲染器本身不会让固定 mpv Render API 获得 Vulkan 后端。本轮保留已验证的桌面 GL；不将原生样机写成整个应用的 Vulkan 支持或性能优越结论。调查至此结束，Vulkan 子窗口或 GPU 图像导入 / 同步方案不再列为接续任务。
