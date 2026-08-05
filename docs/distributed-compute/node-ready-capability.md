---
title: 节点 ReadyCapability 健康证明边界
status: current
reviewed_at: 2026-08-05
owners: node, compute
---

# 节点 ReadyCapability 健康证明边界

## 1. 目标

`ReadyCapability` 只表示某个节点插件在一个很短的时间窗口内具备技术执行条件。它不是安装完成标记、市场报价、可预留容量、账户授权或商业 `ComputeOffer`。

当前代码状态为 `implementation_uncompiled`：健康证明校验合同已经写入，但未编译、未运行健康探针、未接入本地权威 Store，也未向控制面上报能力。

## 2. 已关闭的错误入口

旧接口由调用方传入 `health_is_fresh: bool`，本机任意代码都可能在没有可信时钟证明的情况下声称健康记录仍然有效。该入口已经删除。

新入口必须同时取得：

- 完整 `ComputePluginInventorySnapshot`，而不是拆散的共享开关和插件记录；
- 规范安装身份，用于阻止另一安装实例的时间证明被重放；
- 经挑战、签名和安装身份绑定后产生的 `ComputePluginTrustedTimeObservation`；
- 要发布技术就绪事实的精确 `plugin_id`。

## 3. 验证闭包

入口首先复用库存整体校验，再要求共享已开启、插件期望存在且启用、准入状态为 allowed、活动槽为 installed、运行态为 ready，并核对正数的安装、激活和运行代次。

健康记录必须精确绑定当前活动槽、Runtime generation 和 Runner SHA-256；权限授权、Manifest、package 和 Runner 摘要都必须是规范 SHA-256。健康观察时间不得早于当前记录状态变更时间，不得晚于可信现在，失效时间必须严格晚于可信现在，且单次健康有效期最多五分钟。

`observation_digest` 使用 JCS/SHA-256 覆盖插件 ID、安装与激活代次、健康状态、运行代次、槽、Runner、规范排序的原因码及观察/失效时间。调用方不能只替换 `expires_at` 或 Runner 后继续复用旧摘要。

## 4. 线性输出

校验成功只产生不可 `Clone`、不可序列化、字段私有的 `ValidatedComputeReadyPublication`。该对象封存已经核对的插件记录、库存和策略修订号，并拥有原始可信时间观察；普通 DTO、墙钟时间或裸布尔值不能构造它。

后续短 TTL capability 构建器必须消费该对象，并在本地权威 Store 中重新核对库存 revision、authority epoch、共享状态和活动运行代次。Store 变化、排水、停用、撤销、重启或健康过期均应阻止发布或使既有技术就绪事实失效。

## 5. 尚未实现

- Sidecar 启动、预热和动态健康探针；
- 健康观察的权威签发及写入事务；
- 发布前 Store fresh read、CAS/fencing 和不确定结果恢复；
- `ComputeReadyCapability` 的规范短 TTL 构建、认证上报和服务端验证；
- 共享关闭、排水、崩溃和撤销后的主动失效通知；
- 从技术就绪事实到 Provider、CapacityPool、Offer 和 Attempt 的生产接线。

在上述链路完成前，代码中存在健康证明类型不代表节点已经可被消费者 AI 调度或产生算力收入。
