# 第三方组件与许可记录

YYPlayer 自身源码及原创 UI 资源采用 **GPL-3.0-only**，见 [LICENSE](LICENSE)。该声明不替代第三方组件原有的版权与许可。

| 组件 | 使用方式 | 许可来源 / 当前处理 |
| --- | --- | --- |
| Slint / slint-build 1.17.1 | Rust UI 与构建工具 | 本项目选择 GPL-3.0-only 分支；框架的其他可选许可证不作为本项目的默认授权。见 [锁定版本的上游说明](https://github.com/slint-ui/slint/blob/v1.17.1/LICENSE.md)。 |
| libmpv `20260928-git-e470f8986e` | Windows 开发预览动态库 | 固定构建 manifest 标为 GPL 组合；仅提交获取脚本 / hash / 来源，不提交 DLL。见 [manifest](third_party/mpv/manifest.json) 和 [运行时记录](third_party/mpv/README.md)。 |
| 其他 Rust 依赖 | Cargo.lock 锁定版本 | 保留各上游许可证；本轮 Windows normal / build 依赖的元数据清单见 [依赖许可元数据](docs/dependency-licenses.md)。Cargo manifest 的表达式不能代替每个组件的完整 notices。 |
| 图标 / 演示封面 | 编译进 Slint 的本地 SVG | ADR 0000 记录为原创 SVG，随本项目源码授权；验收截图使用自有素材。用户附件、专辑图片、音视频与 APPDATA 配置不入仓库。 |

本轮准备的是 GitHub **源码仓库**。没有建立携带 exe / DLL 的发行包，也没有完成固定 mpv 构建的所有 FFmpeg / 编解码组件 notices、对应源码 / 构建说明与 DLL 依赖闭包。二进制发行前按 README S00 补齐这些文件，保留上游原文并记录具体构建；不能把本页作为完整二进制许可审计。

本地已安装的 Cargo 依赖通常含 LICENSE / LICENSES 文件；源仓库中不复制整个 registry 或 target。公开源码的许可证选择不改变依赖各自的授权条件。
