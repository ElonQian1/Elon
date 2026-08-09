---
title: 分布式算力 Attempt Execution Plan V1
status: current
reviewed_at: 2026-08-09
owners: backend, node, ai-economy
implementation_status: implementation_uncompiled
---

# 分布式算力 Attempt Execution Plan V1

## 1. 当前状态

v212 在源码中增加 Provider-neutral、不可变的 Execution Plan 合同、五张追加式 SQLite 账本和 sealed Store producer。它补齐 v211 只有 `plan_id/schema/digest`、却没有计划权威真源的问题；本批没有编译、执行迁移或运行数据库。

当前没有 ReadyCapability V2 上报与服务端验真、endpoint verifier、Adapter registry、Artifact credential issuer 或任何 sealed 输入构造器。因此 Store producer 虽能在拿到不可伪造输入后完成单事务计划生成，生产入口仍不可达；v211 Start、accepted ACK 和远端执行仍保持关闭。

## 2. 五类耐久证据

| 账本 | 保存的事实 | 不代表 |
|---|---|---|
| `compute_execution_capability_receipts` | 已认证的节点、Provider endpoint 或 Adapter 技术能力；完整 runner/runtime/model/plugin、数值资源上限、route、来源和 TTL | Provider 自述、Offer 资源声明或市场容量本身可信 |
| `compute_artifact_access_receipts` | 绑定 Job、Reservation、Lease/fence、Provider、executor 和 route 的非 bearer 访问授权引用 | 数据已下载，或数据库保存了 URL、Token、密钥和原始 `location_ref` |
| `compute_attempt_execution_plans` | exact source、Attempt identity、完整 Start 投影、数值 ResourceGrant、Lease authority requirement 与时间窗口 | command 已派发或远端已准备 |
| `compute_attempt_execution_plan_accesses` | Plan 中有序访问集合与每个 receipt 的 exact digest | 访问凭据可跨 Attempt、executor 或 fencing 重用 |
| `compute_attempt_execution_plan_seals` | Plan、capability、访问集合和 ResourceGrant 的闭合摘要 | Start 获得发送授权；发送前仍需 fresh revalidation 与 durable claim |

所有 JSON 使用 RFC 8785 JCS 和独立 domain-separated SHA-256。相同 ID 仅允许逐字、逐字段 exact replay；不同内容冲突。Plan、receipt、join 和 seal 均禁止 update/delete/replace。

## 3. 能力来源不能互相冒充

Execution Capability 显式区分三类来源：

- `node_ready`：面向 `user_node/provider_endpoint`，必须绑定节点安装身份、inventory、install/activation/runtime generation、slot 与完整 Plugin Release；
- `provider_endpoint`：面向受管集群的 endpoint verifier，不能用 Provider 声明或 endpoint 地址代替认证证据；
- `adapter_execution`：面向 `managed_cluster` 或 `external_pool/server_adapter`，由未来 Adapter registry 与认证通道铸造；外部矿池不被强迫伪装成一龙节点，Plugin Release 可空，但 runner/runtime/model、数值合同上限、route 与 TTL 仍必须精确。

当前 Node ReadyCapability V1 只有 `resource_profile_digest`，没有 CPU、内存、显存、磁盘、进程和运行时数值上限，也没有规范构建、认证上报或服务端验证。因此它不能直接构造 v212 receipt；v212 不把一个摘要扩写成不存在的资源事实。

## 4. ArtifactAccess 与 ResourceGrant

Job 的 `location_ref` 只是来源定位，不是执行授权。ArtifactAccess 只保存非 bearer reference、授权摘要、read/write target、exact audience 和到期时间；短期 URL、Token、密钥或正文凭据必须由未来专用 authority 在带外交付，并在使用时复核当前 Lease/fence。

当前闭包只支持输入读取和 `purpose=result_write` 的结果写入。由于尚无 checkpoint read/write ArtifactAccess，producer 对任何非 `disabled` 的 checkpoint policy 固定失败；不能把结果命名空间或其他 purpose 的授权改名后复用。

ResourceGrant 不是调用者填写，也不是把 Job 最低需求改名。Store 按以下边界确定性求交并失败关闭：

