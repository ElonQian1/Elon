---
title: 工作室缓存 AI 接入入口
version_status: current
reviewed_at: 2026-10-01
owner: platform
---

# 工作室缓存 AI 接入入口

供工作室各项目的 Codex、Claude、Copilot 和其他 AI 开发代理按需引用。
这是接入说明，不会自动配置电脑或授予远程执行、删除、迁移权限。

## 先读边界

- 当前工具与入口：[Rust 缓存平台](rust-cache-platform.md)。
- 子项目接入：[跨项目按需共享](rust-cache-on-demand-adoption.md)。
- 现有 UNC 限制与归档：[网络缓存与冷归档](rust-cache-network-storage.md)。
- 下一阶段目标：[工作室分层构建缓存方案](design/studio-build-cache-v1.md)，状态为 proposed。

## 每个项目的最小约定

1. Rust 项目优先调用仓库自己的 `scripts/rust-cache.ps1`；没有时查找当前用户已安装的
   `%LOCALAPPDATA%\Elon\bin\rust-cache.ps1`。缺失即报告需要安装，不自行猜共享路径。
2. 首次接入和长构建前运行入口的 `doctor -ProjectRoot <absolute-root>`。
   先检查安装指纹、实际缓存根、空间、活动写入者与已有等价构建。
3. 用 `adopt-project` 预演并按任务授权接入，提交项目清单和薄 wrapper；
   机器路径、凭据与缓存根留在本地配置。
4. 构建优先走项目已有包装器。没有时才用安装平台的 `run`；
   不自行复制缓存路由、锁或清理逻辑，不为每个会话新造永久分区。
5. 当前 UNC 根是单机独占。不同电脑不共写一个 Cargo/sccache 根；
   宿主不能把别人的 UNC 根换成本地路径来接管。
6. GC 先预演并遵守所属工具的授权与锁；不删除游戏、活动工作区、未提交源码或未知数据。
7. 网络不可用或空间不足时，按当前工具实际能力报告；
   尚未实现的自动分层、预约、远程执行不能由 AI 临时脚本冒充。
8. 输出实际配置与结果，区分“文档可用”“本机已安装”“跨机已验收”。

命令语义不能混用：`adopt-project` 默认预演，但 `install` 不传 `-Apply` 仍有安装写入。
现有 `-SkipCacheGc`/关闭自动 GC 会跳过部分容量预检；不应把它当作低磁盘通行证。
统一、独立于 GC 的容量接纳门禁属于下一阶段实现。

各项目的 AGENTS 或其他 AI 入口应链接这份权威文档的受信版本/本地安装副本。
不要复制整份政策到每个项目；平台升级时统一分发并校验版本。
非 Rust 项目使用各自原生缓存适配器；目前不能假定所有语言已接入。
