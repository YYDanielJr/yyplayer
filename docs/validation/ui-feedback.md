# 浮空提示与全屏控制栏

日期：2026-10-01。范围：Windows 开发预览，沿用 Slint 1.17.1，不新增依赖。

## 实现

- `app.slint` 将状态信息从标题 / 底部占位行移为浮空卡片，4 秒自动消失，可点击 × 提前关闭。文字改变或新的保存 revision 会重新显示和计时；60ms 状态投影不会重新唤醒已关闭的同一次提示。内核信息 / 渲染错误仍可在原信息页 / 错误区域查看。
- 视频全屏下，PlayerBar 覆盖画面底部，3 秒无操作后隐藏。普通窗口 / 最大化 / 音乐页面仍常显。视频区域始终占满全屏可用高度，显示 / 隐藏不改变 FBO 尺寸。打开侧板仍按原逻辑分配宽度。
- `bootstrap.rs` 在现有 Winit 事件过滤器中观察鼠标移动、按下 / 释放、滚轮、触摸和按键，不拦截这些指针事件。鼠标位置按窗口缩放换算为逻辑坐标；悬停控制栏、按住鼠标左键 / 触摸、打开播放选项时暂停隐藏计时，解除保护后重新计时。失焦清理按下状态，离开窗口 / 改变窗口尺寸清理悬停状态。
- 两个 Slint Timer 只在需要等待超时时运行，触发后停止；未增加轮询线程或修改 mpv / OpenGL 状态交接。

## UI 回归

```powershell
cargo run --locked -p yyplayer-app --example ui-feedback --offline
```

`ui-feedback` 使用真实 Slint UI、软件截图和真实计时器，注入 Slint PointerPressed / PointerReleased 来点击关闭按钮。12 个检查点通过：初始显示、3 秒隐藏、4 秒提示超时、唤醒、同文字的新保存 revision、手动关闭后稳定投影、不被旧计时提前关闭、拖动 / 悬停 / 选项保护、解除保护后隐藏、退出全屏恢复常显。普通全屏阶段视频区域保持 1240 × 900 逻辑像素；有侧板时宽 870，窗口模式恢复高 778。

本地结果与布局图片在 `target/ui-feedback/`，不进入正常播放路径。已人工查看可见 / 隐藏截图，浮空卡片和控制栏位置正确。这是 UI 状态和 Slint 点击检查，不冒充物理鼠标 / 触摸设备或跨屏 DPI 的人工验收；后者仍需要对应设备。

## 其他检查

- fmt、严格 workspace all-targets clippy、workspace all-targets tests、Debug / Release 构建通过。
- Windows PowerShell 5.1 读取 UTF-8 诊断 JSON 时默认 ANSI 导致脚本误报，本轮对 Test-Render / Test-Video 指定 `-Encoding UTF8`，兼容旧 PowerShell。
- 真实播放与 GPU 图像回归结果在本轮 STATUS 中记录；沿用原素材与设备资格范围，不扩大到未测平台。
