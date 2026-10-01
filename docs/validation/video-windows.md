# Windows 视频开发预览验证

日期：2026-09-30–2026-10-01。范围：本轮真实视频功能，开发 / release 运行；不等于正式发行资格或性能基准。2026-10-01 用户反馈暴露共享 GL 状态缺陷，已修复并重跑控制脚本、更新本页 debug 数据 / UI 图片；新的画质证据与对旧验收遗漏的说明见 [渲染修复报告](render-state-fix.md)。

## 环境与固定素材

- Windows 11 Pro，10.0.26300，x86_64 MSVC；Rust / Cargo 1.97.0，Slint 1.17.1。
- NVIDIA RTX 5060 Laptop，驱动 32.0.16.1088；同时存在 AMD 860M 32.0.31041.1004 和 MuMu 虚拟显示。报告里的 NVDEC 是实际运行路径；没有据此声称 AMD / 虚拟显示合格。
- 默认窗口 1240 × 900 逻辑像素，缩放 175%，[真实 UI 截图](../video-ui.png) 为 2170 × 1575。使用实际 FemtoVG / GL notifier 的一次开发截图，非软件 UI 预览。
- 固定 runtime `20260928-git-e470f8986e`，实际 `mpv v0.41.0-1087-ge470f8986`、client API 2；archive SHA256 `81795d759e01016f1550fd71651a1a5d59ab5c28ef31c0b6793224e9cff39459`；DLL SHA256 `0d5b9dbecb73e179ef39dc94314ff8905fab926a80f8d60a2367d824b7254e2e`。manifest / 获取脚本 / loader 实际使用这些值。
- 自行生成的 `testsrc2` + 440Hz sine：24 秒、H.264 / AAC、1280 × 720、30fps、48kHz 单声道。SHA256 `05aeee9d532c8b1f68150f1c9782cbab64ca44bb8f96c76d31fbc4e83a99bc59`。文件不提交；脚本给出生成命令。不同 FFmpeg 构建可能生成不同字节，重新跑时记录新 hash。
- 所有实机脚本使用 `YYPLAYER_CONFIG` 独立 JSON，未写用户默认 APPDATA 设置。

## 实际检查与结果

| 执行 | 结果 / 限定 |
| --- | --- |
| `cargo test --locked --workspace --all-targets --offline` | 通过：5 个独立测试，services 测试在 ui-preview target 复用，因此共 6 次执行。覆盖规则优先级 / 相邻目录排除、短按 / 长按 / repeat / cancel、键位冲突 / 非法范围、原子替换 / 坏配置、render ns/us 时钟。 |
| `cargo clippy --locked --workspace --all-targets --offline -- -D warnings` | 修复指出的问题后通过。 |
| `cargo build --locked -p yyplayer-app --offline` | Debug 通过。 |
| `cargo build --locked --release -p yyplayer-app --offline` | Release 通过；实际执行成品并验证退出。 |
| `scripts/Test-Video.ps1 -SkipBuild` | 通过；走生产 controller、engine、native window 与 GPU，并检查真实诊断值 / 保存文件。 |

[controls-playback.json](controls-playback.json)：14 秒运行，11 个控制动作，实际 NVDEC → 软件解码；最终 Playing、13.500 秒、251 次 render、速度 1x、音量 35、静音 true、无 engine / render 错误，进程 exit 0。关键状态：

- 3 秒记录实际 1.5x；4 秒实际 Paused；5 秒恢复 1x，6 秒记录相对跳转后的进度。
- 7 秒最大化，8 秒全屏，9 秒退出全屏恢复最大化。
- 9.5 秒暂停，10.5 秒保存单文件 Software 规则并重载；10、11、12 秒均 Paused、位置 **11.966667 秒**；11 / 12 秒 `hwdec-current=no`。12.5 秒恢复并继续前进，规则在隔离设置文件中实际存在。
- 枚举 4 个设备：自动、Waves SoundGrid、Realtek、OpenAL；初始视频实际 `nvdec`、音频 API `wasapi`。这是枚举 / 输出状态证据，不是独占、各端点切换、听感或物理格式验收。

[software-playback.json](software-playback.json)：从启动设置强制 Software，7 秒运行，实际 no、Playing、4.633 秒、141 次 render、音量 70、无错误、exit 0。

[release-playback.json](release-playback.json)：release 普通自动策略，7 秒运行，实际 NVDEC、Playing、6.333 秒、192 次 render、音量 70、无错误、exit 0。不同运行启动耗时不同，不能把这些帧数当作跨方案性能对比。

GPU 截图人工对照原始 FFmpeg 源帧，修复初始 flip 导致的上下倒置；源帧的时间码现在位于左上，进度条从左端填充并与滑块一致。正常路径没有每帧 CPU 回读；截图的开发开关在 release 不存在。

## 验证方法与重跑

```powershell
powershell -ExecutionPolicy Bypass -File scripts/Get-Mpv.ps1
powershell -ExecutionPolicy Bypass -File scripts/Test-Video.ps1
```

第二条需要 ffmpeg。可用 `-Ffmpeg C:\path\ffmpeg.exe`，或已有构建后 `-SkipBuild`。Windows Start-Process 以 Hidden 启动 helper，不改变系统音频默认设备；播放器渲染窗口可能仍短暂可见。脚本最多等待 60 秒，异常退出 / 错误 / 缺少帧 / 状态不符会失败。环境变量在 finally 恢复；生成的配置 / 视频 / PPM 被 Git 忽略。

诊断开关：`YYPLAYER_SMOKE_SECONDS` 自动正常退出，`YYPLAYER_SMOKE_SCRIPT` 使用确定性控件脚本，`YYPLAYER_DIAGNOSTICS` 写 JSON；`YYPLAYER_UI_CAPTURE` 在 debug 下保存一次 PPM，`YYPLAYER_RENDER_TRACE` 在 debug 下最多记录 1000 次 render 时序。只有显式设置时启用，日常用户不需要它们。

## 实现但尚未逐项真机验收

文件对话框 / 拖放、物理输出端点切换、多音轨切换、字幕格式 / 延迟、章节、逐帧、视频截图、A–B / 单文件循环、比例切换、实际键盘事件重映射等已接到生产命令 / UI，未逐项进行 OS UI 自动化或完整素材矩阵。脚本控制动作不能替代这些人工步骤，状态机单测也不能替代系统键盘验收。

后续按以下顺序补证据：

1. 打开含中文 / 空格的多文件、拖放 / 连续换文件、EOF 自动下一项与最后一项重播，损坏 / 不存在文件错误恢复。
2. 实际按 Up/Down、短 Left/Right、长按后松开 / 松修饰键 / 切窗口；改键和重复键校验；编辑输入框不抢键，关闭窗口不残留临时速度。
3. 含多音轨、SRT / ASS / 内嵌字幕、章节的本地合法素材：切换 / 关闭 / 延迟 / 逐帧 / PNG 截图 / A–B / 单循环。
4. 分别修改全局、父文件夹、子文件夹、单文件；重启后确认优先级，清除覆盖后确认继承；同名相似目录不被误匹配。
5. 切换 Realtek / USB DAC，移除设备、睡眠恢复、多个显示器 / DPI、缩放窗口 / 连续进出全屏和持续播放。
6. release CPU / GPU / 内存 / 功耗基准、HDR → SDR / 原生路径专项、HEVC / AV1 / 10bit / 高分辨率、Intel / AMD、macOS / Wayland / X11。

当前没有独占 / EQ、HDR 输出、gapless、续播数据库、发行依赖闭包 / 许可审计证据。不要将上述未测项或规划表标 Pass。
