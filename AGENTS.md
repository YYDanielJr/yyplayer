# YYPlayer 开发执行规则

本文件面向后续人类开发者和代码代理。产品目标、选型依据、模块接口与逐阶段任务以 [README.md](README.md) 为准。当前已接入 Windows libmpv 视频开发预览：真实播放、GPU 合成、分级解码设置和快捷键已实现；基础 WASAPI 独占 / 分级 EQ / 本地歌词与音乐大页面已接入；USB DAC / 位准确、系统集成和跨平台资格仍未完成。不能把规划描述或示例 UI 当成已实现功能。

## 1. 接手时的执行顺序

1. 读 README 中选型结论、视频呈现、音频策略与当前任务对应章节。
2. 读 [docs/STATUS.md](docs/STATUS.md)，检查实际文件、工作区修改、runtime manifest、上一任务的验收证据。
3. 用户没有另行指定范围时，从 S00 起按前置依赖选取第一个未完成任务。当前 UI 优先交付来自用户明确要求，范围见 `docs/adr/0000-framework-preview.md`；本轮视频优先交付见 ADR 0001；复用现有实现并按 STATUS 补齐剩余资格验证，不重新创建工程。
4. 写清本轮目标、涉及模块、验收方式；实现一个可审查的任务或其明确子步骤。
5. 完成适当检查与运行验证，把结果、限制和下一具体步骤写入 STATUS。环境缺失记录待验证，完成其他独立工作。

用户的明确要求优先于本文件；用户只要求规划时，不擅自推进应用实现、安装工具或改变全局系统设置。本文件不要求在常规实现、修复和可逆文档更新前再次索要批准。

## 2. 已选定的架构

- Rust + Slint + libmpv，音视频使用统一内核和 AppController。
- Windows 优先；首发 x64 MSVC。平台逻辑隔离，保留 macOS、Wayland、X11 空间。
- 技术关卡必须先验证：共享 / 独占音频、GL Render API 合成、Windows HWND / D3D11 呈现。
- 当前 Windows 开发预览采用 GL Render API + Slint 借用纹理，事实与限制见 ADR 0001；HWND / D3D11 和 HDR 输出专项仍需独立验证。
- 不默认引入第二套音频引擎、自建时钟、插件系统、网络服务或全局异步 runtime。

必要选型变化写入 `docs/adr/`：背景、事实证据、候选、决定、影响和验证。不能仅因为 wrapper API 不方便换内核。

## 3. 不得违反的线程 / 生命周期规则

- Slint 组件在主线程创建与修改，worker 使用合并投递与弱引用。
- 普通 libmpv 调用属于 Engine 线程；UI 回调只提交命令，不能等待 Engine。
- 已有 MpvEngine worker、64 项有界命令通道、最新快照和 render 租约。controller 仅提交命令；禁止把普通同步 mpv 调用搬到 UI / GL notifier。初始化 / 退出也必须保持该边界。
- GL notifier / render 线程只操作允许的 render API；不能同时发普通同步 mpv 命令，不能持有 Engine 需要的锁。
- mpv 回调只做非阻塞唤醒，不能 render、更新 UI、等待或执行重工作。
- OpenGL render 调用串行，使用创建 render context 时相同的当前上下文。
- 借用纹理不能跨 Slint 窗口复用；释放前保证 UI 不再使用；旧 GL ID 不能跨 context 重建复用。
- render context 先于 mpv core 销毁。原生 presenter 先停止 / 清理 mpv 输出，再销毁宿主窗口。
- callback context 的 lifetime 必须覆盖解绑与在途回调；FFI 指针数据跨线程前复制。
- `unsafe` 只在边界模块，注明 `SAFETY` 条件；不能凭经验添加 `Send` / `Sync`。
- 通道有界；高频值合并；Stop / Shutdown 始终可达。退出不能以 `process::exit` 掩盖资源清理缺陷。

## 4. 音频与视频语义