1. Job 最低资源与输出/运行时上限；
2. 历史 Offer 的资源 profile 与 execution limits；
3. 当前认证 capability 的数值 ceiling；
4. exact Reservation、held Claim、Broker receipt 与余额预授权；
5. 数据分类、网络出站与 route enforcement。

任何最低需求高于 Offer 或 capability、资源值为负、用 `0` 冒充 unlimited、网络策略被放宽、ArtifactAccess audience/TTL 不覆盖计划窗口，均拒绝整个事务。

## 5. 单事务 producer

`produce_compute_attempt_execution_plan` 只接受字段私有、不可 Clone、不可反序列化且当前没有构造器的 `ValidatedComputeAttemptExecutionPlanInputs`。在一个 `BEGIN IMMEDIATE` 内必须：

1. 复算 capability 与 ArtifactAccess canonical digest，并 exact 插入或回读历史 receipt；
2. 重读 v169–v175 当前 Provider route、历史 Reservation 绑定的 Offer、当前可履约 Offer、reserved Job、active Reservation、held Claim、Broker receipt、消费者余额预授权和 TTL；
3. 从 Job、Offer、capability 与 accesses 重建 runtime/model/plugin、workload/input digest、数值 ResourceGrant、完整 Start 与计划时间；
4. 写 Plan、连续 ordinal 的 access joins 和一对一 seal；
5. 逐表 exact readback，重新解析 JSON、复算所有摘要，再统一 commit。

历史 exact replay 只证明这份不可变计划已存在，不代表其 Provider、预算或期限仍当前，也不能直接交给 dispatcher。

## 6. v211 exact 门

v212 为 `compute_attempt_dispatch_commands` 增加 `BEFORE INSERT` 门卫。新 command 必须命中已 seal 的 exact Plan，并逐列匹配 Provider/Offer/Job/Reservation/Claim、Attempt lease/number/shard/fencing、executor、route binding、plan digest、Lease/hard deadline 和 command 时间窗口。Store 写路径还会主动回读 Plan 与 seal；SQLite trigger 是第二道门。

迁移发现既有 v211 command 时固定返回 `COMPUTE_EXECUTION_PLAN_BACKFILL_REQUIRED`。旧 command 的三个 plan 字段不能恢复 runner、plugin、ArtifactAccess 或数值 grant，禁止生成占位计划伪造历史。

## 7. 仍禁止的路线

v213 已在源码中继续铺设 Adapter/credential/route authorization、outbox/claim/send-attempt、remote observation、LeaseAuthorityBinding、actor receipt 和 no-start proof，但这些仍是无生产构造器、无 worker、无网络消费者的本地权威核。本批仍不实现 concrete Adapter/factory、endpoint credential verifier、KMS、网络发送、ACK ingress、公网 route、Node wire、外部矿池接入或 remote prepare/commit/reconcile/cancel。

真实 Start 启用前还必须：

- 只消费 v213 sealed route authorization，并先 durable command/outbox，后做具有幂等和自动过期语义的远端 provisional prepare；
- 本地 ACK + v185 + application commit 后才交付 exact Lease authority；
- 对未知结果先 reconcile，并以 authenticated never-committed/canceled-before-run proof 解阻 v176；
- 旧 Provider-owner 人工 Renew 与 no-execution Abort 已固定失败；后续继续保持 Provider 授权主体和后台 service actor 分离；
- 接入 fenced Runner events、Cancel/no-start 与崩溃恢复。首版可以限制短任务、不支持 Renew，但不能省略 Cancel/no-start。

## 8. 实现入口

- `server/src/compute_federation/execution_plan.rs`
- `server/src/compute_federation/execution_plan/`
- `server/src/compute_attempt_activation_migration/execution_plan.rs`
- `server/src/store/compute_attempt_execution_plans.rs`
- `server/src/store/compute_attempt_execution_plans/`

下游派发边界见 `attempt-execution-gateway-v1.md`，耐久投递与恢复证据见 `attempt-delivery-outbox-v1.md`；Provider/Offer/Job/Reservation 总览见本目录 `README.md`。
