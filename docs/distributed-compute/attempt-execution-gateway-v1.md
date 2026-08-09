---
title: 分布式算力 Attempt Execution Gateway V1
status: current
reviewed_at: 2026-08-09
owners: backend, node, ai-economy
implementation_status: implementation_uncompiled
---

# 分布式算力 Attempt Execution Gateway V1

## 1. 当前状态

v211 的 Provider-neutral Start command、Adapter binding/ACK、追加式 SQLite 账本和 Store seam 已写入源码；v212 又补上不可变 Execution Plan producer、capability/ArtifactAccess receipt、数值 ResourceGrant 和 plan seal。两批均没有编译、执行迁移或运行真实链路。当前没有可信输入构造器、可用 Adapter、外部入口、节点协议、后台投递器或恢复 worker；Start 与 accepted ACK 均不可达。

源码只有一个返回 `COMPUTE_PROVIDER_ADAPTER_UNAVAILABLE` 的 unavailable 实现；尚无 registry、resolver 或调用者，sealed capability 整体不可达。该失败发生在 command 入账前，不推进 Job、Reservation、Capacity Claim、Attempt Lease 或余额预授权。

## 2. v211 保存的证据

| 账本 | 事实 | 明确不代表 |
|---|---|---|
| `compute_attempt_dispatch_commands` | 不可变 Start command、Provider/Offer/Job/Reservation/Claim 精确版本、Adapter 配置、已 seal Execution Plan、Lease/fencing、Broker 与余额预授权绑定 | 命令已发送、节点已安装插件或远端已准备 |
| `compute_attempt_dispatch_acks` | Adapter 身份校验后的 `accepted` 或 `rejected` 响应，以及平台的 `accepted_applied/rejected/quarantined` disposition | `accepted` 已经开始执行、产生用量或完成任务 |
| `compute_attempt_dispatch_applications` | accepted ACK 与唯一 v185 activation 的 exact application 回执 | 节点已收到 commit、Runner 已启动或 Provider 已获得收益 |

三张表均追加式，拒绝 update/delete/replace。command 和 ACK 的 canonical JSON、domain-separated SHA-256、关系列及 source binding 在 Store 中精确回读；DDL 还投影关键 JSON 字段，并用 deferred activation/application 反向边阻断三者拆开提交。Adapter route 显式区分 `provider_endpoint` 与 `server_adapter`：前者精确绑定当前 Provider endpoint，后者精确绑定当前 Provider adapter；`external_pool` 只能走后者。v169 的 Adapter config digest 仍是 opaque exact value，v211 不把它冒充 SHA-256。

## 3. 本地原子边界

accepted 路径只允许在一个 `BEGIN IMMEDIATE` 中完成：

1. 重读 command、Adapter、当前 Provider/Offer/Job/Reservation/Claim、Broker 与余额预授权；
2. 先插入 `accepted_applied` ACK；其 `activation_lease_id` 与 deterministic `application_id` 都是 deferred foreign key；
3. 调用唯一 `activate_compute_attempt_on` kernel，原子推进 held Claim、reserved Job、active Reservation 并创建 staging Lease；
4. 插入 application 回执并逐字复算；
5. 最后统一 commit。

v185 表的反向 trigger 要求新 activation 已看到 exact command 与 `accepted_applied` ACK，application trigger 又逐字段绑定 ACK、activation 与 command。ACK 单独提交会因两个 deferred foreign key 失败；activation 单独提交会被反向 trigger 拒绝。因此本地数据库不存在 ACK、v185、application 任一缺失仍成功提交的路径。

`rejected` 只追加 ACK；过期 command、路由/版本漂移、既有 activation 或预算失效的 accepted ACK 只追加 `quarantined`。两者都不调用 v185，不改变 Job、Reservation、Claim、Lease 或资金。v211 Start V1 仅允许首次 `attempt_no=1/fencing_generation=1`，全部 command/ACK 入账时间采用固定 UTC 纳秒格式，并在 ACK 截止和 Lease expiry 间保留固定安全余量。相同 `(provider_id, adapter_id, adapter_ack_id)` 的远端重投只返回首次服务端盖章的 `ack_id/received_at`；远端认证事实变化则冲突。

