# 打包与发布

更新：2026-10-13。当前 dev 为 **0.5.4**，功能改动已提交为 `a95dd38`，包括 SQLite 音乐库、统一控件和侧边抽屉、窗口状态查询及 Windows 升级安装。版本从根 Cargo.toml 生成，五个 crate、EXE 资源、安装器和包名使用同一版本。

所有开发和文档修改从 dev 开始，master 只合入已验证的 dev 改动。提交、合入、推送和发布分别按授权执行；当前分支进度以 Git 为准。打包不会自动提交或发布，也不按分支名选择操作系统。

## Windows 本地打包

准备 Rust x64 MSVC、Visual Studio C++ Build Tools / Windows SDK 和 Git，双击仓库根目录的 **Package-Windows.cmd**。

入口会准备缺失的固定 libmpv 与便携 NSIS，编译当前源码，生成 portable ZIP 和 Setup。首次准备依赖需要联网，libmpv 解压需要 7-Zip 或支持 7z 的 Windows tar。使用 pwsh，找不到时使用 Windows PowerShell 5.1，无需 Python 或修改全局执行策略。

每次输出到独立的 `dist/windows-<时间>-<随机标识>/`，成功后打开目录，按键关闭窗口。日志保存在 `target/windows-packaging-runs/<本次标识>/packaging.log`。

需要离线或不打开目录时：

```powershell
.\scripts\Package-Windows-Interactive.ps1 -Offline -NoOpen
```

命令行 / CI 入口：

```powershell
.\scripts\Package-Windows.ps1
# 缓存完整时离线打包；也可指定外部 NSIS 编译器
.\scripts\Package-Windows.ps1 -Offline -NsisCompiler 'C:\Tools\NSIS\makensis.exe'
```

命令行入口需提前准备固定 libmpv 和 NSIS。NSIS 本地 / CI 默认使用 [manifest](../packaging/windows/nsis.json) 锁定的 3.12 便携归档；外部编译器最低 3.12。便携工具放在本地项目缓存或 CI 临时目录，不修改全局安装，详见 [NSIS 工具说明](../packaging/windows/NSIS-BUILD-TOOL.md)。

脚本预检后自动执行 `cargo build --locked --release --target x86_64-pc-windows-msvc -p yyplayer-app --bin yyplayer`，从本次 Cargo 的 `compiler-artifact.executable` 获取 EXE，检查构建结果、版本和 hash。无需提前手动编译，也不会拿旧 EXE 打包。支持 Cargo 自定义产物目录、`-OutputDirectory` 和从其他目录调用。

ZIP 与 Setup 全部生成成功后才发布到输出目录；失败可重试，已有同版本 / 提交号的包不会覆盖。包内 BUILD-INFO 记录生成时的提交号、工作区修改状态和 EXE SHA-256。提交前生成的包应保留真实信息；需要提交后包时重新打包，不能只改文件名或 BUILD-INFO。

### 安装和升级

- **Portable ZIP**：完整解压后运行 `yyplayer.exe`，播放内核在 `runtime/`。设置仍保存在 `%APPDATA%\YYPlayer`。
- **Setup**：限 x64 Windows，需要管理员权限，默认安装到 64 位 Program Files，创建快捷方式和卸载入口。
- **升级**：通过固定卸载登记识别旧版，沿用原安装目录，保留设置、SQLite 音乐库和歌单；同版本可重装，低版本不能覆盖高版本。
- **失败和卸载**：替换程序文件失败时恢复备份；不调用旧卸载器先删除安装。卸载只删除安装清单中的文件和快捷方式，保留用户数据及额外文件。
- **缩放**：NSIS 3.12 安装器启用系统 DPI 感知；不承诺 Per-Monitor V2 或跨显示器自动适配。

0.5.1 → 0.5.4、0.5.3 → 0.5.4 的升级及失败恢复已有隔离测试，完整包也有独立启动证据。真实管理员安装 / 升级 / 卸载和多 DPI 显示器仍待验证，见 [0.5.4 验收](validation/frontend-0.5.4.md)与 [0.5.3 安装器验收](validation/frontend-installer-0.5.3.md)。

