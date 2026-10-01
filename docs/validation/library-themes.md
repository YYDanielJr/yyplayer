# 目录音乐库与主题验证

日期：2026-10-01。Windows 11 / Rust 1.97.0 / Slint 1.17.1 / 固定 mpv v0.41.0-1087-ge470f8986 / RTX 5060 Laptop / 125% 桌面。master 保留音频基线 8467ca9，新实现于 dev。

## 实际实现与验收范围

Test-LibraryTheme 生成四个自有 WAV（32 秒，44.1kHz、双声道、16bit，低幅度 440Hz），只读写 target/theme-validation 独立配置及索引。不修改用户 APPDATA / 系统个性化，也不读取个人歌曲。

例子使用生产 AppController / libmpv Engine / FemtoVG GL presenter。13 阶段通过：目录拖入 / 后台递归索引、两个重叠目录去重、目录筛选、真实音频进入 Playing、移除歌曲 / 目录时继续播放、重新扫描 / 合并 / 取消、配置保存以及手动明暗 / 主题色 / 简洁与时尚四种组合。实际读到 Windows dark=true、DWM 主题色 316da1；这是只读当前 OS 值，不是切换系统设置的实测。

截图采用 FemtoVG 单次 take_snapshot 主动重绘；只证明 UI 布局，不证明听感、位准确输出或性能。已审查常规四种组合、设置页与 1000×640 加选项面板：控件在窗口内、列表可滚动、窄窗口隐藏装饰横幅。此前软件例子未建立 GL 前置条件的播放失败已纠正，最终音频检查使用实际生产 presenter，不伪造 render-ready。

边界测试覆盖旧配置默认值、颜色解析 / 对比度、目录 / 文件重复、音频扩展名、排除、取消、离线缓存恢复及过期扫描拒绝。总计 16 个独立工作区测试，例子复用的 tests 不重复计算。

17 点 ui-feedback 再次通过，真实 Slint 计时及 × / 居中播放点击，原视频顶部超时 / 悬停、全屏自动隐藏 / FBO 尺寸、关闭提示不被投影恢复保持有效。新增阴影令软件截图耗时增加，测试改为首帧后明确启动计时，并从每步截图完成后等待下一间隔；没有缩短生产的 3 秒 / 4 秒超时。页面过渡 Timer 显式单次运行，减少动态效果将时长置零并停掉持续动画。

音乐大页面 audio-ui 五图 / 封面展开收起 / 歌词点击通过；Test-Video 11 动作 / 暂停解码重载 / 保存通过；Test-Render 4K HEVC 硬解 vs 软件平均 RGB 差 0、去色带 vs 软件 0.205388，明显偏差均 0%。音频独占策略未修改，最终 Release Test-Audio 再次通过真实 FLAC / 封面 / LRC、共享 / 独占、三级 EQ 和严格源率拒绝；自有 FLAC SHA256 为 `201ea717873fe7d586755daa5e8cb11a68c60330a1b108dbf1b03975a102875a`。

## 复现命令

```powershell
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked --offline -- -D warnings
cargo test --workspace --all-targets --locked --offline
cargo build --locked -p yyplayer-app --offline
cargo build --release --locked -p yyplayer-app --offline
powershell -ExecutionPolicy Bypass -File scripts/Test-LibraryTheme.ps1
cargo run --locked -p yyplayer-app --example ui-feedback --offline
cargo run --locked -p yyplayer-app --example audio-ui --offline
powershell -ExecutionPolicy Bypass -File scripts/Test-Video.ps1 -SkipBuild
powershell -ExecutionPolicy Bypass -File scripts/Test-Render.ps1
powershell -ExecutionPolicy Bypass -File scripts/Test-Audio.ps1 -Player target/release/yyplayer.exe -SkipBuild
```

Test-LibraryTheme 依赖开发机 Python 生成 WAV；常规程序不依赖 Python。测试数据 / 大图保留 target，选定自有 UI 图提交 docs，runtime 二进制不提交。

## 未验证与下一步

当前目录上限 20000 首 / 200000 项 / 64 层；列表虚拟化且队列 / 搜索按 revision 缓存。没有对上限规模、网络盘、损坏超大标签、长时占用进行 Release 性能测量，不能给出节能 / FPS 提升数字。取消在文件边界生效，正在执行的读取 / OS 对话框返回后才能结束相应 worker。

目录 picker、单文件 picker 与颜色输入框完整 OS 人工操作待验；自动例子走实际目录路径 / controller，并非操作原生文件对话框。系统偏好热切换的真实人工验收未做，后台观察代码已有；macOS / Wayland / X11 尚未编译或验证系统外观。

时尚材质是渐变透明 / 高光 / 阴影，没有原生 Apple backdrop blur 或光学折射；标准 Slint 控件的原生焦点色仍由标准样式决定，主题色应用于应用控件、文字强调和材质。后续先验真实系统明暗 / 强调色变化、大目录性能、OS 文件对话框，再按需求增加真正增量索引 / 专辑浏览。用户魅蓝 DSP 小尾巴的 WASAPI 独占确认已另记录，位准确 / USB 广泛格式仍待验。
