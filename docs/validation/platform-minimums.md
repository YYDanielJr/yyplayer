# 系统最低版本推测与链接核查

日期：2026-10-03。任务：解释 Rust 静态 / 动态链接，按现有 API 和实际包推测 Windows、Ubuntu、Debian 最低版本，写入 README。仅更新文档；没有改变兼容路径、安装系统库或重打包现有产物。

## 代码与产物证据

- Cargo.toml 的目标为 Windows x64 MSVC / Linux x86_64 GNU，没有启用完全静态 C runtime；Rust crate 的静态链接与宿主 C / OS 库的动态链接是不同层次。[Rust 链接规则](https://doc.rust-lang.org/reference/linkage.html)。即便完全静态，CPU、内核、图形 / 音频驱动与安装包格式仍有约束。
- player-mpv/ffi.rs 通过 libloading 解析 libmpv 与 Render API，包括 mpv_get_time_ns；player-ui/presenter.rs 使用它匹配 render 时钟。上游 [client API 变更记录](https://github.com/mpv-player/mpv/blob/v0.41.0/DOCS/client-api-changes.rst) 将纳秒时间 API 列于 2.2（mpv 0.37）。这项 C 符号不是 0.41 才有；当前 Linux 音频初始化 / 日志确认以 0.41 验证，不能仅据 C API 降低完整功能的运行时要求。
- player-platform 的注册表、GDI、DWM 调用与 app 的 MoveFileExW 不要求 Windows 11；可选 DWMWA_WINDOW_CORNER_PREFERENCE 的枚举值自 Build 22000 支持，代码记录 HRESULT 并允许失败。[Microsoft API 要求](https://learn.microsoft.com/en-us/windows/win32/api/dwmapi/ne-dwmapi-dwm_window_corner_preference)。Winit 0.30.13 自身文档允许更旧 Windows，但整个项目还受 [Rust MSVC Windows 10 下限](https://doc.rust-lang.org/rustc/platform-support/windows-msvc.html) 和 [mpv 0.41 Windows 10 1607 下限](https://github.com/mpv-player/mpv/blob/v0.41.0/README.md) 限制。
- Linux 外观 ReadOne 失败时尝试旧 Read，偏好缺失可回退；rfd 使用 portal，字体从后台 fontdb 枚举。锁定图没有启用 wayland-sys/libwayland_client_1_23 的强制新符号。这些源码 API 不能唯一确定一个 Ubuntu / Debian 年号下限，需要对具体构建 / 依赖 / compositor 验证。

## 本机检查

```bash
file target/release/yyplayer
ldd target/release/yyplayer
readelf --wide --dyn-syms target/release/yyplayer
readelf --version-info target/release/yyplayer
dpkg-deb --field dist/yyplayer_0.0.1+linux.6e9e9c42e2d7_amd64.deb Depends
cargo tree --locked --offline -e features -i wayland-sys
git diff --check
```

实测 ELF 为 dynamically linked，解释器 /lib64/ld-linux-x86-64.so.2。直接依赖 libfontconfig.so.1、libgcc_s.so.1、libm.so.6、libc.so.6；ldd 不列出后续 dlopen 的 libmpv / GL / 桌面音频库。未定义全局符号 acosf@GLIBC_2.43、atan2f@GLIBC_2.43 为真实 ABI 下限，不只是打包脚本保守声明。deb 的 Depends 为 libc6 >= 2.43、libmpv2 >= 0.41.0 及图形 / 字体库。AppImage 排除宿主 glibc / GL 驱动，两包共用同一主程序，均不能直接降低到老 glibc。

仓库事实核对：[Ubuntu 26.04 libmpv 0.41](https://packages.ubuntu.com/resolute/libmpv2)、[Ubuntu 24.04 libmpv 0.37](https://packages.ubuntu.com/noble/amd64/libmpv2)、[Debian 13 libc6 2.41](https://packages.debian.org/trixie/libc6) / [libmpv 0.40](https://packages.debian.org/trixie/libmpv2)、[forky libc6 2.43](https://packages.debian.org/forky/libc6) / [libmpv 0.41](https://packages.debian.org/forky/libmpv2)。[forky 尚为 testing](https://www.debian.org/releases/forky/)，不能把它写成已验收的稳定版。Arch 的 [makepkg / pacman 包格式](https://wiki.archlinux.org/title/Creating_packages) 独立于 ELF 是否静态。

结论：核心 Windows 推测 10 1607，原生圆角 Windows 11 Build 22000；现有 Linux 包的 Ubuntu 基线 26.04，Debian 候选为目前的 forky。Debian 13 / Ubuntu 24.04 不能直接使用本次包；源码重编译并配套新版 libmpv 后可以作为降基线验证对象，而非已支持版本。旧 Windows、Debian、Arch、完整 PE / bundle 导入闭包和干净安装均没有新增验收证据。下一步如需降低基线，在候选发行版构建完整依赖并重新验收 UI / GPU 图片 / 音频确认，再发布独立包。
