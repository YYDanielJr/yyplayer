# 共用源码合并与 GitHub 构建

更新：2026-10-03。源码许可为 **GPL-3.0-only**。正式主分支为 `master`，本次 Windows/Linux 共用代码在 `dev-linux` 准备合入主分支；本文更新不代表实际合并已经执行。提交、合并和推送由用户完成。

## 共用源码与仓库目录

Windows x64 MSVC 与 Linux amd64 GNU 使用同一个 Cargo workspace、Cargo.lock、core、controller 和 Slint 界面；平台 API 与依赖通过 target cfg 选择。在各自系统安装构建依赖后，都可以执行：

```bash
cargo build --locked --release -p yyplayer-app --bin yyplayer
```

默认本机目标的 Windows 产物为 `target/release/yyplayer.exe`，Linux 产物为 `target/release/yyplayer`。分支名不决定编译平台，运行时和安装包仍按系统分别准备，见 [README](../README.md) 与 [Linux 指南](LINUX.md)。

VS Code 打开完整仓库根目录，即包含 `.git`、Cargo.toml、Cargo.lock 和 crates 的目录。Windows 历史工作区为 `E:\SourceFiles\rust\yyplayer`，当前 Linux 工作区为 `/home/yydaniel/sourcefiles/yyplayer`；这些是开发者本机路径，不是编译时必须使用的路径。

## 本次合并准备

- 用户已确认：使用 **Windows x64 packages** 工作流，可以编译打包 `dev-linux` 内的共用代码。该证据限 Windows 编译与打包，没有提供 run URL / 提交号，也没有新增 Windows 播放、设备或安装回归记录。
- Ubuntu 26.04 amd64 已有本机编译、Wayland 实际播放 / GPU、独立 X11 UI、deb / AppImage 提取启动与 hash 校验记录；干净系统安装、其他发行版和硬件资格仍按 STATUS 保留。
- README、AGENTS 和开发 / 发布指南已按共用源码更新。历史 STATUS / ADR 的分支与验收表述保留当时事实；最新状态见 [STATUS](STATUS.md) 与 [合并准备记录](validation/shared-source-merge-preparation.md)。
- Cargo.lock、LICENSE 和第三方记录保留；不提交 DLL / so、编译产物、下载缓存、用户媒体、用户配置或凭据。

在 VS Code 源代码管理中检查分支与待提交变更，提交本轮文档到 `dev-linux` 后，将该分支合入 `master`；也可以通过已有 GitHub 仓库的 PR 完成。合并后检查 `master` 包含预期代码与文档，并按用户自己的发布方式推送。开发分支是否保留由用户决定，不要求按 Windows / Linux 分别维护两份应用源码。

只读复查命令：

```bash
git status --short
git branch -vv
git remote -v
git log -1 --oneline
```

不要把合并准备描述为已合并，也不要从某次编译成功推断未测试的播放或安装场景已通过。

## GitHub 构建与报告

以下为当前仓库工作流的实际触发方式；本次只更新文档，未修改 workflow。

| 工作流 | 自动触发 | 手动用途 |
| --- | --- | --- |
| Windows x64 packages | `master` push | Run workflow 选择含工作流的分支；用户已验证 `dev-linux` 编译打包成功 |
| Ubuntu 26.04 packages | `dev-linux` push | 合并后可选择 `master` 构建 deb / AppImage；当前没有 `master` push 自动触发 |
| Linux compatibility compile matrix | 无 | 合并后选择 `master`，测试 Ubuntu / Debian 各版本源码编译下限 |

最低版本矩阵需要工作流在 GitHub 默认分支注册；真实报告尚未提供，当前不能填写实测最低源码编译版本。操作与报告边界见 [矩阵指南](LINUX_COMPATIBILITY_CI.md)。

### Windows x64 产物

`.github/workflows/windows-release.yml` 使用 `windows-2022` x64 MSVC runner，校验固定 libmpv archive / DLL hash，生成：

- `YYPlayer-<版本>-<提交号>-windows-x64-portable.zip`：解压后从 `yyplayer.exe` 启动，运行时 DLL 位于 `runtime/`，配置在 `%APPDATA%`。
- `YYPlayer-<版本>-<提交号>-windows-x64-setup.exe`：NSIS 安装器，限 x64 Windows，需要管理员权限，安装到 64 位 Program Files，创建快捷方式和卸载入口；卸载不删除用户配置。

文件位于该次 Actions run 的 `YYPlayer-windows-x64-<commit>` artifact，保留 30 天。7-Zip / NSIS 缺失时工作流明确失败。

### Linux 产物

`.github/workflows/linux-packages.yml` 在 Ubuntu 26.04 容器中检查格式、lint、测试和独立 X11 UI，随后生成 deb / AppImage、对应源码归档、Rust 源码清单、runtime manifest、构建信息和校验和。产物位于 `YYPlayer-ubuntu-26.04-amd64-<commit>` artifact，保留 30 天。完整目录与运行方式见 [Linux 指南](LINUX.md)。

合并后手动运行这个工作流即可验证 `master` 的 Linux 包；若希望 `master` push 同时自动构建 Linux，需要另行修改它的 branches 触发条件。

## 正式二进制发行边界

工作流生成逐提交 CI artifact，不会自动创建 GitHub Release。Windows 的完整 libmpv 内部组件 notices / 对应源码安排与 DLL 闭包、Linux 上游 AppImage runtime 的完整静态闭包 / 可重链接安排仍待补证。构建通过不代表正式二进制发行审计、干净系统安装 / 升级 / 卸载、USB 位准确或 HDR 资格完成。

Windows 运行时见 [libmpv 说明](../third_party/mpv/README.md)，Linux 发行剩余事项见 [runtime 记录](../packaging/linux/runtime-notices/README.md)。
