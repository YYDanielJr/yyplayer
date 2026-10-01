# ADR 0005：原生窗口表面、可见行封面和保存通知

日期：2026-10-02。状态：采用，Windows 开发预览。

用户要求窗口圆角 / 阴影、音乐库实际封面、切歌不显示保存设置、精简歌词页并检查 GitHub 准备状态。沿用 ADR 0004 的无原生标题栏与集成控制区，保留原 GPU presenter 和音频策略。

## 决定与边界

- platform 接收借用的 WindowHandle，Windows 使用 DWM 属性 33 请求 ROUND / DONOTROUND，返回 HRESULT 与回读值。app adapter 在主线程通过锁定 Winit 0.30.13 的 `set_undecorated_shadow` 开启系统阴影，仅窗口化启用；最大化 / 全屏退出恢复。样式只在状态变化时调用。非 Windows 无操作，不为圆角改成 layered window、透明 CPU 合成或创建窗口 region。
- 原生圆角是系统策略提示，不能保证所有环境；Windows 10、贴靠、VM / 远程等需实机补验。相关依据：[Microsoft 圆角说明](https://learn.microsoft.com/en-us/windows/apps/desktop/modernize/ui/apply-rounded-corners)。系统阴影强度 / 半径由 DWM 控制。回读属性本身不能证明外观，原生裁片另行查看。
- 音乐库视口在 Slint 内输出起始行 / 数量，app 定时投递最多 64 行，队列 / 在途各最多 8。单个后台 worker 读取标签和图片，缩小到最长边 64px；UI 线程仅创建 Slint image。LRU 最多 256 项，缺失封面也缓存；重扫取消旧 generation、清空缓存。原始 RGBA 最大约 4 MiB，不包含 renderer 缓存。加载结果按批更新库 revision，不在每个 tick 读取或重建。
- 优先内嵌正面封面 / 首图，回退同名 JPG / PNG、cover.jpg / cover.png / folder.jpg。编码图片最多 16 MiB、尺寸最多 8192、解码预算 64 MiB；损坏或无图显示中性图标。大页面继续使用既有独立 1024px 资源，防止缩略图放大影响观感。
- 最近记录与设置 dirty 分离。持久化 worker 对比排除 recent 的配置对象，只有成功、请求通知且设置确实不同才提示成功。通知 revision 与磁盘请求 revision 分离，静默保存不能恢复已关闭浮窗；失败仍可见。配置格式和原子替换规则不变。
- 左下封面切换音乐库 / 大页面，大封面返回音乐库。删除“收起 / 自动跟随 / 更换歌词”按钮，保持自动跟随与逐行 seek，手动导入留在播放选项。没有歌词保持居中封面。

## 验证与后续

见 [验收报告](../validation/window-artwork.md)。真实 controller / Engine / GPU 窗口、保存与图片边界测试均通过。跨屏 DPI、实际鼠标拖动和 macOS / Linux 原生表面仍待验；本轮不改变音频输出 / EQ / 解码，也不重跑 USB 数字捕获或 HDR 资格。
