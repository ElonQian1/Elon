---
title: 分布式算力 Attempt 累计声明用量控制面
status: current
reviewed_at: 2026-08-04
owners: backend, node, ai-economy
implementation_status: implementation_uncompiled
---

# 分布式算力 Attempt 累计声明用量控制面

## 1. 当前状态

v188、Store、Service 与 HTTP 路由已经写入代码，但尚未编译、执行迁移或运行接口验证，状态固定为 `implementation_uncompiled`。本控制面只把 Provider 或未来节点 Host 上报的累计 meter 保存为 `provider_declared` 证据，不把它提升为平台观测、验证用量或结算依据。

本批不接 NodeAgent 线协议。`executor_usage_ref` 只是外部 Host 事件的引用；平台不读取其正文、不验证签名，也不据此改变 Lease、Job、Reservation、Capacity Claim、消费者余额或 Provider 收益。

## 2. HTTP 接口

| 方法 | 路径 | 权限 | 作用 |
|---|---|---|---|
| POST | `/api/me/compute/providers/:provider_id/attempt-leases/:lease_id/declared-usage` | Provider 所有者 | 追加一份累计声明用量快照 |
| GET | `/api/me/compute/attempt-leases/:lease_id/declared-usage/latest` | Provider 所有者或 Job 消费者 | 读取并审计最新快照 |
| GET | `/api/me/compute/attempt-leases/:lease_id/declared-usage/by-sequence/:sequence_no` | Provider 所有者或 Job 消费者 | 按序号读取并审计历史快照 |

写请求必须提供当前 Lease 的精确 revision/digest/fencing、JSON 安全正整数序号、外部用量引用、每个 meter 的累计数量、幂等键，并显式设置 `confirm_provider_declaration_only=true`。

## 3. 失败关闭条件

一次写入必须同时满足：

- 调用者仍是当前 Provider 所有者，Provider 状态为 `active` 或 `draining`；
- Lease 当前状态为 `running`，已经存在心跳，且仍未越过软期限或硬期限；
- 请求精确匹配当前 Lease revision、digest 与 fencing generation；
- v185 激活回执绑定的当前 Job 仍为 `running`、Reservation 仍为 `active`、Capacity Claim 仍为 `active`；
- Job、Reservation、Claim、Provider、消费者和 Lease 因果链没有漂移；
- 快照包含 Capacity Claim 合同的全部 meter，不能缺失、增加或重复；
- 每个累计数量为非负整数，当前序号严格大于上一快照，所有 meter 的累计数量不得回退。

相同 Provider 幂等键只能重放相同规范请求；同一 Lease 的同一序号也不能绑定不同声明。

## 4. 超额声明

累计值可以高于原 Reservation 的对应预留数量。平台不会丢弃这类证据，也不会自动把超额值计费，而是在回执的 `overage_meters` 中标记超额 meter。后续验证策略必须结合价格快照、Job 上限、平台观测和合同规则决定哪些值可以进入 `verified_usage` 与 `compensable_usage`。

## 5. 不可变回执

每份 v188 回执固定：

- Lease ID、source revision/digest、fencing generation；
- 当前 Job、Reservation 与 Capacity Claim 的精确版本和摘要；
- Provider、消费者、用量序号和外部事件引用；
- 排序后的累计 `provider_declared` meter、逐项 reading digest 与总 usage digest；
- 原预留 meter 合同、合同摘要和超额 meter；
- 规范请求摘要、事件摘要、操作者与服务端时间。

表和回执为追加式，禁止更新或删除。读取和幂等重放会重新计算请求、reading、usage、合同及事件摘要；重放不会产生第二份快照。

## 6. 效果边界

响应固定声明：

- `verification_status: "unverified_provider_declaration"`；
- `execution_effect: "evidence_only"`；
- `capacity_effect: "unchanged"`；
- `reservation_effect: "unchanged"`；
- `money_effect: "preauthorization_unchanged"`。

因此，v188 不表示任务成功、用量可信、容量已经消耗、消费者已经扣款或 Provider 已获得收入。

## 7. 尚未实现

- Cargo 编译、v188 迁移执行、HTTP 真实调用、并发和故障注入验证；
- NodeAgent Host 到云端的签名事件传输、outbox、断点续传和真实身份校验；
- 自动接入控制面、网关、重新分词、计时器、确定性复算和挑战任务；v191 已允许管理员保存第一份待验证平台观测，见 `docs/distributed-compute/attempt-platform-observation-api.md`；
- `verified_usage`、`compensable_usage`、Execution Receipt 和争议流程；v189 只保存绑定本快照的 Provider 终态候选，见 `docs/distributed-compute/attempt-terminal-candidate-api.md`；
- running Attempt 可信终态、Capacity Claim `UsageConsumed`、消费者扣款、Provider 收益和最终结算；
- 多次 Attempt、自动重试、外部矿池、多币种和 Sui 链上资产。

## 8. 实现入口

- `server/src/store/compute_attempt_usage.rs`
- `server/src/store/compute_attempt_usage/`
- `server/src/compute_attempt_usage_migration.rs`
- `server/src/compute_federation_attempt_service.rs`
- `server/src/compute_federation_attempt_api.rs`