- 请求独占不等于已确认独占；独占不等于 bit-perfect。当前设备、API 输出格式、物理硬件格式分开表示，未知就显示未知。
- 不默认强制 192 kHz / 32 bit、音量超过 100%、EQ、响度归一化或额外音效。
- 严格模式失败不回退；优先独占按保存策略回退并保持可见状态。设备意外失联默认暂停，不突然切到扬声器出声。
- DSP 与独占可并存；未处理候选模式与会改样本的功能互斥；压缩直通与 PCM DSP / 变速互斥。
- EQ 链有名称、有验证、有更新回滚；不持续叠加节点，全部为零时移除节点。
- 同格式 gapless 依赖同一 mpv playlist；不同采样率保持源格式与绝对无缝之间的取舍必须透明。
- HDR 素材、HDR → SDR、合格 HDR 显示输出分别标记；普通 RGBA Image 不能证明 HDR 输出。
- Render API 不当作可直接使用 D3D11 / Vulkan / WGPU 的接口；Wayland 不使用 `wid` 冒充原生合成。
- 硬件解码与零拷贝分别验证；有 copy fallback 时记录实际路径。
- 不用每帧 CPU RGBA 回读 / 上传作为正常视频架构。
- 自动 next 只由一个明确的实际执行方触发；队列 ID、mpv playlist ID 与媒体 generation 分开。

## 5. 实现与验证要求

- `player-core` 无 Slint、mpv、OS API；平台差异限制在 platform / presenter。
- 绑定、crate、选项和构建版本依据锁定版本官方资料核实；禁止编造 Slint / mpv API。
- 所有初始化参数检查返回值，按 Required / Optional 分级；缺 P0 能力不能继续展示假成功。
- 文件路径保留平台原始表示，显示字符串不作为文件身份；不拼接 shell 命令处理媒体路径。
- 本地打开、标签读取、DB、设备选择、视频呈现出错都应返回可恢复的分类错误。
- 配置版本化、迁移可测、坏数据可恢复；原子替换，不先删除唯一文件。
- 新依赖最少 features、提交应用 Cargo.lock，运行时二进制用 manifest + hash 锁定。
- 不依赖开发者 PATH 上偶然存在的 DLL；最终包自带合格 runtime 与必要 notices / 源码安排。
- 功能实现按 README 第 14 节做 fmt、clippy、相关 tests 与 release build。用户限定只做框架与少量检验时，按该范围做必要格式 / 编译 / UI 预览检查，在 STATUS 明确省略项；不要把没有跑过的检查写为通过。
- `demo.rs` 与 `MediaPreview` 仅为展示，不转换成真实 PlaybackSnapshot，不伪造播放、设备、独占、EQ 或硬解成功。接入媒体后使用真实状态源替换 demo。
- Slint 与 slint-build 当前固定 1.17.1；升级两者一起改并验证对应 API。`ui-preview` 的软件截图只用于布局审查，普通应用默认 Winit + FemtoVG。
- 测试覆盖边界、竞态和数学 / 数据风险；不为低风险文字修改造无意义测试。
- 真实 DAC、HDR、多 GPU、macOS / Wayland / X11 验收需要真实环境；CI 编译不能代替。
- 性能测试用 release，记录硬件 / 驱动 / 参数 / 素材 hash 与口径。不同默认值的独立 mpv 数据不能直接拿来做对比。
- UI 日常流程不要求用户理解 mpv 选项；底层字段进入媒体信息 / 诊断页。

## 6. 每次交付与进度更新

STATUS 中填写：当前任务 / 子步骤、改动文件、检查命令及结果、真机场景结果、未验证条件、已知缺陷、下一步明确动作。相关报告放 `docs/validation/`。

任务只在代码和要求的证据都满足时标 Done。样机结论写 ADR；改变默认路径或质量配置必须有新证据。

交付说明用中文，简述实现结果、验证、限制。不能以“已支持所有平台”“完美音质”“零拷贝”“无缝”“HDR”替代明确的适用组合与测量。

## 7. 接续当前实现

先查 STATUS 与 ADR 0001 / 0002。本轮用户目标已记录在 README 顶部并在 `dev` 实现；视频已合入 `main` 的 `4eca722`；旧框架 `d10a4e8` 保留在 Git 历史。用户没有明确新任务时，优先补齐当前实现的格式 / 多设备 / 生命周期资格验证和 S00 工具链 / 发行许可；再补齐 S01 的 USB DAC / 位准确资格。不要把视频预览重新降为假状态或占位引擎。

