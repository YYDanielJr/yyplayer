# Linux CI 打包 Git 所有者检查修复

日期：2026-10-03，分支 dev-linux。用户提供的 `Ubuntu 26.04 packages` 日志显示 Release 编译已成功，随后 `scripts/package-linux.py` 的 `git rev-parse` 因 `detected dubious ownership` 返回 128，deb / AppImage 打包未完成。不是该次 Rust 编译失败。

## 修复

`scripts/package-linux.py` 的提交号、dirty 状态和 NUL 分隔源码清单查询统一使用：

```text
git -c safe.directory=<脚本所在仓库绝对路径> -C <同一路径> <查询参数>
```

此信任只用于当前 Git 命令，不写全局配置，也不信任其他仓库；行为依据 [Git safe.directory 文档](https://git-scm.com/docs/git-config#Documentation/git-config.txt-safedirectory)。提交号检查移到 Cargo 构建之前，以便真实的 Git 访问故障及早暴露，不用替代提交号或假源码清单掩盖失败。

`scripts/test-linux-packaging-git.py` 使用 Git 上游自己的 [所有者检查测试机制](https://github.com/git/git/blob/master/t/t0033-safe-directory.sh)，在临时仓库中模拟 foreign owner；不修改实际 checkout 所有权或用户配置。Ubuntu 打包 workflow 在安装 Rust 前运行此测试。

## 验证与边界

- `python3 scripts/test-linux-packaging-git.py`：4 个测试通过。实际复现未加例外时 Git 128 / dubious ownership，验证三种打包查询成功，包含空格 / 中文目录和换行文件名；未持久化信任、其他仓库仍拒绝；Git 预检失败不会启动 Cargo。
- `python3 -m py_compile scripts/package-linux.py scripts/test-linux-packaging-git.py`：通过。
- actionlint 1.7.12 检查 `.github/workflows/linux-packages.yml`：通过（外部 shellcheck / pyflakes 未启用）。
- `git diff --check`：通过。

本地 Git 2.53.0。未运行真实 GitHub job、未重新编译应用或生成 / 覆盖既有 dist。本轮没有 Rust / UI / 音视频代码变化，无需 apt 安装。完整包是否还有后续环境问题，需用户推送新提交并重新运行 `Ubuntu 26.04 packages` 验证。

GitHub 的 [Re-run jobs 使用原运行的提交](https://docs.github.com/en/actions/how-tos/manage-workflow-runs/re-run-workflows-and-jobs)；本修复提交推送后应新建一次运行，手动入口选择包含修复的 dev-linux 分支，或由该分支的新 push 触发。
