# 手动 Linux 编译矩阵验证记录

日期：2026-10-03。分支：dev-linux。本轮范围为编写用户手动触发的 GitHub Actions 和汇总程序；没有执行远端 workflow，没有实际在旧发行版容器内编译应用。

## 交付文件

- `.github/workflows/linux-compatibility.yml`：手动入口、准备矩阵、独立构建、失败日志上传和报告汇总。
- `scripts/linux-compat/matrix.json`：5 个 Ubuntu LTS、5 个 Debian 正式版、1 个可选 testing。
- `scripts/linux-compat/container-build.sh`：仅限专用临时 Docker 容器，apt / Rust / fetch / Release 编译 / ELF 证据分阶段记录。
- `scripts/linux-compat/run.py`：Docker 宿主执行、超时清理和保守最低版本推断。
- `scripts/linux-compat/test_reports.py`：报告及宿主故障边界测试。
- README、CHANGELOG、STATUS、[运行指南](../LINUX_COMPATIBILITY_CI.md) 和生成目录忽略规则。

## 已运行检查

| 检查 | 结果 |
| --- | --- |
| `python3 scripts/linux-compat/test_reports.py` | 14 个测试通过；数字版本排序、缺失 / 重复 / 畸形 / 过期证据、testing 排除、环境未决、实际失败证据、错报系统版本、弱 GLIBC 符号、Markdown、报告落盘 / Summary、Docker 缺失 / 超时 / OOM 及清理边界 |
| `bash -n scripts/linux-compat/container-build.sh` | 通过 |
| `python3 -m py_compile scripts/linux-compat/run.py scripts/linux-compat/test_reports.py` | 通过 |
| `actionlint -shellcheck= -pyflakes= .github/workflows/linux-compatibility.yml` | 1.7.12 通过；外部 shellcheck / pyflakes 未启用，Bash / Python 另做上述检查 |
| YAML / CLI 矩阵检查 | 只有 workflow_dispatch，3 个 job 均为现代宿主、没有 job-level container；默认 10 个 case，testing 开启 11 个，Ubuntu / Debian 列表及唯一 ID 正确 |
| 无专用容器标记时调用构建脚本 | 返回 2，在任何 apt 操作前拒绝运行 |
| Docker 官方 Registry API | 11 个镜像 tag 均存在并提供 linux/amd64；只读查询，没有本机 Docker 拉取 / 构建 |
| `git diff --check` | 通过 |

actionlint 由上游 v1.7.12 release 临时下载到 `/tmp`，经同一官方 release 的 SHA-256 清单校验：`8aca8db96f1b94770f1b0d72b6dddcb1ebb8123cb3712530b08cc387b349a3d8`（linux_amd64.tar.gz）。没有 apt 安装或系统设置变更。

报告测试使用明确标记的合成 observations 和 mock Docker，验证推断及故障处理，不构成任何发行版编译通过证据。测试确保先前发行版无法得出结论时，即使后续编译成功，也不能声称已确定绝对最低版本。

本轮仅 CI / 文档变更，没有应用代码修改；既有 Release SHA-256 保持 `aaa47c083f1c39a3c750f965f3ba998c7dd90add165144ddf7ada7e334ee03e4`，不重新打包已有 dist，不重跑不相关的播放 / GPU / Rust 检查。

## 待验证与下一步

当前本机没有 Docker；没有用其他宿主的编译冒充对应容器结果。旧仓库签名 / 下载链路、Rust / 系统库实际构建、GitHub artifact 服务和 runner 执行均待首次远端运行。

用户将文件提交 / 推送并在默认分支注册手动入口，再选择含有 Linux 适配的分支执行。读取真实 `report.md` / `report.json` 后才能填写实测最低编译版本。该报告本身也不能证明旧内核、播放、GPU、deb / AppImage 安装或正式发行资格。
