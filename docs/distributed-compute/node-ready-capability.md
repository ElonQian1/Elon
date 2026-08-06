---
title: 节点 ReadyCapability 健康证明边界
status: current
reviewed_at: 2026-08-07
owners: node, compute
---

# 节点 ReadyCapability 健康证明边界

## 1. 目标

`ReadyCapability` 只表示某个节点插件在一个很短的时间窗口内具备技术执行条件。它不是安装完成标记、市场报价、可预留容量、账户授权或商业 `ComputeOffer`。

当前代码状态为“远程 canonical 基线已编译验证、本批安全加固未重新编译、生产路径未接线”：staged 候选的进程内健康评估、成功与终止失败观察的规范校验、不可变健康回执 Store、失败 quarantine Store、cleanup authorization Store 及进程内不确定结果恢复均已通过 `elon-pc-node` 基线编译。当前增量只加固 cancellation、exact receipt/owner chain、PlanApply/replay 和 schema fence，按架构铺设策略未重新编译或执行事务夹具。真实 Sidecar 探针、生产本地数据库、NodeRuntime/Host 接线及控制面上报仍未完成。

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

## 5. 候选预热评估合同

staged 候选现可进入独立的进程内健康评估状态机。入口消费 `StagedComputePluginCandidateArchive`，把原文件句柄保管权与签名 Manifest、安装身份、staging receipt、候选 token、staging run、release、可执行入口和 Runner SHA-256 固定在同一个不可 Clone、不可序列化的对象内。入口不接受普通路径、裸候选 ID 或调用方拼装的 staged 布尔值。

每次 Host 侧探针观察必须提供成功或失败终态、耗时、响应摘要和失败原因码。状态机限制单次超时、探针总数、原因码数量以及 Manifest 声明的连续成功/失败阈值，并用前一摘要、严格序号和本次观察形成追加式 JCS/SHA-256 transcript。达到失败阈值后不能继续伪造成功；达到连续成功阈值后也不能继续改写历史。

健康评估只有在 authenticated trusted-time observation 的单调时间严格晚于最后一次探针，且安装身份和取消门卫仍然有效时才能封存。输出绑定完整探针摘要、staging receipt、release、Runner、健康参数、规范原因码、可信时间证明和最长五分钟有效期，并继续持有原 staged custody。`ValidatedCandidateHealthPublication` 是健康 Store 事务的线性输入，本身仍不是耐久健康回执、installed 状态、promotion permit、ReadyCapability 或商业算力证明。

当前 `CandidateHealthProbeObservation` 仍是 Host 进程内部观察，不证明真实 Sidecar 已经运行，也不替代尚未实现的认证 IPC、进程隔离、响应大小限制和探针调度器。调用方不能据此宣称插件已经可执行。

## 6. 候选健康回执 Store

存储前必须进行无副作用的 fresh authority read，并精确绑定当前 staged 候选、staging receipt、inventory revision/digest、authority epoch、process owner epoch 与 trusted-time high-water。只有这些事实未漂移时，Store 才能在同一 `BEGIN IMMEDIATE` 事务中写入不可变 `candidate_health_receipts` 并推进可信时间高水位。schema trigger 禁止回执更新或删除，并拒绝同一候选上有效期重叠的健康回执。

Store 返回成功后产生继续持有 staged 文件句柄的 `DurableCandidateHealthPublication`。如果写入结果不确定，调用方只能使用进程内 recovery key 读出稳定的 `NotCreated` 或 exact `Recorded`；身份碰撞、fence 漂移或时间回退均失败关闭。exact `Recorded` adoption 会再做 fresh read，并从保留句柄重新哈希 staging 文件和 seal；`NotCreated` 只是结果证明，不恢复写入许可。该耐久结果仍不会修改槽位、安装或激活代次，也不会生成 `ReadyCapability` 或商业回执。

## 7. 终止失败与 quarantine Store

达到 Manifest 声明的连续失败阈值后，评估不能再走健康成功封存入口。独立失败入口要求终止不健康状态、足量连续失败、至少一个规范原因码、完整 transcript、严格晚于最后探针和 staging 的 authenticated trusted time，并继续检查安装身份和取消门卫。输出为不可 Clone、不可序列化且继续持有 staged 文件句柄的 `ValidatedCandidateHealthFailurePublication`；它不是调用方可拼装的失败布尔值。

quarantine 授权先做无副作用 fresh authority read，精确核对 staged 槽、staging receipt、candidate owner、无未过期健康回执、inventory/state/authority/process fence 与 trusted-time high-water。Store 在单一 `BEGIN IMMEDIATE` 中推进可信时间，把槽从 `staged` 改为 `failed`，令 state、inventory 与 authority fence 各精确加一，并插入不可更新、不可删除的 `candidate_health_quarantine_receipts`。它保留 candidate owner、candidate pointer 和原 staged 文件 custody，不删除目录，也不授予下载、重试、安装或推广。

提交结果不确定时，进程内 recovery key 只能得到稳定 `NotCreated` 或 exact `Quarantined`。前者不恢复写许可；后者必须 fresh read 精确的 failed inventory/fence，再从保留句柄重新哈希 staging 文件和 seal，才可恢复 `DurableCandidateHealthQuarantine`。底层同句柄删除原语和私有 cleanup authorization Store 已形成代码，但尚未接入生产 Host、物理执行或 completion Store；quarantine 回执和 cleanup authorization 都不是清理成功证明。精确边界见 `node-plugin-candidate-cleanup.md`。

## 8. 尚未实现

- Sidecar 启动、预热和动态健康探针；
- Sidecar 到 Host 的认证 IPC、响应验证和真实探针调度；
- 失败候选的目录树线性清理编排、清理完成回执、重试授权与跨重启 custody 治理；
- 同时消费 staging 与健康回执的原子 installed/promotion 门卫；
- 发布前 Store fresh read、CAS/fencing 和不确定结果恢复；
- `ComputeReadyCapability` 的规范短 TTL 构建、认证上报和服务端验证；
- 共享关闭、排水、崩溃和撤销后的主动失效通知；
- 从技术就绪事实到 Provider、CapacityPool、Offer 和 Attempt 的生产接线。

在上述链路完成前，代码中存在健康证明类型不代表节点已经可被消费者 AI 调度或产生算力收入。
