# 0.5.2 SQLite 音乐库迁移验收

日期：2026-10-12。开发起点为 dev / `5618134`，本轮修改留在 dev 工作区，未提交、合入 master 或推送。用户要求完整迁移音乐库索引并更新为 0.5.2，不做存储方案性能对比；OpenGL 保持，未继续 Vulkan 探索。决定与恢复规则见 [ADR 0012](../adr/0012-sqlite-music-library.md)及[歌单指南](../MUSIC_PLAYLISTS.md)。

状态：Done（Windows 实现、数据 / 播放回归与最终 Release 范围）。

## 存储与迁移

- 音乐标签索引、目录 / 单独文件 / 排除项、歌单、有序成员和我喜欢进入 `*.library.sqlite3`。MusicStore 封装 schema、路径编码、参数绑定与事务，连接只由后台磁盘 worker 使用；core / UI / Engine / GL 不依赖 SQL。视频索引仍独立保存，播放队列仍是内存会话。
- 固定 rusqlite 0.40.2，关闭默认 features，仅 bundled；libsqlite3-sys 0.38.2 内置 SQLite 3.53.2，静态链接，不增加外部 SQLite DLL。Cargo.lock 新增五个包，保留完整跨平台解析图；Windows normal / build 依赖许可元数据已更新。Slint 1.17.1 与固定 mpv runtime 未变。
- 首次事务导入旧设置中的音乐配置 / 歌单与旧音乐 JSON 索引，保存原设置的字节备份，旧索引不删除或改写。新设置格式 2 不再写 library / playlists；后续以数据库为准，拒绝损坏、未来版本、外来库和缺失库，不静默清空收藏。
- 路径 BLOB 保留 Windows UTF-16LE / Unix 原始字节；稳定歌曲 ID 用于歌单外键，专辑键含歌手。扫描按路径 UPSERT，退休记录不删除歌单成员；取消 / 无效批次回滚。歌单重命名不重建成员行，名称 / 位置交换原子提交。
- 启动完成前的导入有界排队，初始化前的保存保留数据库音乐配置；退出排空有界队列并保存最新快照。运行中写库失败后保留当前会话编辑，原位重试不以旧快照覆盖；替换为其他备份需要退出后恢复并重启。
- 数据库音乐事务与 JSON 偏好是两个提交；若 JSON 替换失败，会报告音乐库已提交、应用设置保存失败，未宣称二者跨文件原子提交。

[迁移与重启证据](sqlite-music-library/migration.json)来自本轮 GUID 独立配置和自有 FLAC，28 阶段真实 Slint 操作完成后，新 controller 恢复「夜航收藏」及其成员，两个会话队列为空。旧设置备份与旧索引的字节保持一致。[只读数据库审计](sqlite-music-library/database-audit.json)确认 application_id=0x5959504c、schema=1、quick_check=ok、外键检查无错误；4 首索引歌曲、2 个歌手 / 同名专辑分组、2 个歌单和 1 个成员。音乐字段已从偏好 JSON 移除。

## 检查与真实回归

环境：Windows x64 MSVC，rustc / cargo 1.99.0；GUI / 硬件场景串行，使用 target 下自有素材和独立配置，不访问用户 APPDATA 设置或用户媒体。

| 检查 | 结果 |
| --- | --- |
| cargo fmt --all -- --check | 通过 |
| cargo clippy --locked --offline --workspace --all-targets -- -D warnings | 通过 |
| cargo test --locked --offline --workspace --lib --bins | 70 项通过：core 20、mpv 9、platform 1、UI 1、app 39；既有 HTTPS 环境测试 1 项 ignored |
| Test-Collections -SkipBuild | [28 阶段](sqlite-music-library/collections.json)，分类 / 多选 / 歌单 / 四种顺序 / 独立队列 / EOF / 抽屉 / 三尺寸 / 重启通过 |
| Test-UiCustomization -SkipBuild | [22 阶段](sqlite-music-library/ui-customization.json)，80 行导入、缩略图、列宽保存、字体、SRT / ASS、原生窗口与静默保存通过 |
| Test-LibraryTheme -SkipBuild | [13 阶段](sqlite-music-library/library-theme.json)，目录去重、筛选、真实音频、移除不停止、重扫 / 取消、SQLite 配置与四种主题通过 |
| navigation-ui / ui-feedback / audio-ui | [导航 11 阶段](sqlite-music-library/navigation-ui.json)、[反馈 17 阶段](sqlite-music-library/ui-feedback.json)及五组音乐页布局通过 |
| Test-VideoLibrary -SkipBuild | [十次 renderer 释放 / 重建](sqlite-music-library/video-library.json)、恢复位置、取消、移除、正常暂停 / EOF 重播和新 controller 恢复通过；[SQLite 音乐库仍为空](sqlite-music-library/video-isolation.json) |
| Test-Video -SkipBuild -EvidenceDirectory target/sqlite-video | [控件 / 暂停重载](sqlite-music-library/controls-playback.json)、[软件解码](sqlite-music-library/software-playback.json)、保存与退出通过 |
| Test-Render -SkipBuild | [三场景](sqlite-music-library/render-summary.json)，实际桌面 GL 3.3，shader 编译 / 链接成功，硬解与去色带图像通过 |
| Test-WindowsPackaging / Test-WindowsPackagingLauncher | Windows PowerShell 5.1 下 [7 项](sqlite-music-library/packaging.json) / [10 项](sqlite-music-library/packaging-launcher.json)隔离 fixture 通过，检查 0.5.2 版本、失败 / 重试、旧 EXE / 包保护与入口 |

