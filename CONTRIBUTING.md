# 参与 YYPlayer 开发

正式源码分支为 master，功能开发使用 dev 或从最新 master 建立自己的功能分支。项目自身源码 / 原创 UI 资源采用 GPL-3.0-only，贡献应使用相同许可；不要提交无权授权的代码、用户媒体或凭据，第三方组件保留原许可。

## 开发环境

先读 [README](README.md)、[开发规划](docs/DEVELOPMENT_PLAN.md)、[开发规则](AGENTS.md) 和 [实际状态](docs/STATUS.md)。当前首要验证平台是 Windows x64 MSVC，需要 Visual Studio C++ Build Tools / Windows SDK、Rust 和 Git。当前已验证 Rust / Cargo 1.97.0，MSRV 1.92 尚未专项验证；Slint / slint-build 固定 1.17.1。

```powershell
powershell -ExecutionPolicy Bypass -File scripts/Get-Mpv.ps1
cargo run --locked -p yyplayer-app
```

获取脚本按 manifest 检查运行时 hash。DLL、下载 archive、target、用户配置和媒体不得提交。macOS / Linux 保留平台代码边界，尚未资格验证。

## 变更与检查

改动前先写明目标和验收方式，沿用 Slint 主线程、Engine worker 与 GL presenter 边界。功能变更运行：

```powershell
cargo fmt --all -- --check
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo test --locked --workspace --all-targets
cargo build --locked --release
```

再运行对应场景脚本，结果、未验证条件和下一步写入 STATUS。布局样例不能替代真实播放、USB DAC 或 HDR 验收；测试使用独立配置和自有素材，不修改默认 APPDATA。PR 描述注明触发问题、修改后的行为和实际检查结果。

编译缓存较大，完成验证并保存必要证据后，可先运行 `scripts/Clean-Workspace.ps1 -WhatIf` 核对，再清理。脚本保留 Release exe、固定 runtime 和源码，之后完整重新编译是正常现象。

公开源码步骤见 [发布说明](docs/PUBLISHING.md)。不要通过改动此仓库的文档来声称二进制发行许可或硬件资格已经完成。