修改打包或安装器后运行 Test-WindowsPackaging、Test-WindowsPackagingLauncher 和 Test-WindowsInstaller，覆盖编译失败、旧 EXE、自定义目录、NSIS 失败、重试、旧包保护、升级和恢复。入口与历史证据见 [打包验收](validation/windows-packaging.md)及[双击入口验收](validation/windows-packaging-launcher.md)。

## Linux 打包

Ubuntu 26.04 amd64 的环境和音频策略见 [Linux 指南](LINUX.md)。Windows 与 Linux 使用同一源码和锁定依赖，运行时与安装包分别准备。Linux 打包入口：

```bash
python3 scripts/package-linux.py --check-tools
python3 scripts/package-linux.py
# 已构建 Release 且缓存完整时
python3 scripts/package-linux.py --offline --skip-build
```

输出包含 deb、AppImage、对应源码归档、Rust 源码清单、runtime manifest、构建信息和校验和。deb 使用系统 libmpv；AppImage 校验包内内核和非驱动依赖，仍需宿主 glibc、显卡驱动和桌面服务。

已有 Ubuntu 本机播放、GPU 与包提取启动记录，不能替代 0.5.2–0.5.4 的 Linux 复测，也不代表其他发行版兼容。源码包需保留完整 Cargo.lock 依赖图的原始源码与许可，不能只收集当前平台的依赖。完整规则与证据见 Linux 指南。

## GitHub Actions

以下是当前工作流的实际触发方式：

| 工作流 | 自动触发 | 手动运行 |
| --- | --- | --- |
| Windows x64 packages | master push | 选择 dev、master 或其他包含该工作流的分支 |
| Ubuntu 26.04 packages | dev-linux push | 可选择 dev、master 等分支；master push 不会自动触发 |
| Linux compatibility compile matrix | 无 | 检查各 Ubuntu / Debian 版本能否编译源码 |

Windows 工作流在 windows-2022 上准备 Rust stable、固定 libmpv 和 NSIS 3.12，检查图标及打包 / 安装器回归，再调用同一本地打包脚本。产物为 `YYPlayer-<版本>-<提交号>-windows-x64-portable.zip` 与 `YYPlayer-<版本>-<提交号>-windows-x64-setup.exe`。

Linux 工作流在 Ubuntu 26.04 容器中运行格式、lint、测试和独立 X11 UI 检查，再生成完整包与源码记录。最低版本矩阵只报告源码编译结果，操作见[矩阵指南](LINUX_COMPATIBILITY_CI.md)；当前没有已保存的远端报告，不能填写实测最低版本。

在对应 Actions run 下载 `YYPlayer-windows-x64-<commit>` 或 `YYPlayer-ubuntu-26.04-amd64-<commit>` artifact，保留 30 天。工作流不会自动创建 GitHub Release；手动入口需要工作流已在 GitHub 默认分支注册。

## 发布前还需完成的事项

项目源码与原创 UI 资源采用 GPL-3.0-only，第三方组件保留各自许可。发布前检查当前分支、工作区、暂存 diff、Cargo.lock、LICENSE 和第三方记录；不提交 runtime 二进制、编译缓存、发行包、用户设置 / 数据库、媒体或凭据。

目前的包是开发预览。Windows libmpv 完整组件许可 / 对应源码和 DLL 依赖清单、AppImage runtime 的完整静态依赖 / 可重链接安排仍待补齐。CI 编译和隔离安装测试不能替代正式二进制发行审计、干净系统安装或 USB / HDR 实测。

Windows 剩余事项见 [libmpv 说明](../third_party/mpv/README.md)，Linux 见 [runtime 记录](../packaging/linux/runtime-notices/README.md)，实际检查结果与下一步见 [STATUS](STATUS.md)。
