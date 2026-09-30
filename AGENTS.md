# YYPlayer 开发执行规则

本文件面向后续人类开发者和代码代理。产品目标、选型依据、模块接口与逐阶段任务以 [README.md](README.md) 为准。当前已有 Rust / Slint 框架与界面预览；真实播放和系统能力未接入。不能把规划描述或示例 UI 当成已实现功能。

## 1. 接手时的执行顺序

1. 读 README 中选型结论、视频呈现、音频策略与当前任务对应章节。
2. 读 [docs/STATUS.md](docs/STATUS.md)，检查实际文件、工作区修改、runtime manifest、上一任务的验收证据。
3. 用户没有另行指定范围时，从 S00 起按前置依赖选取第一个未完成任务。当前 UI 优先交付来自用户明确要求，范围见 `docs/adr/0000-framework-preview.md`；后续复用现有框架补齐 S00，不重新创建工程。
4. 写清本轮目标、涉及模块、验收方式；实现一个可审查的任务或其明确子步骤。
5. 完成适当检查与运行验证，把结果、限制和下一具体步骤写入 STATUS。环境缺失记录待验证，完成其他独立工作。

用户的明确要求优先于本文件；用户只要求规划时，不擅自推进应用实现、安装工具或改变全局系统设置。本文件不要求在常规实现、修复和可逆文档更新前再次索要批准。

## 2. 已选定的架构

- Rust + Slint + libmpv，音视频使用统一内核和 AppController。
- Windows 优先；首发 x64 MSVC。平台逻辑隔离，保留 macOS、Wayland、X11 空间。
- 技术关卡必须先验证：共享 / 独占音频、GL Render API 合成、Windows HWND / D3D11 呈现。
- Windows 默认候选是合格的原生呈现路径；GL 合成负责可叠加 UI 和跨平台候选。最终决策以 S02 的 ADR / 实测为准。
- 不默认引入第二套音频引擎、自建时钟、插件系统、网络服务或全局异步 runtime。

必要选型变化写入 `docs/adr/`：背景、事实证据、候选、决定、影响和验证。不能仅因为 wrapper API 不方便换内核。

## 3. 不得违反的线程 / 生命周期规则

- Slint 组件在主线程创建与修改，worker 使用合并投递与弱引用。
- 普通 libmpv 调用属于 Engine 线程；UI 回调只提交命令，不能等待 Engine。
- 当前占位引擎仅返回即时错误，controller 在主线程处理 UI 预览。实现 loader / 播放前建立 worker、命令通道与快照投递，再接通 UI；不能在当前同步回调中直接增加 libmpv 调用。
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

## 7. 下一实现任务

补齐 README 的 **S00**：复用已有五个 crate 与 Slint 窗口，固定具体工具链版本，建立可复现 libmpv runtime manifest / loader / 获取脚本，并记录 Windows 运行结果。其后执行 S01 与 S02 能力关卡。

当前应用可用 `cargo run --locked -p yyplayer-app` 启动界面。macOS / Linux 模块只是 cfg 边界，未做真实系统适配，不能标为已支持。
