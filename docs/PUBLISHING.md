# GitHub 发布准备

检查日期：2026-10-02。本文件记录仓库实际状态，不表示已经发布或完成二进制发行审计。

## 当前状态

- Git 已初始化，当前开发分支 **dev**；本轮完成后本地提交，未合入 master / main。本轮前基线为 bbd8023，master 保留音频基线 8467ca9，main 保留视频基线 4eca722。正式主分支需要先选定并合并 dev 的最终版本。
- 当前没有远端；没有创建 GitHub 仓库、设置 URL 或执行 push。
- `.gitignore` 已存在：排除 target / dist、下载运行时目录、日志、PDB / dump、原子保存临时文件、根目录本地设置 / .env 与系统杂项。Cargo.lock、manifest、源码和自有验收资料保留。
- 已用 check-ignore 检查 exe / runtime DLL / .env / 本地配置 / dump；已跟踪文件未发现播放器二进制、音视频或常见私钥扩展。历史最大 blob 为约 1.32 MiB 自有验证图，没有巨型编译产物。常见密钥标记的文件名扫描未发现命中；这不是对所有历史秘密的保证。用户提供的图片 / 音乐 / APPDATA 配置未加入 Git。
- 根目录尚无 **LICENSE**，workspace 也未指定 license。应用许可证由项目所有者确定后补齐；Slint 与固定 GPL 组合运行时的许可记录仍需按 README S00 完成。当前仅为源码和本地开发预览，不把 exe + DLL 标为完成发行包。

## 发布前操作

1. 选定源码许可证，补根目录 LICENSE 及 Cargo 许可字段，核对依赖和第三方素材记录。二进制 Release 还要完成 runtime 的 notices、对应源码 / 构建信息与 DLL 依赖闭包，参见 [运行时说明](../third_party/mpv/README.md)。
2. 选定 master 或 main 作为唯一正式默认分支，并将已验收的 dev 合入。当前各分支基线不同，不能仅推送旧主分支就当作最新功能已发布。
3. 在 GitHub 建立仓库，使用自己的仓库 URL 配置 origin；先检查 `git status --short`、`git diff --cached --stat` 和 `git ls-files`，确认没有用户媒体、凭据或运行时二进制。
4. 推送所选主分支；如需要继续开发再推送 dev。本轮没有自动执行这些外部操作。

本地可复查：

```powershell
git status --short
git branch -vv
git remote -v
git check-ignore --no-index target/debug/yyplayer.exe third_party/mpv/windows-x64/libmpv-2.dll .env settings-local.json crash.dmp
git ls-files
```

编译缓存和运行时无需上传；首次开发按 README 构建并使用受控下载脚本获取固定 libmpv。GitHub 源码发布与携带 runtime 的二进制发行是两个不同的交付步骤。
