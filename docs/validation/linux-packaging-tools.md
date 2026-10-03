# Linux CI AppImage 必需宿主命令修复

日期：2026-10-03，dev-linux。用户第三次日志显示 Release 编译和 deb 生成均成功，AppImage 阶段的固定 appimagetool 因缺少 `file` 退出 1。此前 CI 和本地依赖审计清单均漏列此包；本地桌面已经安装它，因此此前本机完整打包未暴露缺项。

## 范围与检查依据

按本项目的固定 appimagetool 1.9.1 [上游源码](https://github.com/AppImage/appimagetool/blob/1.9.1/src/appimagetool.c) 核查必需 / 可选外部命令，读取实际校验过的 AppImage 内 AppRun 与工具目录。file 必需；mksquashfs / desktop-file-validate / zsyncmake 随该工具提供；AppRun 还用宿主 dirname / readlink。当前参数关闭 AppStream、未启用签名或差量更新，因此没有将这些可选路径的程序误列为本轮必需项。

本项目本身调用 Cargo / Rust、Git、dpkg 系列、ldd、patchelf 和 desktop-file-validate，全部纳入 `packaging/linux/tools.json` 的宿主命令到 apt 包映射，Rust 两命令单独提示工具链安装。`scripts/check-linux-deps.py` 共用映射，避免 file 等打包必需命令再次漏出本地审计。

## 改动

- Ubuntu packages workflow 显式 apt 安装 `file`，Rust 安装后、fetch / lint / 编译前运行工具回归及 `package-linux.py --check-tools`。
- 打包入口在架构 / Git / Cargo / 创建输出前检查所有宿主命令，一次报告缺项及 apt 包。`--check-tools` 仅检查工具，不读 Git 状态或进行下载 / 编译。
- 新增 `scripts/test-linux-packaging-tools.py`；原 Git / 源码边界测试隔离新工具预检，以继续测试各自的失败阶段。
- 更新 Linux 指南、CHANGELOG、STATUS 及本验证记录。应用、Cargo.lock、工具下载 URL / SHA-256 和既有 dist 不变。

## 已执行验证

- 工具边界 2 个、源码边界 2 个、Git 所有者 4 个测试，共 8 个通过。缺少多个工具时，在任何子进程 / 输出目录创建之前报全；只读工具检查不触发 Git 或 Cargo。
- 实际固定工具验证：通过 manifest SHA-256 校验并新解压 appimagetool，创建自有最小 AppDir，以仅含 dirname / readlink 的宿主 PATH 执行，精确复现 `file command is missing but required` / 退出 1。仅加入系统 file 后，以同一参数、同一内置 runtime 成功生成 AppImage。该 payload 为 `/bin/true` 和项目图标，不是播放器运行资格。
- `python3 scripts/package-linux.py --check-tools` 通过；本地完整 apt 审计缺少为“无”，没有 apt 安装或全局系统改动。
- Python 编译、actionlint 1.7.12（不启用外部 shellcheck / pyflakes）、git diff --check 和新增文档链接检查通过。

未重新编译 Rust、重复完整播放器打包或启动 UI / GPU，已有完整打包证据见 [上轮记录](linux-packaging-sources.md)。本轮实际 AppImage 工具测试覆盖新增依赖问题；远端干净 Ubuntu 容器的完整 workflow 和 artifact 上传需用户推送修复后新运行验证。
