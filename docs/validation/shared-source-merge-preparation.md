# Windows/Linux 共用源码合并准备

日期：2026-10-03。当前工作分支为 `dev-linux`；目标是更新文档与执行规则，供用户随后合入 `master`。本轮没有修改应用、Cargo.lock、构建脚本或 GitHub Actions，也没有提交 / 合并 / 推送。

## Windows 编译打包证据

用户明确反馈：“可以用打包windows程序的流程打包dev-linux分支内的代码”。据此记录 **Windows x64 packages 对共用源码的编译打包通过（用户确认）**。未提供该次 Actions run URL、提交号或日志，不能填写这些信息，也不把此确认扩展为新一轮 Windows 播放、WASAPI、GPU / 退出、安装 / 卸载或所有系统版本验收。

此前源码审查确认 Windows WASAPI、固定 DLL 校验与 Windows 打包脚本保留，Linux 平台实现 / 依赖用 cfg 隔离；对 Windows MSVC 目标执行 `cargo tree --locked --offline --target x86_64-pc-windows-msvc -p player-platform --depth 1` 成功，直接依赖为 raw-window-handle / windows-sys。依赖解析仅作为结构检查，实际编译打包结论来自上述用户反馈。

Linux 的实际编译 / 播放 / GPU / 包证据沿用 [Linux 报告](linux.md) 与近期 [打包源码检查](linux-packaging-sources.md)、[工具检查](linux-packaging-tools.md)。用户此次反馈不改变 Linux 远端完整打包、旧发行版矩阵或干净系统安装的证据状态。

## 文档调整

- README 明确相同 Cargo 命令在 Windows 生成 exe、Linux 生成 ELF；更新平台功能、运行时和主分支说明。
- AGENTS 以共用源码为接续基线，保留 target cfg、Windows / Linux 回归、音频语义与 GL 生命周期要求；移除当前规则中 Linux 尚未编译的过时表述。
- 开发、Linux、兼容性矩阵、发布与运行时指南同步；规划 / ADR 增加当前状态说明，历史验证段落保留原日期事实。
- 发布指南按实际工作流说明：Windows 自动构建 master，Ubuntu 自动构建 dev-linux，两者均可手动选择分支；Linux 最低版本矩阵仍仅手动运行。

## 检查与后续

本轮文档检查使用 `git diff --check`、变更 Markdown 相对文件链接检查、工作流触发器与文档逐项核对、变更范围检查。没有应用或构建配置变化，不重复 Rust 编译、GUI、设备或打包测试。

下一步由用户提交文档、合入 master 并推送；合并后 Windows push 自动构建，Linux 可手动选择 master 打包。待提供矩阵报告后再写入实测最低源码编译版本，硬件和正式发行限制保持。
