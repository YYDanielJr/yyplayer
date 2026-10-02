# 用 VS Code 发布源码到 GitHub

准备日期：2026-10-02。用户已选择 **GPL-3.0-only**；本轮仅做本地源码仓库准备，GitHub 发布由用户在 VS Code 完成。

## 应打开的目录

在 VS Code 选择 **文件 → 打开文件夹**，打开：

```text
E:\SourceFiles\rust\yyplayer
```

这个目录包含 `.git`、Cargo.toml、Cargo.lock、README 和 crates，是完整仓库根目录。直接打开根目录即可识别现有 Git 历史与五个 Rust crate。

## 已完成的本地准备

- 最新功能和发布文档已合入 **master**，当前停留在 master，工作区干净。dev 保留同版本作为后续开发入口；旧 main 是历史视频基线，保留但本次不使用。
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
5. 需要公开开发分支时再切到 dev，使用 Publish Branch 发布；日常新代码在 dev 完成、验证后合入 master。保留的旧 main 不必额外推送。

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

公开源码准备完成。携带 exe / DLL 的 GitHub Release 仍需完成固定 runtime 的完整第三方 notices、对应源码 / 构建信息、DLL 依赖闭包和发行验收，参见 [运行时说明](../third_party/mpv/README.md)。本轮不会把本地开发程序标为合格安装包，也未重新宣称 USB 位准确、HDR 或 macOS / Linux 已支持。
