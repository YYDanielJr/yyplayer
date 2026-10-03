# YYPlayer G08 PNG 图标

本目录中的透明背景 PNG 均由 [精修版 G08 SVG](../../../icon-concepts/liquid-08-orbit-disc.svg) 直接栅格化生成，颜色为 sRGB，保留 Alpha 通道。

可用尺寸：16、20、24、32、40、48、64、96、128、256、512、1024 px。文件名格式为 `yyplayer-<尺寸>.png`。

16–256 px 可用于应用界面、快捷方式及图标资源管线；512 / 1024 px 适合作为高分辨率源图。`yyplayer.ico` 汇集了 16–256 px 的 10 个帧，由 `scripts/Build-AppIcon.ps1` 从 PNG 生成，并用于 Windows EXE 与 NSIS 安装器 / 卸载器资源。
