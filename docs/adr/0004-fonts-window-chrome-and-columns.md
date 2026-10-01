# ADR 0004：字体、集成窗口控制栏与音乐库列

日期：2026-10-01。范围：Windows 开发预览，继续使用 Rust / Slint 1.17.1 / 固定 libmpv。

## 背景与事实

用户希望分别选择界面、歌词、字幕字体，取消与应用主题割裂的 Windows 原生标题栏，改善音乐库元数据排列与小按钮对齐。此前歌曲列占满剩余空间，专辑靠近窗口最右边；文字按钮的字形基线还会影响 − / × 的视觉居中。

Slint 锁定版本提供 Window 的 default-font-family / no-frame 和 winit_030 窗口访问。mpv 的 sub-font、sub-ass-style-overrides、sub-ass-override 可以在 Engine 线程设置；不能在 Slint 的视频 Image 上单独改变字幕字体。相关语义见 [mpv 官方手册](https://mpv.io/manual/master/#options-sub-font) 和 [ASS 样式覆盖](https://mpv.io/manual/master/#options-sub-ass-style-overrides)，固定 runtime 的接受与渲染另做本机验证。

## 候选与决定

1. 保留原生装饰、只改内容，或自行绘制统一主题窗口控制区。选择后者：36px 主题控制栏，右置 Windows / Linux 按钮，平台投影可切换左置红黄绿按钮。拖动、双击最大化、八方向边缘缩放使用 Winit 原生动作，不用持续修改窗口坐标模拟拖动。不引入 HWND 子窗口或第二套 UI 框架。
2. 扫描字体文件、读取字体注册表，或使用 Windows 字体枚举 API。Windows 选择 GDI EnumFontFamiliesExW，在单次后台任务中取得字体族，不在主线程扫描文件。非 Windows 使用已有依赖 fontdb 的系统字体加载入口，保留适配空间；未完成跨平台资格。
3. 固定像素列宽或持久化比例。选择歌曲 / 歌手比例，专辑占剩余空间，时长 / 移除动作固定；两条表头分隔线可拖动，双击还原。窄窗口隐藏行封面、保留歌曲信息并截断长文本。

## 实现边界

- core 的 Fonts / LibraryColumns 是与 UI、OS、mpv 无关的配置；旧 version=1 设置以 serde 默认字段兼容。名称限制 128 字符，并拒绝控制符、逗号、等号和反斜线，避免拼入 ASS 样式语法。列比例验证有限值及边界。
- 全局默认为 Slint 系统字体；歌词默认跟随全局；字幕默认 sans-serif。缺失的已配置字体有可见提示、临时回退，不抹掉用户原选择。刷新字体后重新解析。当前实现选择字体族，不调整字号或加载外部字体文件。
- 字体服务一次最多 4096 族、一个后台任务 / 一个容量为 1 的回复通道；UI 只投影 revision。目录未变化不重建下拉模型，防止用户选择被框架重新初始化。
- 字幕命令由普通 Engine worker 执行，三项 mpv 属性逐项检查；失败尝试恢复全部旧值。Snapshot 显示实际回读属性。SRT / 文本字幕使用所选字体；默认保留 ASS / SSA 作者字体，可显式只覆盖 FontName。嵌入的行内字体指令等复杂排版、缺字回退仍受 libass 语义限制；图片 / 烧录字幕无法换字体。
- 视频控制栏覆盖画面，窗口状态下自绘控制区和播放顶部工具栏一同自动隐藏，全屏不显示窗口控制区。隐藏不缩放视频 / FBO。不改变 GL prepare_gl、渲染租约或普通 mpv 调用线程。
- 减号和关闭使用对称 SVG 与统一 IconButton；列表尾部保留 24px 滚动条空间，图标按按钮宽高居中。
- 关闭走正常事件循环退出、保存服务和 GL / Engine 清理。最小化 / 最大化通过 UI 主线程执行；原有快捷键和窗口模式设置继续沿用 controller。

## 验证与影响

新增 Test-UiCustomization / ui-customization，使用自有 80 首 WAV、LRC、30 秒视频及 SRT / ASS，独立配置；验证真实字体目录、Slint 下拉 / 列拖动 / 移除、原子保存、歌词和字幕、Windows 窗口动作，并审查 GPU 截图。既有 ui-feedback / navigation-ui 保留回归。

本机结果及截图见 [验收记录](../validation/ui-customization.md)。Windows 物理鼠标拖动 / 八方向缩放、多屏 DPI / Snap，以及 macOS 原生交通灯、全屏 Space 和 Linux 窗口管理器需要单独人工验证；左置布局的 Windows 模拟不能替代这些资格。没有实现 Windows 最大化按钮悬停的系统 Snap 菜单或 macOS 原生交通灯 API。
