# Linux CI 跨平台条件依赖源码准备修复

日期：2026-10-03，分支 dev-linux。用户第二次打包日志中，Git 所有者错误已消失，Release 编译成功。新的失败为 `cargo metadata --locked --offline --format-version 1` 需要未缓存的 `accesskit_ios 0.1.2`，返回 101，发生在 `rust_notices` 阶段。

Linux 构建不会自动下载所有其他平台条件依赖；许可记录和可在空 Cargo home 中解析的对应源码包需要完整依赖图。依据 [Cargo fetch 文档](https://doc.rust-lang.org/cargo/commands/cargo-fetch.html)，不指定 target 的 fetch 下载全部目标依赖，`--locked` 保持依赖版本。

## 改动

- `scripts/package-linux.py`：Git 预检之后、Release 构建之前执行 `cargo fetch --locked`；离线模式添加 `--offline`，缺少源码缓存时在编译 / 创建输出前失败；`--skip-build` 也准备源码缓存。
- `.github/workflows/linux-packages.yml`：安装 Rust 后、lint / tests 前执行相同 fetch，并运行源码准备边界测试。
- `scripts/test-linux-packaging-sources.py`：覆盖在线 / 离线、编译 / 复用二进制四种组合，以及缺少缓存时不启动构建 / 打包。
- 完整的离线 metadata、notices 和原始 crate 归档收集保留，没有过滤掉 iOS / Windows 等条件依赖。更新 Linux 指南、CHANGELOG 和 STATUS。

## 已完成检查

- 新增 2 个边界测试（覆盖四种模式及失败路径）通过；此前 4 个 Git 所有者检查测试通过。
- Python 编译、actionlint 1.7.12 workflow 检查、`git diff --check` 通过；actionlint 未启用外部 shellcheck / pyflakes。
- 独立真实 Cargo fixture：只声明 iOS 的 `cfg-if 1.0.5` 条件依赖，私有缓存仅有索引，无该 crate 源码。Linux `cargo build --locked --offline` 成功，而无过滤的 offline metadata 与 fetch 返回 101。补入该锁定原始 archive 后，完整 offline fetch 与 metadata 成功，Cargo.lock hash 不变；全过程不使用网络，也不改开发缓存。此 fixture 验证依赖准备差异，不是 YYPlayer 或 iOS 编译验收。

## 完整本地打包结果

在 Ubuntu 26.04 复用既有 Release，使用项目独立 Cargo home，执行完整 `--offline --skip-build` 打包流程。通过模块将输出定向到 `target/linux-source-qualification/dist`，没有覆盖既有 `dist`。输出：

- deb 9.7 MiB、AppImage 118.0 MiB、源码包 102.3 MiB，另有三个 JSON 清单及 SHA256SUMS；六个被记录产物的 SHA-256 全部通过。
- 解读源码包：614 个 registry crate，与 Cargo.lock 的 registry 项完整一致；每个原始 archive hash 与 manifest 一致。包内 Cargo.lock 与当前文件一致，打包脚本包含本修复。
- `accesskit_ios 0.1.2` 原始源码 archive 存在；提取 deb 后确认许可 manifest 中也有该条目（MIT OR Apache-2.0；上游该 crate 无独立 notice 文件，`notice_files` 为空）。此处证明完整依赖收集，不代替正式发行许可资格。
- Cargo.lock 未改。原 Release hash 保持 `aaa47c083f1c39a3c750f965f3ba998c7dd90add165144ddf7ada7e334ee03e4`；本轮没有重新编译应用、启动播放器或重跑 GPU / 安装验证。

本轮未触发远端 workflow，未更改应用 / Rust 依赖版本或全局系统设置，未安装 apt 库。真实 GitHub 新容器的完整下载、编译及 artifact 上传仍需用户推送修复后新运行 `Ubuntu 26.04 packages`；不是对旧失败任务直接 Re-run。已有正式发行限制保持。
