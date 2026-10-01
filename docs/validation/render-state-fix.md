# NVDEC 横向噪点：OpenGL 状态交接修复

日期：2026-10-01，dev 分支。问题：用户的 4K HEVC 视频，在自动硬解且“去色带”关闭时出现大量横向彩色噪点，勾选后正常。

## 复现与原因

原文件只读探测：3840 × 2160，HEVC Main / 8-bit yuv420p，30000/1001 fps，BT.709 SDR，AAC / 48kHz / 双声道，30.805 秒，78,981,236 字节。SHA256 `6e3b42a1cfc0b76587ff1694c48e8610823cb721d2f506e6f8bdb934144e836e`。媒体、个人配置和截图均未提交；诊断产物仅在本地 target/render-regression。

与此前同一 Windows / RTX 5060 Laptop / Slint 1.17.1 / mpv e470f8986e 环境，使用独立、静音配置复现：

| 修复前 | 实际 decoder | 结果 |
| --- | --- | --- |
| 自动，去色带关闭 | nvdec | 复现横向噪点；engine / render 错误为空，帧数仍正常增加。 |
| 强制软件，去色带关闭 | no | 正常。 |
| 自动，去色带开启 | nvdec | 正常。 |

`render_gl.h` 要求调用方在进入 render API 前保持 OpenGL 默认状态。锁定的 FemtoVG `renderer/opengl.rs` 渲染过程启用 blending、使用 texture unit 1，收尾不完整恢复默认状态；此前 presenter 没有做交接。固定 mpv 的 `ra_gl.c` 在未要求 blend 的 pass 中依赖入口 blend 已禁用，`video.c` 的去色带会退出单次绘制模式、改变 pass 顺序，因此开启去色带可以掩盖错误。不是正常色带，也无需假定源文件损坏 / HEVC 不支持。

修复在 `presenter.rs` 中新增 `prepare_gl`，在 render create、update / info / render 与 free 前执行。恢复 raster / blend、程序、VAO、buffer / FBO、active texture、像素上传布局等默认状态，按版本处理 GL / GLES 的不同可用状态。没有改 hwdec 默认、强制 deband、禁用 NVDEC、使用 glFinish 或引入正常视频的 CPU 回读。对照试验保留同一文件 / 内核 / NVDEC / deband=false，画面恢复正常，确认为本应用 GL 交接缺陷。

来源：[固定 render_gl.h](https://github.com/mpv-player/mpv/blob/e470f8986e/include/mpv/render_gl.h)、[固定 ra_gl.c](https://github.com/mpv-player/mpv/blob/e470f8986e/video/out/opengl/ra_gl.c)、[固定 video.c](https://github.com/mpv-player/mpv/blob/e470f8986e/video/out/gpu/video.c)、[FemtoVG 0.25.1](https://github.com/femtovg/femtovg/blob/f32dc79cd4762feb6027c55d2bf10d39de3c7767/src/renderer/opengl.rs)。代码依据下载头文件 / 固定源码以及本机 Cargo 缓存核对。

## 图像验证

对原视频取相同大小 GPU 截图，以软件解码为参考，比较视频内侧区域：x=65、y=236、w=2039、h=1023。每通道差异超过 12 算明显偏差；容许明显偏差像素比例 ≤1%、RGB 平均绝对差 ≤1.5 / 255。控制栏 / 标题 / 黑边不参与比较。

| 对比原视频 | 明显偏差像素比例 | 平均 RGB 绝对差 | 结果 |
| --- | --- | --- | --- |
| 修复前 NVDEC，无去色带 vs 软件参考 | 25.66795% | 6.57656 | 失败，且人工确认噪点。 |
| 修复后 NVDEC，无去色带 vs 同一参考 | 0% | 0 | 通过，人工确认画面正常。 |

这一次采样区域恰好逐像素相同，不代表全视频或所有硬件都必然逐像素一致。原视频为运动素材，只作为同帧诊断证据；可复现自动回归改用自有静态图案：

```powershell
powershell -ExecutionPolicy Bypass -File scripts/Test-Render.ps1
```

脚本用 FFmpeg / libx265 生成 8 秒静态 SMPTE 色条，3840 × 2160、30fps、HEVC / yuv420p，fixture SHA256 `3cbbf6d3a5d62541a5574528b224802acbd586338625c4dcf889193ced65ecd2`。不同 FFmpeg / encoder 构建需记录新 hash。分别运行自动 / 软件 / 去色带，检查实际硬解、帧数、退出码、截图新鲜度，再调用开发例子 render-compare 比较静态视频区域。

本机实际结果：

| 修复后静态 4K HEVC | decoder / render 次数 / exit | 相对软件参考 | 结果 |
| --- | --- | --- | --- |
| 自动，无去色带 | nvdec / 136 / 0 | 明显偏差 0%，平均差 0 | 通过。 |
| 软件，无去色带 | no / 132 / 0 | 参考 | 通过。 |
| 自动，去色带开启 | nvdec / 137 / 0 | 明显偏差 0%，平均差 0.22694 | 通过。 |

“无错误 + 帧数增长”不能再独立作为画质验收；此前视频报告 / testsrc2 截图漏过本缺陷。本轮同时重跑 Test-Video，更新自有素材截图，补上图片一致性证据。开发比较工具 / 一次回读不参与正常播放，静态 SDR 的阈值不能直接应用于任意运动素材或 HDR。

## 检查与边界

严格 clippy / workspace 全 targets tests / debug 构建通过；release 已重新构建。最终代码再次播放原视频，均为 NVDEC / deband=false：Debug 播放位置 4.6046 秒、render 135 次，截图相对软件参考仍为明显偏差 0%、平均差 0；Release 播放位置 6.6066 秒、render 199 次、丢帧 0。两者 engine / render 错误为空，退出码 0。共享设备、GL 版本及其他 GPU / macOS / Wayland / X11 仍仅按原矩阵资格，不能把本次修复扩大为所有平台通过。根目录 manifest 和用户默认设置不变。

用户当时仍打开旧 release 进程；为避免关闭用户窗口，将旧 executable 在 target/release 内改名备份后构建新 `yyplayer.exe`，进程保持运行。用户需要关闭旧窗口、重新打开修复版后才生效。备份和构建均在 Git 忽略目录，未修改原视频。
