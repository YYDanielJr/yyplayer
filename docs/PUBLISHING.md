# 用 VS Code 发布源码到 GitHub

准备日期：2026-10-02。用户已选择 **GPL-3.0-only**；本轮仅做本地源码仓库准备，GitHub 发布由用户在 VS Code 完成。

## 应打开的目录

在 VS Code 选择 **文件 → 打开文件夹**，打开：

```text
E:\SourceFiles\rust\yyplayer
```

这个目录包含 `.git`、Cargo.toml、Cargo.lock、README 和 crates，是完整仓库根目录。直接打开根目录即可识别现有 Git 历史与五个 Rust crate。

## 已完成的本地准备

- 最新功能和发布文档已合入 **master**，当前停留在 master，工作区干净。dev 保留同版本作为后续开发入口；重复的本地 main 已移除，原 main 的全部提交保留在 master 历史中。
- 根目录 LICENSE 为完整 GPLv3 标准文本，README 明确 **GPL-3.0-only**，五个 crate 继承 workspace 许可字段。保留 Cargo.lock；不自动发布 crates.io。
- THIRD_PARTY_NOTICES.md 记录 Slint 1.17.1 的 GPL 选择、固定 GPL 组合的 libmpv 与原创 SVG。docs/dependency-licenses.md 记录锁定 Windows normal / build 依赖元数据；不等同于二进制完整 notices。
- `.gitignore` 排除编译产物 / runtime DLL / 下载 archive、临时媒体 / 本地设置 / .env / dump、音乐库缓存和损坏配置备份；VS Code 本机配置默认忽略，extensions / tasks 可按需要提交。`.gitattributes` 固定源码文本 LF 和 PNG 二进制。
- 已检查工作区 / 历史大文件 / 常见密钥标记；没有把用户媒体、截图附件或 APPDATA 配置加入仓库。Git 历史最大 blob 约 1.32 MiB，是自有验收图片，无大型编译产物。
- 没有配置 Git 远端、创建 GitHub 仓库或执行 push。当前发布的是源码；本地 target/release/yyplayer.exe 和 libmpv DLL 均不上传。

## VS Code 操作

1. 打开上述目录，检查左下角当前分支为 **master**，源代码管理没有待提交改动。
2. 按 **Ctrl+Shift+P** 打开命令面板，输入并运行 **Publish to GitHub**（发布到 GitHub）。需要时按提示登录自己的 GitHub 账号。
3. 输入可用仓库名，例如 `YYPlayer`，选择 **Public / 公开仓库**。现有提交已准备好；出现文件选择提示时保留源码、LICENSE、Cargo.lock 和文档，遵循现有忽略规则。
4. 完成后到 GitHub 确认 master 是默认分支、最新 README / LICENSE 可见，VS Code 已添加 origin 并设置 master 的 upstream。
5. 需要公开开发分支时再切到 dev，使用 Publish Branch 发布；日常新代码在 dev 完成、验证后合入 master。本地只保留 master / dev 两个分支。

如果 GitHub 上已经手动创建了空仓库，使用 **Git: Add Remote** 添加该仓库 URL（名称 origin），再 Push master；不需要再次创建另一个仓库。首次空仓库不要另生成冲突的 README / LICENSE 提交。官方操作说明：[VS Code 仓库与远端](https://code.visualstudio.com/docs/sourcecontrol/repos-remotes#publish-to-github)。

## 发布后的本地复查

```powershell
git status --short
git branch -vv
git remote -v
git log -1 --oneline
```

这些命令应显示工作区干净、master 关联 origin/master，远端为你自己的 GitHub 仓库。发布前远端为空是预期状态。Git 的提交历史也随 master 上传，而不是把目录内所有文件打包上传。

## 后续二进制发行

### master 推送的 Windows x64 构建

`.github/workflows/windows-release.yml` 在每次向 `master` 推送时使用 `windows-2022` x64 MSVC runner 构建，并可从 Actions 页面手动运行。工作流校验固定的 libmpv archive / DLL hash，再生成两个提交号命名的文件：

- `YYPlayer-<版本>-<提交号>-windows-x64-portable.zip`：解压后从 `yyplayer.exe` 启动，运行时 DLL 位于 `runtime/`。播放器配置仍保存在当前 Windows 用户的 `%APPDATA%`。
- `YYPlayer-<版本>-<提交号>-windows-x64-setup.exe`：NSIS 安装器，要求 x64 Windows 和管理员安装权限，默认安装到 64 位 Program Files，创建开始菜单快捷方式并提供卸载；卸载不删除用户配置。

两份实际文件会一起出现在该次 Actions run 的 `YYPlayer-windows-x64-<commit>` artifact 中，保留 30 天。它们是逐提交 CI 构建产物，不会创建 GitHub Releases 页面上的版本发布；源代码发布仍由用户 push。NSIS 被选为安装器，因为其开源许可适用于商业和非商业用途，Windows 2022 hosted runner 已预装 NSIS 与 7-Zip。[NSIS 官方许可](https://nsis.sourceforge.io/Docs/AppendixI.html)、[GitHub Windows 2022 runner 清单](https://github.com/actions/runner-images/blob/main/images/windows/Windows2022-Readme.md)

NSIS setup 本身由 NSIS 编译，但它只在 x64 Windows 上安装 x64 app。Windows 2022 runner 镜像和 NSIS 版本可能更新，工作流会在找不到 7-Zip / `makensis.exe` 时明确失败。

构建包随附本项目 GPL 与当前第三方许可记录、固定 mpv manifest、源码提交链接和 NSIS 构建工具归属说明。`THIRD_PARTY_NOTICES.md` 明确当前还不是 libmpv 构建内部所有 FFmpeg / codec 组件的完整 notices；对应源码 / 构建脚本安排与 DLL 闭包也未完成最终发行审计。因此这些自动产物用于本仓库的提交构建，不将其描述为完成了二进制发行合规审计。正式 GitHub Release 还需先补齐上述事项并做干净 Windows 安装 / 卸载与运行时依赖验收，参见 [运行时说明](../third_party/mpv/README.md)。CI 编译包也不代表 USB 位准确、HDR 或 macOS / Linux 资格。
