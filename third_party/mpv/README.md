# libmpv 运行时边界

当前 Windows x64 开发预览已动态加载真实 libmpv。固定构建为 **20260928-git-e470f8986e**，实际版本 `mpv v0.41.0-1087-ge470f8986`、client API 2；来源是 mpv 官方安装页推荐的 shinchiro Windows 构建。此构建不是稳定版号对应的源码 release。

仓库提交 `manifest.json` 的下载 URL、archive / DLL SHA256。运行 `powershell -ExecutionPolicy Bypass -File scripts/Get-Mpv.ps1`，脚本下载、校验 archive、优先用 7-Zip 解压（不可用时尝试 Windows tar）、校验 DLL；失败即停止。下载和提取目录 `windows-x64/` 已忽略，不提交大文件或自动下载到用户系统目录。解压需要 7-Zip 或支持该 archive 的 Windows tar。

Windows loader 只接受可执行文件旁 `runtime/libmpv-2.dll` 或开发目录中的固定 DLL，校验 hash，使用受控 Windows DLL search flags。Linux 开发 / deb 使用绝对系统 libmpv2 路径，AppImage 使用包内 manifest / SHA-256 核验；说明见 [Linux 指南](../../docs/LINUX.md) 和 `linux.json`。显式 `YYPLAYER_MPV_LIBRARY` 路径只供开发，同时绕过 checksum；macOS 尚未资格验证。缺少库 / 符号 / API 不匹配时显示错误，不能伪造播放成功。

Windows archive 提供 `include/mpv/` 头文件，用于本轮 FFI / Render API 核对；bindings 位于 `crates/player-mpv/src/ffi.rs`。Windows `unsigned long` 必须用 `c_ulong`；clock API 必须传 mpv handle。固定源码 render target 单位与头文件注释不同，详见 [ADR 0001](../../docs/adr/0001-libmpv-video.md)。升级必须同步更新 manifest、核对 ABI / 选项、重新运行时序 / GPU / 退出测试。

当前已有 Windows portable ZIP / NSIS 与 Linux deb / AppImage 开发预览包，用户已确认共用代码的 Windows 编译打包通过；构建包不代表正式发行资格。本构建为 GPL 组合，应用已选择 GPL-3.0-only；正式发布前仍需补齐实际依赖 notices、构建配置与对应源码取得方式，并验证 DLL 依赖闭包。这里的摘要不是已完成许可 / 分发审计。来源：[mpv 安装说明](https://mpv.io/installation/)、[固定构建](https://github.com/shinchiro/mpv-winbuild-cmake/releases/tag/20260928)、[构建仓库](https://github.com/shinchiro/mpv-winbuild-cmake)。
