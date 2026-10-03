# AppImage runtime 记录

本包使用 SHA-256 固定的 appimagetool 1.9.1 AppImage 内嵌 runtime，
`--appimage-version` 返回 `caf24f9f712084686bfc24a70b75e50df0aefb9c`。
对应源码与构建脚本：
https://github.com/AppImage/type2-runtime/tree/caf24f9f712084686bfc24a70b75e50df0aefb9c

保留该提交完整 LICENSE 和 BUILD.md。其构建脚本固定 libfuse 3.15.0、
squashfuse 0.5.2，并使用 Alpine 3.21 的 musl / zstd / zlib / mimalloc 静态库。
Makefile 的实际链接列表包含 mimalloc，因此本包同时保留其 MIT 原文。
各文件来自上游对应版本的 LICENSE / COPYRIGHT；发行库 ELF 的精确版本
与完整 copyright 则独立列在 runtime-manifest.json。

上游预构建 runtime 未给出每个 Alpine 静态包的完整版本 / 对象清单。
因此本轮包是本机验证的开发预览，不能把这些记录称为完成全部二进制
发行许可资格。公开正式发行前，须补齐该 runtime 的精确静态闭包和
可重链接的对应源码 / 构建安排，或从完整固定的源码组合重新构建 runtime。
不要执行 BUILD.md 中的 chroot 示例去修改开发机；本项目脚本不运行它。