v176 现会阻断仍有未 ACK command 或 accepted/quarantined ACK 的 Reservation 退款与容量释放；只有明确 `rejected` 可继续旧未执行终态。quarantined 表示远端可能已接受，必须等待后续 durable cancel/no-start proof，不能据“本地未激活”推断远端未执行。

## 4. accepted 只是 provisional

本地 SQLite 事务不能覆盖远端节点、集群或矿池。当前 `accepted` 只定义为 Adapter 提供的 provisional remote acceptance reference；它不是 Runner 已开始执行的声明。

任何生产 Adapter 启用前必须另有两阶段协议：

1. 远端 prepare 只保留可幂等恢复、自动到期且尚不可执行的 provisional reservation；
2. 平台原子提交 ACK + v185 + application；
3. commit 后耐久投递执行授权；
4. 远端只有收到精确 lease authority 后才可运行。

缺少这条协议时，远端可能已运行而本地事务回滚，所以当前 accepted capability 保持不可构造。

## 5. 旧入口与兼容边界

原 `/api/me/compute/providers/:provider_id/attempt-activations` POST 依赖调用者填写 `confirm_executor_accepted=true`，不能证明 Adapter 身份或远端接受。v211 源码已把该写入口改为稳定失败 `COMPUTE_ATTEMPT_EXECUTION_GATEWAY_NOT_READY`；候选和历史读取仍可保留，但不能再用人工声明绕过 Gateway。

节点 Host 的 `Start / RenewLease / Cancel` 仍是本机插件执行合同，不是 Provider-neutral wire。现有 Provider-owner Renew/Abort 也仍是人工确认旧路径；真实 Adapter 启用前必须失败关闭，直到各自具备 durable command、认证 ACK、fencing 与恢复账本。v212 只允许从认证 capability、ArtifactAccess、Job/Offer/Reservation/Claim 与预算共同投影完整计划；不能把 Job digest、最低资源需求或旧 Host 类型改名后冒充可执行授权。

## 6. 尚未实现

- ReadyCapability V2/endpoint/Adapter capability 的真实构建、认证上报与服务端验证，以及 Artifact credential issuer；
- user-node、managed-cluster、external-pool 三类生产 Adapter 与 Adapter credential；
- command outbox 投递、ACK ingress 身份认证、provisional prepare/commit、崩溃恢复和迟到事件调和；
- 发送 claim/sweep：每次派发前重审 deadline、Provider route、Offer/Job/Reservation/Claim、Broker、预算与 fencing，历史 replay receipt 不能当发送授权；
- quarantined Start 的 durable cancel/no-start 证明与安全 v176 解阻；
- Node Agent capability、协议、精确 session 派发及本地 durable redelivery；
- RenewLease、Cancel 与 Runner event 的同类 v212+ Gateway 账本；
- 编译、迁移、SQLite trigger/崩溃注入、并发、接口和真实远端验证。

在上述能力完成前，不得开放公网 ACK route，不得从临时 WebSocket `session_id` 派生 command/Lease 身份，也不得回退到旧 Host 或人工确认入口。

## 7. 实现入口

- `server/src/compute_federation/attempt_gateway.rs`
- `server/src/compute_federation/execution_plan.rs`
- `server/src/compute_attempt_activation_migration/attempt_dispatch.rs`
- `server/src/compute_attempt_activation_migration/execution_plan.rs`
- `server/src/store/compute_attempt_dispatches.rs`
- `server/src/store/compute_attempt_execution_plans.rs`
- `server/src/store/compute_attempt_dispatches/`
- `server/src/store/compute_attempt_activations.rs`
- `server/src/compute_federation_attempt_service.rs`

上游计划权威见 `attempt-execution-plan-v1.md`，预留边界见 `broker-api.md`，底层 v185 状态变化见 `attempt-activation-api.md`，节点本机执行合同见 `node-client-and-plugins.md`。
