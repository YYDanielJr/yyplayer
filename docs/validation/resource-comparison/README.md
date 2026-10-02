# 2026-10-02 资源测量原始记录

视频库 / 按需渲染一轮的 Release 短样本，汇总与方法见 [视频库报告](../video-library.md)。清理 target 前归档 WorkingSet / PrivateBytes 原始样本与当时测量脚本，不包含用户媒体或配置 / 设备诊断。脚本原路径为 target/resource-comparison，运行前需要重新生成相同自有 WAV 和独立配置，并选择 hash 匹配的旧 / 新可执行文件；旧 exe 已被新版构建覆盖，保留脚本不表示当前可以重现旧二进制。最新 UI 高度修改没有重测内存。
