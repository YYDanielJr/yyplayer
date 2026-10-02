# 公开源码发布准备检查

日期：2026-10-02。目标：用户在 VS Code 将现有项目公开到 GitHub；在本地完成许可证、元数据、文档、忽略规则和正式分支准备，不代替用户发布。

## 文件与证据

- 用户明确选择 GPL-3.0-only。根 LICENSE 从已安装且锁定的 Slint 1.17.1 `LICENSES/GPL-3.0-only.txt` 复制；字节 / SHA256 对比一致，保留完整标准文本。README 与 Cargo workspace 声明 only，五个 crate 全部继承；不是 GPL-3.0-or-later。
- THIRD_PARTY_NOTICES 记录 Slint GPL 选择、固定 GPL mpv 组合与原创 SVG；dependency-licenses.md 是 Cargo.lock + Windows normal / build 可达 383 项的上游元数据，全部有 license 字段。它不是二进制组件的完整授权审计。
- CONTRIBUTING、PUBLISHING、README、STATUS、AGENTS 更新；.gitattributes 管理 LF / 二进制图片，.gitignore 补本地 VS Code / 库缓存 / 坏配置。

```powershell
cargo metadata --locked --offline --no-deps --format-version 1
cargo metadata --locked --offline --format-version 1 --filter-platform x86_64-pc-windows-msvc
cargo fmt --all -- --check
git diff --check
git fsck --full
git merge-base --is-ancestor master dev
git check-ignore --no-index target/release/yyplayer.exe third_party/mpv/windows-x64/libmpv-2.dll .env settings-local.json library-cache.json settings.library.json settings.corrupt-123.json .vscode/settings.json
```

结果：五个 workspace 许可均 GPL-3.0-only；fmt / diff 检查通过，Cargo.lock 未改变。Git fsck 未发现缺失或损坏对象；报告少量无引用 blob / tree，保留恢复用途，不改写或清理历史。这些无引用对象不属于分支发布历史。

发布准备前，全分支历史有 348 个不同 blob，最大 1,381,484 字节（docs/video-ui.png 自有验收图）；没有大型编译产物。按 blob 读取文本、文件名输出的扫描未发现常见私钥头、GitHub token、AWS key 或 OpenAI 项目密钥标记。已跟踪文件无 exe / DLL / PDB / archive / 音视频 / 常见私钥扩展。模式扫描有范围限制，不宣称覆盖所有凭据类型。新增文件为源码许可、文本元数据 / 文档 / Git 规则，不含本机 registry 绝对路径；生成过程的临时 JSON 已删除。

所有配置 / 视频素材 / runtime / target 仍被忽略，Cargo.lock、manifest、LICENSE、README 等保留。现有成品与固定 DLL 未修改，没有重编译，未产生新编译缓存。本轮只有许可、元数据、文档与 Git 整理，没有重复 Clippy / tests / 播放 / 硬件验收；上一轮完整验证见 window-artwork.md。

## 分支与发布

master 原先是 dev 的祖先，采用 `git switch master`、`git merge --ff-only dev`，不改写提交、不需要解决冲突；最新功能与本轮准备一起进入 master，dev 保留相同版本，旧 main 保留历史视频基线。当前停留 master、提交后状态干净、远端仍为空。用户打开 `E:\SourceFiles\rust\yyplayer`，运行 VS Code Publish to GitHub 并选择公开仓库，操作见 ../PUBLISHING.md。

源码公开准备完成；GitHub 仓库 / origin / push 未执行。打包 exe + DLL 的发行许可材料、USB / HDR / macOS / Linux 资格仍待补，不标 S00 的全部许可 / 发行任务完成。