新增 13 项数据 / 生命周期测试覆盖迁移幂等、备份、原始路径、稳定 ID、移除保留成员、事务取消与非法整数、名称交换 / 顺序、外键与容量、忙 / 只读、损坏 / 未来 / 外来库、缺失库、启动保存、队列满退出、失败重试及会话编辑保留。Windows 非法 Unicode UTF-16 路径经真实偏好保存和数据库重开无损恢复；Unix 对应 cfg 测试本轮未运行。

实际 GPU 为 AMD Radeon 860M，GL_VERSION 为 `3.3.0 Core Profile Context 26.8.1.260810`。自有静态 4K HEVC，实际 d3d11va-copy，三个场景各 148 帧；[硬解](sqlite-music-library/render-hardware-metrics.json) / [去色带](sqlite-music-library/render-deband-metrics.json)相对软件参考平均 RGB 差为 0.062545 / 0.225618，坏像素比例均 0。本项是图像正确性回归，不是存储或图形性能比较，也不证明 AMD 独显资格。

初次 UiCustomization 超时暴露了数据库尚未就绪时启动导入被忽略的问题，已修正为有界排队并完成最终复跑。第一次 VideoLibrary 超时是旧 PowerShell 把 UTF-8 中文测试文件名读取为其他字符，生成额外自有副本；仅删除 target 中该次的两个错码副本，改用 PowerShell 7 后完整通过，未修改行数 / 超时断言。

## Release 与限制

`cargo build --locked --offline --release -p yyplayer-app --bin yyplayer` 通过（4分32秒）。五个 workspace crate、Cargo.lock、EXE FileVersion / ProductVersion 均为 0.5.2。最终 `target/release/yyplayer.exe` 为 32,204,800 字节，SHA-256 `f095eb20048b008df88aea5290ba90867fc9a671354f3cd15fb499bcce1b2cca`，见[产物信息](sqlite-music-library/release-artifact.json)。

新建 GUID 独立配置分别运行 4 秒：[空闲启动](sqlite-music-library/release-idle.json)保持 Idle；[自有 FLAC 播放](sqlite-music-library/release-audio.json)为 Playing、实际 WASAPI / 48kHz，位置超过 2 秒。二者正常退出 exit0、ready=true、music_database_ready=true，error / render_error 空，libmpv renderer 初始化与视频帧数均为 0，新设置格式为 2，无 library / playlists。[两个新库的审计](sqlite-music-library/release-databases.json)确认 schema / 类型标识、完整性、外键与我喜欢初始化。固定 mpv v0.41.0-1087-ge470f8986，未使用 YYPLAYER_MPV_LIBRARY 绕过 hash。

首次 Release 启动检查的环境清理留下了空值变量，触发自定义 libmpv 空路径和测试动作；SQLite 已正常初始化。改为删除进程级变量后，同一 EXE 的上述两场景通过，没有改动应用源码或重编译掩盖结果。隔离打包测试使用小型 fixture，本轮未生成完整应用 ZIP / Setup 或执行安装 / 卸载。

音乐库 / 单队列仍为 20000 首，歌单最多 128 个 / 合计 20000 成员；应用 JSON 限 2MB，歌单成员不占该配额，旧索引导入仍限 32MB。界面继续按 revision 缓存快照并在内存筛选 / 分组，未新增分页 / FTS、实时监听或文件改名追踪，不声称标签扫描已增量化。

本轮未运行 Linux / macOS 编译与真机、AMD 独显 / 其他驱动、多 DPI / 多屏 / 物理输入、DAC / 位准确 / HDR 资格；既有运行时发行闭包限制保持。下一步在 Linux 用同一自有 fixture 验证 Unix 路径迁移、歌单重启和包内 bundled SQLite 构建；Git 操作按后续明确授权执行。
