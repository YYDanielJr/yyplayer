# Windows 音频开发预览验证

日期：2026-10-01。范围：本轮音频功能实现和本机 Windows 验证。main 已合入 4eca722，音频继续在 dev。

## 实现与操作

- 打开一个或多个本地音频，音乐浏览页显示真实队列；点击左下封面展开，左上“收起”返回。内嵌 / 本地封面，标签标题 / 歌手 / 专辑和本地歌词已接入。
- 播放栏右侧设置按钮、浏览页“播放选项”、大页面“音频信息与输出 / EQ”均打开同一面板。上方选择设备、查看媒体信息、选择共享 / 优先独占 / 严格独占，重试输出；源采样率严格策略可单独勾选。
- 展开 EQ 编辑器后导入、手动编辑、保存命名预设 / 导出。选择全局、当前具体设备、当前文件范围并应用；单文件优先。清除覆盖回到下一级。视频页也应用这套链。
- 同文件名 .lrc 自动读取；手动 LRC / TXT 绑定到媒体原始 PathBuf。支持内嵌 Lyrics、时间偏移、自动跟随 / 自由浏览、点击定时歌词 seek。网络搜索 TODO。

## 环境、素材和实际结果

Windows 11、Rust 1.97.0、Slint 1.17.1、固定 mpv v0.41.0-1087-ge470f8986、RTX 5060 Laptop，桌面 125%。音频设备枚举包括 Realtek、Waves SoundGrid 与 NVIDIA HDMI。测试音量 35%，自有 32 秒 FLAC：1kHz、44.1kHz、双声道、24 bit、内嵌 PNG 和六行同名 LRC；不使用个人歌曲或配置。

`scripts/Test-Audio.ps1` 使用 target/audio-validation 内独立配置并恢复调用进程环境。共享 / 严格独占实际初始化成功；Realtek 共享提交 48kHz float，独占提交 44.1kHz s32。文件 24bit 标签与解码 / API PCM 容器位宽分开，不推断物理 DAC 位宽。

控制脚本七阶段通过：展开、全局增益 4.5dB、命名保存、选具体设备绑定、文件增益 -6dB 覆盖、清除回到设备、独占切换、回浏览、清除设备并全局平直。最终 AF 链为空；预设保存成功，封面 / 歌手 / 专辑 / 六行歌词存在，正常退出。脚本执行生产 AppController；它不代替物理文件对话框、鼠标或听音验收。

严格源采样率用自有 768kHz FLAC：Realtek 协商 192kHz，程序实际暂停并返回源采样率不兼容错误，没有把协商称为原格式输出。

`scripts/Test-AudioBusy.ps1` 用三个自有进程争用具体 Realtek：持有者保持独占正常播放；严格实例拒绝，未回退；优先实例尝试同设备共享，而设备仍被独占时明确失败，未改到扬声器 / 其他设备。所有实例正常退出。该测试证明受控尝试和双失败路径，尚未证明某个“不支持独占但允许共享”的实际设备上的成功回退。

原始报告、日志、配置和 fixture hash 保存在已忽略的 target/audio-validation。测试素材生成包含封面，实际 hash 以每次脚本输出为准。最终 Release 测试主 FLAC 的 SHA256 为 `4c871b0e219fc326750de882dafc2a7626610c53c5e55e81f622a0340d868b75`。软件截图是开发样例，有显式“播放状态为示例”文字；仅审查布局，不能证明真实独占或音频效果。

## 检查命令

```powershell
cargo fmt --all -- --check
cargo clippy --locked --workspace --all-targets --offline -- -D warnings
cargo test --locked --workspace --all-targets --offline
cargo build --locked -p yyplayer-app --offline
cargo build --locked -p yyplayer-app --release --offline
powershell -ExecutionPolicy Bypass -File scripts/Test-Audio.ps1
powershell -ExecutionPolicy Bypass -File scripts/Test-AudioBusy.ps1
cargo run --locked -p yyplayer-app --example audio-ui --offline
cargo run --locked -p yyplayer-app --example ui-feedback --offline
powershell -ExecutionPolicy Bypass -File scripts/Test-Video.ps1
powershell -ExecutionPolicy Bypass -File scripts/Test-Render.ps1
```

格式 / 严格 clippy / workspace all-targets tests、Debug / Release 构建通过（13 个独立测试）。最终 Release 成品再次通过 Test-Audio 和 Test-AudioBusy；最终 Debug 成品再次通过 Test-Video 和 audio-ui。独立边界测试覆盖 EQ 参数 / 输入格式 / 回退规则优先级、旧设置默认值、极端 offset、LRC 重复时间戳 / seek、UTF-16 和文件大小、WASAPI 暂时错误 / 成功证据 / 请求模式及设备身份 / 设备变更；原快捷键、原子保存、render 时钟测试也通过。

audio-ui 实际 Slint 点击通过封面展开、收起与歌词 seek，审查常规有歌词 / 无歌词、1000×640、小窗口加面板五张图片。ui-feedback 的 17 个计时 / 点击检查再次通过。视频 11 动作、暂停位置重载和持久化复验通过。4K HEVC GPU 对照再次通过：硬解 vs 软件平均 RGB 差 0，去色带 vs 软件 0.205388，明显偏差比例均 0%；未改变 GL 状态交接。Release 实际音频输出与竞争结果和上述一致。

## 未验证条件与下一步

USB DAC / 蓝牙 / 多声道设备，广泛 16 / 24 / 32bit 与 44.1 / 48 / 96 / 192kHz 格式矩阵、物理拔插 / 默认端点变化 / 睡眠恢复仍需实机。端点身份变更和设备失联的暂停逻辑及单测已存在，不能当作真实拔插验收。

未做与 Foobar2000 相同配置的数字输出捕获、位准确对比、模拟听感 AB、长时性能测量；独占、源率一致、无 EQ 不能证明 bit-perfect。默认策略避免无必要处理，但不声称已证明所有设备默认音质与 Foobar2000 对等。ASIO、压缩直通、DSD DoP / native DSD、真正 gapless playlist 未实现；文件格式解析支持范围不等于资格矩阵。

预设导入 / 导出与手动歌词对话框代码已实现，解析 / 保存边界检查通过；OS 对话框的完整人工操作还需用户真机验收。macOS / Linux 音频独占返回未验证提示，默认共享入口 / cfg 保留，未编译或真机验收。下一步先对 USB DAC 做格式、共享 / 独占竞争、设备失联和成功回退验证，再做数字捕获与输出对比。

## 用户补充设备证据（2026-10-01）

用户实际连接魅蓝 DSP 小尾巴，报告 WASAPI 输出已确认独占。记录为用户实测：具体硬件修订 / 固件 / 源格式 / API 格式 / 系统驱动和数字捕获未提供，不能扩展为所有 USB DAC 或 bit-perfect 已通过。后续 USB 格式、竞争、拔插和数字捕获矩阵仍需补齐。
