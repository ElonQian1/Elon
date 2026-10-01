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
- 已实现的入口与现场验收：[安装与验收](studio-cache-operations.md)。
- 完整目标：[工作室分层构建缓存方案](design/studio-build-cache-v1.md)，目前部分实现。

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
7. 已启用机器 profile 时，工具先检查本地容量，仅在明确允许时选本机独占 UNC 根。
   当前用户的跨项目预约已经接入 Cargo 包装器；不能当作跨机器配额。
   远程对象服务、代编译和非 Rust 适配器均需各自部署验收，不能由临时脚本冒充。
8. 输出实际配置与结果，区分“文档可用”“本机已安装”“跨机已验收”。

命令语义不能混用：`adopt-project` 默认预演，但 `install` 不传 `-Apply` 仍有安装写入。
`-SkipCacheGc`/关闭自动 GC 不绕过 Cargo 包装器的容量接纳。
直接调用裸 Cargo 没有平台预约保障；应使用项目包装器或平台 `run`。

各项目的 AGENTS 或其他 AI 入口应链接这份权威文档的受信版本/本地安装副本。
启用本机 control root 后，安装副本位于该目录的 `docs/studio-cache-ai-entry.md`。
不要复制整份政策到每个项目；平台升级时统一分发并校验版本。
非 Rust 项目使用各自原生缓存适配器；目前不能假定所有语言已接入。
