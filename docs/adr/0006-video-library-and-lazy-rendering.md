# ADR 0006：视频库、同窗口播放器与按需 render context

日期：2026-10-02。状态：Windows 开发预览采用，验收结果见 [视频库验收](../validation/video-library.md)。

## 背景与决定

用户要求视频剧场成为可添加目录 / 单文件的视频列表，点击后才进入播放器，并只在视频播放启动时初始化视频渲染器。保留 Rust / Slint / libmpv、同一 AppController 和单个 Engine；抽出 VideoPlayer 播放表面与 VideoPlayerToolbar，视频库独立于播放器页。暂不新增窗口：避免重复 UI / GL 上下文与 GPU 缓存、跨窗口借用纹理和两套设备所有权；仍支持现有最大化 / 全屏。

导航页 5 为视频库，页 1 为播放器；导航不能隐式创建播放器。视频配置 video_library 与音乐 library 独立，通过 serde 默认兼容已有 version=1 配置。settings.videos.json 是独立、版本化的索引缓存。共用有界目录扫描服务（2 命令 / 4 回复），以独立 generation / revision 取消 / 丢弃过期结果；20000 个媒体 / 200000 项 / 64 层、32MB 缓存上限不变。后台规范化 PathBuf，跳过链接 / reparse 条目，失联目录保留缓存；移除记录不删除文件、不修改已建立的队列。

视频索引只读文件名、所在目录和文件大小，不为扫描启动 decoder / render context，也不虚构时长 / 封面。候选扩展名负责筛选；实际音视频轨由 libmpv 确认。显示 ListView 虚拟化，搜索 / 筛选 / 扫描 revision 才重建投影，无目录监听 / DB / 视频缩略图。

## 播放与生命周期

core Load 带 video 输出意图。Engine 启动设 vid=no，音频 Load 不等待 render 租约。视频 Load 等待 RenderBridge ready 后才启用 vid=auto；VideoGate 合并最新等待请求，音频、Stop、关闭视频输出清除旧等待加载，避免后来误播。扩展名判为音频但实际存在视频轨时，controller 根据真实快照请求 render，再由 Engine 启用视频输出。

Slint notifier 仅在 controller 请求播放或 Engine 尚持有实际视频输出时创建 / 保留 render context；创建成功计数可诊断。普通 mpv 调用仍只在 Engine；notifier 保留 prepare_gl、同上下文串行 render、旧纹理本帧后回收与先 render_free 后 core 销毁。窗口 UI 本身仍用 FemtoVG / OpenGL，按需的是 libmpv 视频 renderer，不宣称音乐页完全不使用 GPU。

返回库先保存当前媒体 / 位置、恢复临时倍速、退出全屏，再提交 Stop。Engine 真正退出 VO 后回报，notifier 才清除 Slint 图片引用并释放 render context / FBO / texture；继续观看或再次点击同项重新 load 并从保存位置恢复。不能只用 Pause + vid=no：无音轨视频关掉唯一轨后会变为 Ended，已在本轮真机暴露；使用独立保存位置的停止 / 恢复流程保证视频有 / 无音轨行为一致。此位置仅保存当前运行会话，未新增跨重启的视频续播。

Engine 命令通道保留 64 个普通命令和 1 个 Stop 专用槽；停止不等待、不丢弃已排队的设置 / 临时倍速恢复，重复 Stop 在专用槽已满时合并，Shutdown 仍走 atomic。播放器初始化失败可恢复，取消未完成加载；未知格式 / 解码失败保留可见错误。不更改解码 / 去色带默认值、时钟 ABI、共享音频默认或用户 APPDATA 测试配置。

## 验证与边界

固定 mpv e470f8986e 的 include/mpv/render.h 要求在 VO 创建前初始化 render context；render_free 会关闭视频，且必须先于 core 销毁。实现延续这些约束。验证用自有素材与独立配置，覆盖真实 Slint 列表 / 返回点击、空页 / 最小尺寸、音频 0 次创建、视频反复创建 / 释放 / 恢复、快速取消、索引隔离 / 移除 / 重启恢复，以及 4K HEVC NVDEC / 软件 / 去色带 GPU 图片比较。具体通过项以验收报告为准。

Windows 其他设备 / 驱动、多屏 DPI、真实文件对话框与超大 / 网络目录仍需额外资格；没有改变 USB 位准确 / HDR / 跨平台状态。内存数字需同 Release、素材、设置与口径比较，不能以截图证明节能或零拷贝。
