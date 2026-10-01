# ADR 0002：统一 mpv 音频输出、参数 EQ 与本地歌词

日期：2026-10-01。接受用于 Windows 音频开发预览。USB DAC、位准确输出和其他平台资格另行验证。

## 决定与默认策略

现有视频版本先快进合入 main（4eca722），音频实现继续在 dev。保留同一个 libmpv、Engine worker、AppController 和 PlayerBar，不引入第二套音频时钟 / 引擎。WASAPI 共享为默认，音量保留 70%、上限 100%；EQ、ReplayGain 默认关闭，采样率为自动协商，声道以源布局为起点。不会为“HiFi”强制 192 kHz、浮点设备输出、响度增强或音效。

Windows 固定 AO 为 WASAPI，避免独占初始化失败后换用其他 AO / 设备；关闭 null 输出回退。提供共享、优先独占、严格独占。优先独占只尝试同一请求设备的共享输出，保留失败 / 回退状态；严格失败暂停。可选择独占时保持源采样率，若协商不同则暂停。设备身份变化 / 明确设备失联后暂停，用户通过重新连接按钮恢复。未把请求独占、软件 PCM 或音量 100% 称为 bit-perfect。

## 固定内核的状态证据

依据固定提交 [ao_wasapi.c](https://github.com/mpv-player/mpv/blob/e470f8986e/audio/out/ao_wasapi.c) 与 [ao_wasapi_utils.c](https://github.com/mpv-player/mpv/blob/e470f8986e/audio/out/ao_wasapi_utils.c)。格式接受日志只是提案；必须同时有成功初始化日志和实际 current-ao=wasapi。新初始化会清除旧证据。日志经 client API 事件在 Engine 线程复制，32 项 / 每项 1200 字符上限；只订阅 WASAPI 调试和其他模块警告，不进行全局 debug 日志流。

本机 Realtek 在独占初始化中先遇到缓冲区对齐错误，内核重试后成功。因此不能看到一条 Error 就立即判定失败：成功初始化会清除暂时失败；最终播放失败、运行时设备错误或确认超时才执行回退 / 暂停。确认还需匹配请求模式及具体设备身份；切换输出和独占加载时保持暂停，实际确认后恢复原暂停意图。同设备同模式的健康输出可保留已有证据，重试失败输出会重新加载并保持暂停。WASAPI 的设备选择日志、源解码 PCM、API 输出 PCM 分开呈现；物理 DAC 格式未知。

## EQ、预设与配置

全局、设备绑定、单文件为完整 EqPreset 对象，优先级是单文件 > 设备 > 全局。设备绑定要求具体设备 ID，不能把系统默认身份当成某个 DAC。命名预设可保存 / 删除，删除会解除引用它的设备绑定；单文件为独立副本。

默认十段；可增删至 32 段，编辑频率 / Q / 增益、峰值 / 低架 / 高架与前置增益。导入 YYPlayer JSON 和带 Q 的 APO PK / LS / HS，导出 JSON；拒绝 Include、任意滤镜和无法等价转换的 GraphicEQ。依据 [FFmpeg 参数滤波器](https://www.ffmpeg.org/ffmpeg-filters.html#equalizer) 生成固定名称的 lavfi 链，内部双精度，设备格式仍由协商决定。替换失败恢复旧链；关闭 / 平直移除链，不持续叠加。

自动余量采用所有正增益之和作保守衰减，实际前置增益在界面显示；它不是重建峰值 / 任意输入不削波保证。导入 APO 后可关闭自动余量以使用其原前置增益。没有默认限幅器、直通压缩音频、ASIO、DSD DoP 或原生 DSD 通路。

Settings v1 增加带默认值的 audio 字段，旧设置兼容。路径保持 PathBuf；后台原子保存，临时文件使用独立名称，2MB 保存 / 读取限制一致，错误保留原文件。导入大小、规则数量、有限数值均验证。

## 音乐界面与资源生命周期

真实浏览页隐藏示例专辑，左下封面展开独立大页面，音乐与视频共用播放控件和输出 / EQ 面板。有歌词左右分栏，无歌词封面居中；歌词高亮 / 滚动、进入动画和背景呼吸仅在相关页面运行，暂停后停止持续动画。

后台使用锁定 Lofty 0.25.1 读取标签 / 内嵌封面 / Lyrics 字段；启用 id3v2_compression_support，因为该固定版本关闭此 feature 时不能编译。PNG / JPEG 图片解码限制尺寸 / 内存，缩至最多 1024px，主线程只转换一次 Slint 图片。也读取同名图片及 cover.png / cover.jpg / folder.jpg，mpv audio-display=no 避免重复呈现封面。正常视频仍使用 GPU 合成。

歌词优先手动绑定，再同名 LRC，再内嵌 Lyrics；UTF-8 / UTF-16 / GB18030、重复时间戳、offset 与纯文本可用。请求携带媒体 revision / PathBuf，过期回复不覆盖新歌曲；快速切歌时未提交的读取请求合并成最新一项，纯文本歌词也缓存，避免每次投影复制整份文本。歌词模型按资源 revision 更新，播放只更新活动行；参数表单按设置 revision 更新。网络歌词检索仅 TODO。

## 验证与资格

实际命令、结果和限制见 [音频 Windows 报告](../validation/audio-windows.md)。音频资格不从 UI 样例、请求标志或视频帧数推断；不能把本机 Realtek 结果扩展成所有 USB DAC / 系统、Foobar2000 数字输出对等或跨平台已完成。
