---
title: 分布式算力 Attempt Execution Gateway V1
status: current
reviewed_at: 2026-08-10
owners: backend, node, ai-economy
implementation_status: implementation_uncompiled
---

# 分布式算力 Attempt Execution Gateway V1

## 1. 当前状态

v211 的 Provider-neutral Start command、Adapter binding/ACK 与追加式账本，v212 的 sealed Execution Plan，及 v213 的 route/credential authority、耐久 outbox、send-attempt、认证 observation、LeaseAuthorityBinding、actor receipt 与 no-start proof 均已写入源码。v214 又形成 rejected cleanup、quarantine pair、cancel→reconcile 解锁和 final reconcile proof 的本地 Store 闭包。上述批次均未编译、执行迁移或运行真实链路；accepted happy application/actor/Lease authority/commit outbox issuer 仍固定返回 `COMPUTE_ATTEMPT_ACCEPTED_ACK_V213_ISSUER_UNAVAILABLE`。

旧的 `prepare_start` Adapter trait 会把远端调用放在 durable command 之前，已从源码移除。v213-v214 registry 与 Store 仍只是生产不可达的本地权威核：没有 sealed route/credential/plan/observation 构造器，也没有 resolver、transport 或 worker 消费 send authority。因此真实 Start、远端 ACK/reconcile 和 accepted 应用都不可达，不会推进 Job、Reservation、Capacity Claim、Attempt Lease 或余额预授权。

## 2. v211 保存的证据

| 账本 | 事实 | 明确不代表 |
|---|---|---|
| `compute_attempt_dispatch_commands` | 不可变 Start command、Provider/Offer/Job/Reservation/Claim 精确版本、Adapter 配置、已 seal Execution Plan、Lease/fencing、Broker 与余额预授权绑定 | 命令已发送、节点已安装插件或远端已准备 |
| `compute_attempt_dispatch_acks` | Adapter 身份校验后的 `accepted` 或 `rejected` 响应，以及平台的 `accepted_applied/rejected/quarantined` disposition | `accepted` 已经开始执行、产生用量或完成任务 |
| `compute_attempt_dispatch_applications` | accepted ACK 与唯一 v185 activation 的 exact application 回执 | 节点已收到 commit、Runner 已启动或 Provider 已获得收益 |

三张表均追加式，拒绝 update/delete/replace。command 和 ACK 的 canonical JSON、domain-separated SHA-256、关系列及 source binding 在 Store 中精确回读；DDL 还投影关键 JSON 字段，并用 deferred activation/application 反向边阻断三者拆开提交。Adapter route 显式区分 `provider_endpoint` 与 `server_adapter`：前者精确绑定当前 Provider endpoint，后者精确绑定当前 Provider adapter；`external_pool` 只能走后者。v169 的 Adapter config digest 仍是 opaque exact value，v211 不把它冒充 SHA-256。

## 3. 本地原子边界

v214 当前形成的 cleanup/recovery Store 闭包都在一个 `BEGIN IMMEDIATE` 中完成：

1. authenticated final `prepare_response(rejected)` observation、`rejected` ACK 与 exact `prepare_rejected` no-start proof 同事务入账，不调用 v185；
2. accepted 因 command/source/budget/route currentness 失败而 quarantine 时，先创建 `cancel` pending 与 `reconcile` blocked pair，再提交 `quarantined` ACK；
3. authenticated cancel observation 只把 exact reconcile 从 `blocked` 解锁为 `pending`，不创建 no-start proof；
4. authenticated final reconcile 的 `never_committed` observation 与 durable no-commit tombstone 同事务生成 exact `remote_never_committed` proof。

Store 派生的 proof `recorded_at` 与 outbox 状态转换 `updated_at` 由本地时钟生成；authenticated observation 的 `recorded_at` 则属于不可改摘要的 sealed envelope。任一步失败均整体回滚；cleanup 闭包不推进 Job、Reservation、Claim、Lease 或资金，也不把 cancel ACK 冒充远端未执行。

accepted happy issuer 仍在写入 ACK、v185、application、actor receipt、LeaseAuthorityBinding 或 commit outbox 前固定失败。v211/v213 的 deferred foreign key 与反向 trigger 仍定义目标闭包：未来必须在同一事务逐字段绑定 exact command、`accepted_applied` ACK、v185 activation、application、actor/binding 和 commit outbox，任何一项都不得单独提交。v211 Start V1 仍只允许首次 `attempt_no=1/fencing_generation=1`；相同 `(provider_id, adapter_id, adapter_ack_id)` 重投只能复用首次服务端盖章的 ACK，认证事实变化则冲突。