- 固定运行时见 `third_party/mpv/manifest.json`，获取脚本 `scripts/Get-Mpv.ps1`，不要提交 DLL / archive。
- `YYPLAYER_MPV_LIBRARY` 明确绕过 hash，仅供开发；发行路径必须受控。
- 新增解码选项时同步更新 validate、分级规则、controller、UI、重载恢复和验证；规则是完整对象覆盖。
- 快捷键需同时考虑输入焦点、repeat、modifier 释放、失焦、文件对话框和退出的临时速度恢复。
- 生产 GPU 合成不得改为 CPU 图片循环；debug 单次截图与实际视频路径分开记录。
- Slint / FemtoVG 会留下 blend / active texture 等 GL 状态；进入所有 libmpv render 调用前使用 presenter 的 prepare_gl 恢复默认状态。不得删除这个边界而用默认开启去色带 / 软解掩盖画面损坏。升级渲染器后核对 render_gl.h，并重跑 Test-Render 的图片比较。
- 播放状态、实际硬解名称和帧数只能证明运行，不能证明画面正确；涉及 GL 状态 / 纹理 / NVDEC 的修改必须比较真实 GPU 图像。Test-Render 使用自有静态 4K HEVC，不提交用户视频或其截图。
- 固定 git runtime 的 render target 使用 ns，头文件注释为 us；修改 presenter / 运行时先核实 source 和 clock ABI，并跑时序测试 / 实机帧数，不能凭注释盲改。
- 不在每次投影重建相同模型或覆盖用户正在编辑的表单；设置字段由 revision 驱动。
- 状态提示是 UI 层临时浮空信息，重复状态投影不得重置超时 / 恢复手动关闭；新保存 revision 可以再次通知。全屏控制栏覆盖视频，隐藏不能改变视频 / FBO 尺寸，按住拖动、悬停控件和打开选项时保留可操作性。修改此交互后运行开发例子 ui-feedback；它只证明 Slint 计时 / 点击逻辑，不代替物理设备验收。
- 视频顶部工具栏也覆盖画面并自动隐藏，不恢复原顶部空白占位；播放按钮组相对整个窗口居中，不依赖左右内容宽度分配。实际倍速统一由下拉显示，非预设值也必须正确选中。布局修改后核对 ui-feedback 的最小 / 常规 / 较宽窗口中心点击与截图。
- `scripts/Test-Video.ps1` 使用独立配置；新增验收不能改用户默认 APPDATA 设置。
- 当前媒体位置重载已验证，快速连续打开 / EOF / 设备失联 / 睡眠 / 多屏仍需增加证据。

启动命令与操作见 README。macOS / Linux 仅保留 cfg 与库覆盖入口，未编译 / 真机验收，不能标为已支持。运行报告里区分生产 controller 脚本、状态机单测和实际键盘 / 对话框人工验收。

## 8. 音频开发预览接续

- WASAPI 独占必须结合固定 runtime 的格式接受、初始化完成和 actual AO；缓冲区对齐错误可能由内核重试恢复，不因单条日志提前判失败。更换 runtime 后重新核对日志协议，未知状态不显示已确认。
- 默认共享 / 无额外 DSP / 自动采样率协商 / 70% 音量；不改为默认 100% 或强制 192kHz。源 PCM、API PCM、物理 DAC 格式分开；bit-perfect 需要数字捕获。
- 优先独占只回退同设备共享；明确设备 / 端点失联暂停，严格源采样率不匹配暂停。重试可恢复媒体位置且保持暂停，不能突然自动换设备出声。
- EQ 单文件 > 具体设备绑定 > 全局，完整对象覆盖；有名称、有限参数、固定链替换和回滚，平直移除链。JSON / APO 参数导入不接受任意 filter / Include / 不等价 GraphicEQ。
- 标签 / 图片 / 歌词 / 预设文件在有界后台服务读取；过期媒体 revision 不覆盖当前歌曲，歌词模型和图片仅资源 revision 更新。动画只在相关大页面运行；Slint 主线程不读整首音频或解码图片。
- Test-Audio / Test-AudioBusy 使用自有 fixture / 独立配置；audio-ui 是布局样例。不得把设备竞争双失败测试当作成功共享回退，或把 PCM 位宽当成 DAC 实际格式。USB 拔插 / 成功回退 / 数字捕获仍按报告补证据。
