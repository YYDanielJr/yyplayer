# YYPlayer 程序图标方向稿

日期：2026-10-02

## 本轮目标

为 YYPlayer 提供 10 个可供选择的 SVG 程序图标方向，其中 5 个采用 Material Design 视觉语言，5 个采用原创液态玻璃视觉语言。图标需简洁、清晰，并体现本地音视频播放、媒体库、进度控制、EQ 或精确输出等产品特征。

## 交付内容

- `assets/icon-concepts/material-01-play-pulse.svg`
- `assets/icon-concepts/material-02-y-play.svg`
- `assets/icon-concepts/material-03-track-orbit.svg`
- `assets/icon-concepts/material-04-eq-frame.svg`
- `assets/icon-concepts/material-05-library-stack.svg`
- `assets/icon-concepts/liquid-06-prism-play.svg`
- `assets/icon-concepts/liquid-07-y-ribbon.svg`
- `assets/icon-concepts/liquid-08-orbit-disc.svg`
- `assets/icon-concepts/liquid-09-wave-panel.svg`
- `assets/icon-concepts/liquid-10-dual-lens.svg`
- `assets/icon-concepts/README.md`：候选总览、设计说明与使用建议。

## 设计约束

- 统一使用 `viewBox="0 0 1024 1024"`，保留足够的图标安全区。
- 不使用外部字体、远程资源或嵌入位图。
- Material 组以平面色块、清晰几何轮廓和有限层级为主。
- 液态玻璃组使用 SVG 原生渐变、透明度、描边和滤镜构成，不依赖生成式位图。
- 方向稿未接入应用构建，也未替换当前正式图标。

## 验证与限制

- 已对源文件结构和画布规格做人工检查。
- 当前环境的命令执行辅助进程未能启动，因此本轮未运行 XML 解析器、SVG 栅格化和小尺寸自动截图比较；不能把方向稿视为最终发行图标。
- 选定方向后仍需进行 16–256 px 光学校正、透明边缘检查，以及 Windows ICO / 安装器 / 任务栏实机验收。

## 下一步

由用户选择 1–2 个方向；再根据反馈统一配色、减少细节、制作小尺寸变体，并接入 Windows 资源与安装器构建。

## G08 首选方案精修

用户将 G08 选为最高候选后，对 `liquid-08-orbit-disc.svg` 做了第二轮定向精修：

- 将普通圆角矩形底板改为自定义连续曲率轮廓，四角过渡更接近成型玻璃而非网页卡片。
- 背景收敛为青色与紫色两处有目的的折射场，移除模糊色团堆叠。
- 外盘、轨道、中心键重新按 298 / 232 / 124 半径分级，留白更稳定。
- 轨道由顶部起点改为左上至左侧的非对称长弧，形成更明确的前进方向。
- 播放三角加大并做水平光学补偿，优先保证任务栏和开始菜单尺寸下的识别。
- 高光减少为两条结构性弧线；双层细描边仅用于表达玻璃厚度。
- 保持纯 SVG，不增加字体、位图或外部依赖，也未接入正式构建资源。

根据首轮预览反馈再次微调：移除与彩色轨道、盘片描边争抢视觉层级的顶部白色长高光；播放箭头整体向右做 16 个 SVG 单位的光学补偿。中心按钮本身保持在画布数学中心，避免用移动整个按钮来掩盖箭头重心问题。

第二次预览后移除右下方额外的浅蓝结构高光，使外盘边缘保持连续、安静；箭头从 16 单位右移补偿回调为 8 单位，修正上一轮略微偏右的问题。

## G08 程序图标接入

用户确认 G08 后，将精修 SVG 接入 YYPlayer 的应用与 Windows 发布资源：

- 将 256 px 透明 PNG 用于 Slint 侧栏品牌标记、自绘标题栏和运行时 Winit 窗口 / 任务栏图标；其余 PNG 尺寸继续作为资源保留。
- `scripts/Build-AppIcon.ps1` 从 16–256 px 共 10 个 PNG 生成 PNG-compressed 多尺寸 ICO；Windows MSVC `build.rs` 调用 Windows SDK `rc.exe` 将 ICO 编译为应用 `.res` 资源。
- Windows NSIS 安装器 / 卸载器使用该 ICO；开始菜单快捷方式和卸载项显示图标指向实际 EXE 图标资源。Windows Release workflow 在构建前生成 ICO，打包脚本将 ICO 路径传给 NSIS。
- 视觉预览：`target/icon-preview.png`。可执行文件图标资源：Release 构建目录中的 `yyplayer-icon.res`。

### 验收

- 通过：`cargo fmt --all --check`；`cargo clippy --locked --offline --workspace --all-targets -- -D warnings`；`cargo test --locked --offline --workspace --all-targets`；`cargo build --locked --offline --release -p yyplayer-app --bin yyplayer`；`git diff --check`。
- 通过：图标 ICO 生成与 10 帧尺寸检查、Windows `.res` 资源生成；UI preview 人工检查侧栏 / 自绘标题栏布局及标识清晰度。
- 待验证：环境缺少 `makensis.exe`，没有本机 NSIS 编译、安装 / 卸载验证；实际窗口、任务栏、资源管理器图标显示未做 GUI 实机检查。Windows Release workflow 配置已接入，但尚无本轮 CI 运行结果。
- 范围说明：本轮只接入图标，不代表应用的其他 S14 安装 / 卸载、runtime notices 或发行资格已经完成。

## 侧栏品牌区对齐修正

用户在 Windows 实际窗口中确认系统图标显示正常，但指出侧栏品牌图标明显高于“YYPlayer”文字。原因是 76px 的横向布局分别放置了固定 40px 图标与按自身文本高度排版的文字，二者没有共享垂直基准。现将图标按品牌行的高度居中，文字使用整行高度并垂直居中，水平间距仍为 10px。

- 通过：`cargo run --locked --offline -p yyplayer-app --example ui-preview -- --output target/icon-alignment-preview.png`；人工检查预览中图标和文字中心对齐、品牌文字完整可见。
- 通过：`cargo test --locked --offline -p player-ui`（1 项测试）、`cargo fmt --all --check`、`cargo build --locked --offline --release -p yyplayer-app --bin yyplayer`、`git diff --check`；Release EXE 已更新。
- 待验证：此轮仅生成软件渲染预览，没有在用户实际 DPI / 字体配置的播放器窗口中重新截图；预览仅证明布局，不代替真实窗口显示验收。