v176 继续阻断仍有未解决 Start 的 Reservation 退款与容量释放。v214 可在上述 rejected 或 final reconcile 闭包中生成 exact proof，但只有同一 command/reservation/Job/Claim/budget/Lease/fence 的 proof 才能解阻；command/claim 过期、ACK 缺失、quarantined 或 cancel ACK 仍不足以证明远端未执行。没有可信 observation 构造器与网络时，这些远端证明路径仍不能由生产事实触发。

## 4. accepted 只是 provisional

本地 SQLite 事务不能覆盖远端节点、集群或矿池。当前 `accepted` 只定义为 Adapter 提供的 provisional remote acceptance reference；它不是 Runner 已开始执行的声明。

任何生产 Adapter 启用前，accepted happy path 必须实现以下两阶段协议：

1. 远端 prepare 只保留可幂等恢复、自动到期且尚不可执行的 provisional reservation；
2. 平台原子提交 ACK + v185 + application；
3. 本地同时保存 exact LeaseAuthorityBinding、actor receipt 与 commit outbox；
4. commit 后耐久投递执行授权；
5. 远端只有收到精确 lease authority 后才可运行。

缺少这条协议时，远端可能已运行而本地事务回滚，所以当前 accepted issuer 保持固定不可用；v214 cleanup/recovery 不会旁路该门。

## 5. 旧入口与兼容边界

原 `/api/me/compute/providers/:provider_id/attempt-activations` POST 依赖调用者填写 `confirm_executor_accepted=true`，不能证明 Adapter 身份或远端接受。v211 源码已把该写入口改为稳定失败 `COMPUTE_ATTEMPT_EXECUTION_GATEWAY_NOT_READY`；候选和历史读取仍可保留，但不能再用人工声明绕过 Gateway。

节点 Host 的 `Start / RenewLease / Cancel` 仍是本机插件执行合同，不是 Provider-neutral wire。Provider-owner Renew/Abort 写入口现已分别固定失败 `COMPUTE_ATTEMPT_RENEW_GATEWAY_NOT_READY` 与 `COMPUTE_ATTEMPT_ABORT_GATEWAY_NOT_READY`；GET、列表和历史回执可保留。v212 只允许从认证 capability、ArtifactAccess、Job/Offer/Reservation/Claim 与预算共同投影完整计划；不能把 Job digest、最低资源需求或旧 Host 类型改名后冒充可执行授权。

## 6. 尚未实现

- ReadyCapability V2/endpoint/Adapter capability 的真实构建、认证上报与服务端验证，以及 Artifact credential issuer；
- user-node、managed-cluster、external-pool 三类生产 Adapter、credential verifier 与 resolver；
- outbox worker、ACK ingress、公网认证、provisional prepare/commit/reconcile/cancel 网络协议与 crash injection；
- accepted application、actor receipt、LeaseAuthorityBinding 与 commit outbox 的可用 issuer；
- send authority 的真实消费者；历史 replay receipt、claim 或 send-attempt 都不能单独当发送授权；
- authenticated remote observation 的真实构造器与 Runner 侧 durable no-commit tombstone producer；
- Node Agent capability、协议、精确 session 派发及本地 durable redelivery；
- RenewLease、Cancel 与 Runner event 的同类 v212+ Gateway 账本；
- 编译、迁移、SQLite trigger/崩溃注入、并发、接口和真实远端验证。

在上述能力完成前，不得开放公网 ACK route，不得从临时 WebSocket `session_id` 派生 command/Lease 身份，也不得回退到旧 Host 或人工确认入口。

## 7. 实现入口

- `server/src/compute_federation/attempt_gateway.rs`
- `server/src/compute_federation/route_authority.rs`
- `server/src/compute_federation/start_outbox.rs`
- `server/src/compute_federation/execution_plan.rs`
- `server/src/compute_attempt_activation_migration/attempt_dispatch.rs`
- `server/src/compute_attempt_activation_migration/execution_plan.rs`
- `server/src/store/compute_attempt_dispatches.rs`
- `server/src/store/compute_attempt_execution_plans.rs`
- `server/src/store/compute_attempt_start_outbox.rs`
- `server/src/store/compute_attempt_dispatches/`
- `server/src/store/compute_attempt_activations.rs`
- `server/src/compute_federation_attempt_service.rs`

派发恢复权威见 `attempt-delivery-outbox-v1.md`，上游计划权威见 `attempt-execution-plan-v1.md`，预留边界见 `broker-api.md`，底层 v185 状态变化见 `attempt-activation-api.md`，节点本机执行合同见 `node-client-and-plugins.md`。
