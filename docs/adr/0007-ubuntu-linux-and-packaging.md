# ADR 0007：Ubuntu Linux 输出、桌面边界与打包

日期：2026-10-03。状态：`dev-linux` 开发预览采用；实际组合与证据见 [Linux 验收](../validation/linux.md)。

## 背景

用户已在 Windows 完成当前功能，要求新建 dev-linux，适配 Ubuntu 26.04，保留当前 Windows 功能并生成 deb / AppImage，先一次性审计 apt 需求。本轮从 master 的 `6e9e9c42e2d7` 新建分支，复用五个 crate、AppController、后台服务和 GPU presenter。没有重新创建 UI、引入第二套音频内核或改动 Windows 默认策略。

## Linux 播放运行时

deb / 开发构建使用 Ubuntu 安全维护的 `libmpv2` 绝对系统路径；本轮实际为 0.41.0-2ubuntu4、client API 2。固定 Windows DLL 的 hash 不适用于 Linux 发行包。AppImage 则把本次实际 libmpv 和非驱动 ELF 闭包打入包，用生成的 manifest / SHA-256 核验。包旁存在 runtime 却缺文件、清单错误或 hash 不符时拒绝初始化，不偷偷绕过到系统库。显式 `YYPLAYER_MPV_LIBRARY` 仍是开发覆盖入口。

上游 / 发行资料：[mpv v0.41.0](https://github.com/mpv-player/mpv/tree/v0.41.0)、[Ubuntu 对应源码](https://launchpad.net/ubuntu/+source/mpv/0.41.0-2ubuntu4)。动态 API 绑定不变；Required 初始化参数继续检查返回值，启动 / 普通调用只在 Engine worker。

## 输出与独占

共享模式保留自动协商、70% 音量和无额外 DSP。mpv 使用自动 AO 优先顺序，本机默认为 PipeWire。不要同时强制 `ao` 列表：实测 `ao=pipewire,pulse,alsa` 会覆盖 `audio-device=alsa/hw:...` 的后端，错误地创建 PipeWire 流。移除该强制后，具体 PipeWire / Pulse / ALSA 设备分别使用自己的实际 AO，验收检查真实 `current-ao`。行为依据 [锁定版本文档](https://github.com/mpv-player/mpv/blob/v0.41.0/DOCS/man/options.rst) 的 audio-device / ao 规则。

PipeWire 独占根据 `PW_STREAM_FLAG_EXCLUSIVE` 请求、成功连接日志、实际 AO 和输出 PCM 分别确认；它不等于绕过桌面采样 / 混音，也不等于硬件 DAC 独占。ALSA `hw:CARD=<稳定ID>,DEV=n` 从 `/proc/asound` 在 Engine 枚举，成功打开同一 hw 设备、Final HW params 与实际 AO 才确认硬件占用；`plughw:` / `dmix:` 不作为 hw 独占证据。依据 [ao_pipewire.c](https://github.com/mpv-player/mpv/blob/v0.41.0/audio/out/ao_pipewire.c) / [ao_alsa.c](https://github.com/mpv-player/mpv/blob/v0.41.0/audio/out/ao_alsa.c)，不同 AO 不复用 WASAPI 日志确认协议。

严格独占失败暂停；优先独占可回退同一个具体 PipeWire 节点或明确的 Pulse 共享设备，回退保持可见。自动设备独占被拒绝，ALSA hw 失败不猜测桌面 sink，不突然切到扬声器。设备失联暂停；恢复输出保留媒体位置和暂停。修改模式重新初始化一次，在确认之前不播放。源 PCM / API 输出 PCM / 未知物理 DAC 格式继续分开；源率保护与 DSP 优先级 / 平直移除链保持。

启动参数打开媒体发生在第一个 UI tick 之前。Linux 首次 Load 前按命令顺序提交保存的音量 / 输出策略，Engine 在处理命令前枚举实际设备。保存设备不可用时拒绝 Load / Resume，不先初始化默认输出；快照的 audio_blocked 表示与媒体身份无关的输出阻止状态，界面不能因 pending_load 过滤掉这个错误。实际 AO 与请求后端不一致时不确认独占。

## 桌面、原始路径与 GL

Winit + FemtoVG 原生 Wayland / X11，不把 XWayland 或软件截图作为原生 GPU 资格。设定 XDG app id / 桌面图标 / MIME，现有有界 rfd 对话框通过 portal 运行，字体仍在后台枚举；设置仍按 XDG 保存。外观 Observer 后台只读 Settings portal 的色彩、强调色、减少动画，并保留缺接口回退，不写系统主题。

Unix 非 UTF-8 路径保留字节身份。JSON 的特殊编码使用实际路径不可能含有的 NUL 标记，避免与任何已有合法 UTF-8 文件名碰撞；普通旧配置仍为相同字符串。mpv 输入 file URI 还原字节，FILE_LOADED 使用原始 C 字节比较，截图目标也直接传原始字节，不用显示文本识别文件。配置与索引的原子写入 / 容量边界不变。

GL 生产路径保留 prepare_gl、同上下文串行 render、GPU 借用纹理和按需 render 租约。Wayland 隐藏窗口时 Slint 持有可变 backend borrow；原 teardown 回调修改 UI 图片会触发重入。UiShell 在 event loop 返回后、hide 之前清除 UI 图片引用，RenderingTeardown 只清 render / GL，不设置 UI 属性；先 render_free 再释放 core。导航 / 视频库验收等待实际透明度完成，有明确超时，不能因同步 GPU 截图延迟当次 timer 回调而误判。

自绘窗口控制动作仍由 Winit 执行。Linux 圆角 / 阴影由 compositor 决定，不移植 DWM 属性、透明分层窗或 CPU 视频循环。Wayland 不提供可查询最小化状态或客户端还原；自动验收不伪造确认，物理最小化 / dock 恢复独立记待验。

## 包与发行边界

deb 依赖系统 libmpv 和桌面运行库，无删除用户配置 / 媒体的 maintainer script。AppImage 携带 libmpv、FFmpeg、libplacebo、音频客户端 / SPA 插件及依赖，宿主保留 glibc、GPU 驱动 / GL / Vulkan、字体、portal、PipeWire / Pulse 服务。目标限 Ubuntu 26.04 amd64，未宣称老发行版兼容。

appimagetool 1.9.1 的归档与 runtime 源码 SHA-256 固定，每次重新提取已核验工具，复用其内嵌 caf24f9 runtime，不下载可变 continuous runtime。打包输出源码工作树、锁定 crate 原始源码与可离线准备的 vendor 脚本、完整 Rust notices、Ubuntu copyright / 精确 source package 页面、构建信息和总校验和。上游预构建 runtime 的 Alpine 静态包精确闭包 / 可重链接安排仍需补证，保留 [完整记录](../../packaging/linux/runtime-notices/README.md)，本轮不宣称正式发行许可资格。

新增 Linux Actions workflow 使用 Ubuntu 26.04 容器，只生成短期 artifact；本轮不推送、不创建 GitHub Release。真实安装 / 升级 / 卸载和首次文件关联在干净系统继续验收。USB DAC / 数字捕获、HDR 输出、AMD / Intel、其他 compositor、多屏 / 睡眠与长时性能仍按报告补资格。
