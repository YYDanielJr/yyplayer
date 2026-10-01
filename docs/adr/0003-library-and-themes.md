# ADR 0003：目录音乐库与可扩展语义主题

日期：2026-10-01。Windows 开发预览采用。本轮先创建 master 并快进至音频版本 8467ca9；旧 main 保留在 4eca722。新代码继续在 dev。

## 目录音乐库

目录 / 单文件 / 排除规则属于 core LibrarySettings；索引 Song 采用 PathBuf 身份，标签只用于显示。库与播放队列分离，移除库记录不能删除磁盘文件或打断现有队列。点击曲目将当前筛选结果交给现有 AppController，自动 next 仍由原控制器唯一执行。

独立后台服务使用两项命令、四项回复通道。扫描 revision + 原子取消 generation 拒绝旧回复，待提交扫描合并最新配置；只在资源 revision 变化时投影列表。播放队列和搜索投影缓存为 Rc；媒体 / 标签 / 时长 / 搜索变化才更新，不能每 60ms 重建大目录队列。Lofty 使用锁定 0.25.1 且扫描关闭封面读取。音乐列表使用 Slint ListView 虚拟化，封面完整解码仍由原有资产服务完成。

跳过符号链接和 Windows reparse 目录条目，避免递归循环；每次最多 20000 首、200000 项、64 层。缓存版本 1、上限 32MB，配置限制沿用 2MB。原子替换，读取错误可重新扫描；不可读目录保留旧记录。没有实时目录监听、DB、索引增量更新或完整专辑库，后续按测量推进。

OS 文件选择在后台，手动重新扫描 / 初次导入会读取标签，不在 Slint 主线程读取音频。取消发生在目录 / 文件边界；进行中的本地标签读取或 OS 对话框需要先返回。正式发行前还需网络盘、极大目录和长时测试。

## 主题模型

Appearance 将 Design（Simple / Fashion）、Scheme（System / Light / Dark）、system_accent、custom accent、reduce_motion 分开持久化，旧设置通过 serde 默认兼容。默认时尚 + 跟随系统；不改变任何音频默认值。

Theme global 是唯一语义令牌层：canvas / sidebar / surface / raised / border / text / muted / faint / accent / accent-text / accent-ink / selected，以及 glass / rim / shadow / radius / fast / smooth。页面和组件引用令牌，不能重复实现 OS 探测或硬编码品牌色。两个设计共享内容与交互；简洁保留平实灰黑材质，时尚增加透明分层、浮动侧栏、圆角和高光。颜色为 #RRGGBB 有限格式；按钮前景按相对亮度选取，文字强调色在浅 / 深表面上至少达到 4.5:1 的目标（覆盖当前两套标准背景，不是任意封面像素承诺）。

增加主题时：扩展 core Design 枚举及 controller 的选择映射、AppearanceOptions 选择项和 Theme 中央 profile 分支；页面不改。若引入用户主题文件，需版本化 token schema、大小 / 颜色验证和回退，不执行脚本、shader 或任意路径。当前没有主题导入插件或在线主题商店。

Windows platform Observer 每两秒后台读取 AppsUseLightTheme、DwmGetColorizationColor、SPI_GETCLIENTAREAANIMATION，主线程只取最新小快照。API 依据锁定 windows-sys 0.61.2；只读，无注册表 / 系统偏好修改。macOS / Linux 返回未知及默认回退，适配层保留，未宣称已支持系统自动切换。

## 材质、动画与性能

参考 [Apple macOS 26 设计发布](https://www.apple.com/newsroom/2025/06/apple-introduces-a-delightful-and-elegant-new-software-design/)。采用 Slint 1.17.1 已核对的渐变、alpha、阴影、圆角及短时属性动画；不引入真实 backdrop blur、实时折射、窗口截图循环或新渲染器。正常视频 GL / prepare_gl / 借用纹理边界不变。

动画令牌 140 / 260ms、页面进入、导航 / hover / 按下反馈、侧面板、歌词 / 音乐背景。操作立即执行，减少动态效果将短动画时长置零、停止持续 Timer；暂停 / 非音乐大页面不持续呼吸或旋转。全屏视频显示 / 隐藏仍不改变 FBO 尺寸，原居中控件与浮空提示计时规则保留。性能数字需要 release 实测，视觉截图不能证明功耗提升。

## 验证

见 [目录与主题报告](../validation/library-themes.md)。用户魅蓝小尾巴的独占报告记录为用户实测，不替代数字捕获或格式矩阵。USB / 位准确后续资格仍按 ADR 0002。
