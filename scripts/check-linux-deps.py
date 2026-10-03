#!/usr/bin/env python3
"""Read-only Ubuntu dependency audit. Never invokes sudo or modifies apt state."""
import argparse
import json
import shutil
import subprocess
from pathlib import Path

PACKAGES = {
    "libmpv2": "播放内核",
    "mpv": "核实运行时 AO / GPU 能力",
    "ffmpeg": "生成自有音视频验收素材",
    "patchelf": "AppImage 动态依赖打包",
    "libudev-dev": "Linux 设备构建边界",
    "libasound2-dev": "ALSA 硬件资格验证",
    "libxkbcommon-x11-dev": "X11 键盘支持",
    "libx11-xcb-dev": "X11 支持",
    "xinput": "X11 输入诊断",
    "pulseaudio-utils": "桌面输出与恢复验证",
    "xvfb": "独立 X11 回归",
    "weston": "独立原生 Wayland 回归",
    "libfuse2t64": "兼容 type-2 AppImage 用户环境",
    "build-essential": "Rust 原生链接工具链",
    "pkg-config": "开发库检查",
    "libfontconfig-dev": "字体开发库",
    "libwayland-dev": "Wayland 开发库",
    "libxkbcommon-dev": "键盘开发库",
    "libegl-dev": "GPU 渲染",
    "libgl-dev": "OpenGL 构建",
    "libx11-dev": "X11 开发库",
    "libxcursor-dev": "X11 指针",
    "libxi-dev": "X11 输入",
    "libxrandr-dev": "X11 显示",
    "xauth": "独立显示授权",
    "fonts-noto-cjk": "中文字体回退",
    "ca-certificates": "工具下载证书",
    "curl": "工具下载",
    "python3": "打包与验收脚本",
    "squashfs-tools": "AppImage 归档",
    "desktop-file-utils": "桌面入口校验",
    "xdg-desktop-portal": "原生文件选择及外观",
}

def audit():
    packages = {}
    for name, purpose in PACKAGES.items():
        result = subprocess.run(["dpkg-query", "-W", "-f=${db:Status-Status}\t${Version}", name], capture_output=True, text=True)
        fields = result.stdout.split("\t", 1)
        packages[name] = {"installed": fields[0] == "installed", "version": fields[1] if len(fields) == 2 else None, "purpose": purpose}
    return {"os": Path("/etc/os-release").read_text(), "architecture": subprocess.check_output(["dpkg", "--print-architecture"], text=True).strip(), "packages": packages, "tools": {name: shutil.which(name) for name in ["cargo", "rustc", "dpkg-deb", "patchelf", "ffmpeg", "mpv", "Xvfb", "weston"]}}

if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--json", action="store_true")
    args = parser.parse_args()
    result = audit()
    if args.json:
        print(json.dumps(result, ensure_ascii=False, indent=2))
    else:
        missing = [name for name, state in result["packages"].items() if not state["installed"]]
        print("缺少 apt 包：" + (" ".join(missing) or "无"))
        if missing:
            print("sudo apt install --no-install-recommends " + " ".join(missing))
