# 字体、集成窗口控制栏与音乐库列：Windows 验收

日期：2026-10-01。基线 dev `1f5f8c9`，继续在 dev。Slint / slint-build 1.17.1、Winit 0.30.13、libmpv `v0.41.0-1087-ge470f8986`。本轮不改变音频默认策略或 GL 状态恢复。

## 范围与实现

README 在代码前记录本轮目标。core 新增字体 / 列宽配置与验证；platform 的 fonts 模块后台 GDI 枚举；app 的 font_service / font_controller 处理有界任务、缺失回退、选择与原子保存。UI 新增字体页、歌词字体、主题窗口控制栏、可拖动列和对称图标。Engine 设置并回读字幕属性，失败尝试回滚。设计见 [ADR 0004](../adr/0004-fonts-window-chrome-and-columns.md)。

## 检查与实测

使用自有 80 首四秒 WAV 和同名 LRC、30 秒 1280×720 静态视频、SRT / ASS。配置 / 索引 / 临时素材均在 target/ui-customization，不读取用户媒体，不写 APPDATA。测试走生产 Controller、Engine 与 GL presenter；WindowEvent 是合成的 Slint 事件，窗口按钮随后调用真实 Windows 窗口操作。

```powershell
powershell -ExecutionPolicy Bypass -File scripts/Test-UiCustomization.ps1
cargo fmt --all
cargo clippy --locked --offline --workspace --all-targets --config profile.dev.debug=0 --config profile.dev.incremental=false -- -D warnings
cargo test --locked --offline --workspace --all-targets --config profile.dev.debug=0 --config profile.dev.incremental=false
cargo run --locked --offline -p yyplayer-app --example ui-feedback --config profile.dev.debug=0 --config profile.dev.incremental=false
cargo run --locked --offline -p yyplayer-app --example navigation-ui --config profile.dev.debug=0 --config profile.dev.incremental=false
cargo build --locked --offline --release
```

- 本机系统字体目录读到 **278 个族**；设置页真正点击全局字体下拉并通过键盘选中 Arial，界面属性与显示值一致。歌词和字幕选择 Times New Roman，保存后的配置一致。
- 字幕实际回读 `sub-font=Times New Roman` / `sub-ass-style-overrides=FontName=Times New Roman`，清除覆盖后回读为空。真实 GPU 截图中 SRT / ASS 显示衬线字体，渲染无错误；这些结果证明本机这组文本素材，不推断所有 ASS 排版一致。
- 音乐库真实拖动表头，歌曲比例从 0.46 变为约 **0.5600000024**，保存后读回一致；1000×640 和 1240×900 布局已审查。歌曲、歌手、专辑与表头对齐；减号在图形按钮内居中，与滚动条分离。实际点击移除，80 行变为 79 行，没有删除音频。
- 同名歌词真实加载，进入音乐大页面，歌词字体为 Times New Roman；界面其余字体仍使用全局 Arial。局部字体变化不重建目录模型或在主线程解码图片。
- Windows 最小化、最大化 / 还原、关闭通过真实窗口状态断言；关闭后正常清理 Engine / presenter。左侧红黄绿仅为 Windows 上的平台布局模拟，不能称为 macOS 真机资格。
- ui-feedback **17 点**通过：关闭按钮点击、四秒提示超时、三秒视频控件隐藏、悬停 / 拖动保护、视频尺寸稳定及三尺寸整体居中播放按钮；navigation-ui **10 点**通过：实际导航、空库引导、右置面板、留白、透明度、最小 / 常规 / 较宽与简洁主题。
- fmt、严格 clippy 与 all-targets tests 通过，**17 个独立单测**；例子重复导入的 services 测试不重复计数。新增单测验证旧配置默认值、非法字体语法、NaN / 超界列比例。最后的左置布局点击与无提示遮挡的字幕 / 歌词截图再次通过 20 阶段脚本及严格检查。
- 最终 Release 构建通过。三个独立配置启动均 page=0、字体目录 278、原生 decorated=false、无 Engine / render 错误并正常退出：Arial / Times New Roman / 列宽 0.56 保存恢复，旧默认设置使用系统 UI / sans-serif，缺失字体回退至相同默认。精简结果见 [成品启动结果](ui-customization/release-results.json)。
- 最终 Release 自有视频 **11 动作**回归通过，266 次 render：倍速 / 音量 / 静音 / 暂停 / seek、真实最大化 / 全屏 / 退出全屏、暂停解码重载保留位置、字体保持 Times New Roman、正常退出。见 [成品视频结果](ui-customization/release-video.json)。
- 清理先核对 WhatIf，移除 **5,015,720,628 字节（约 4.67 GiB）** 可再生成产物。工作区从约 5.19 GB 缩至 **169,546,222 字节（约 161.7 MiB，提交前）**，文档 / Git 提交会小幅增加。target 仅保留 21,572,608 字节的 Release exe；固定 DLL / 头文件、源码、Git 和自有证据保留。清理后再次验证三个启动配置通过。哈希和体积见 [产物记录](ui-customization/artifacts.json)。

检查中纠正了首次布局的隐式居中坐标、切换字体时重建下拉模型导致显示值重置，以及验收例子过短音频 / 移除后的索引变化。失败不计作通过；最终脚本使用四秒素材与当前行身份。

## 保留的自有图片

[默认列宽](ui-customization/library-default.png) · [拖动后列宽](ui-customization/library-resized.png) · [字体选择](ui-customization/font-selected.png) · [左置布局模拟](ui-customization/left-controls.png) · [实际歌词字体](ui-customization/lyric-font.png) · [阶段结果](ui-customization/results.json)

图片是本机开发预览和自有素材；不包含用户上传的截图或视频。保留布局证据后清理 target 临时图像、素材及编译缓存；可用脚本重新生成。

## 限制与下一步

Windows 物理鼠标拖动 / 边缘缩放、双击标题区、多屏 DPI、拖动 Snap 和任务栏交互尚需人工矩阵；当前合成点击不替代这些操作。没有 Windows 最大化悬停 Snap 菜单。macOS 左置控件、原生全屏 Space 与 Linux Wayland / X11 未编译 / 真机验收。

当前设置选择字体族，字号沿用现有界面。字体缺字由渲染器回退，复杂脚本、彩色字体、安装 / 卸载热刷新和缺失字体提示仍需扩大覆盖。字幕设置不修改图片字幕 / 已烧录字幕；ASS 行内字体命令及排版需要专项素材。

未修改 WASAPI / EQ、解码选择或 GL prepare_gl，本轮没有重复 USB 数字捕获、4K HEVC 像素比较、HDR 或性能测试。下一具体动作是物理窗体操作 / 多屏 DPI 与字体安装移除矩阵，再按音频记录补齐 USB DAC 资格。
