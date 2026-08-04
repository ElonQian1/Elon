---
title: 分布式算力 Attempt 消费者终态审核证据
status: current
reviewed_at: 2026-08-04
owners: ai-economy, backend
---

# 分布式算力 Attempt 消费者终态审核证据

## 1. 当前实现

v190、追加式 Store、Service 与 HTTP 路由已经写入代码，但尚未编译、执行迁移或运行接口验证，状态固定为 `implementation_uncompiled`。本控制面允许 Job 消费者对 v189 Provider 终态候选登记第一份 `accepted`、`rejected` 或 `disputed` 审核证据。

消费者审核只是一方证明，不是平台 Verification 决定。即使消费者选择 `accepted`，也不会生成 Execution Receipt、扣除预授权、释放 Provider 收益、消费容量或推进 Lease、Job、Reservation、Claim 状态。

## 2. HTTP 路由

| 方法 | 路径 | 调用者 | 作用 |
|---|---|---|---|
| POST | `/api/me/compute/attempt-leases/:lease_id/terminal-candidate/consumer-review` | Job 消费者 | 登记第一份消费者审核证据 |
| GET | `/api/me/compute/attempt-leases/:lease_id/terminal-candidate/consumer-review` | Job 消费者或 Provider 所有者 | 读取并重新审计审核回执 |

POST 请求必须提供精确终态候选 ID、候选事件摘要、决定、规范原因码、消费者侧审核引用、可选证据引用、稳定幂等键，并显式设置 `confirm_consumer_attestation_only=true`。

## 3. 失败关闭边界

首次写入在同一 `BEGIN IMMEDIATE` 事务中执行。Store 会重新读取并审计 v189 候选，然后检查：

- 登录用户就是候选绑定的 `consumer_account_id`；
- `lease_id`、候选 ID 与候选事件摘要完全一致；
- 候选内部 Lease、Job、Reservation、Claim、最终 v188 快照及全部摘要仍能通过审计；
- 同一候选和 Lease 尚无其他消费者审核；
- 相同消费者幂等键没有绑定不同规范请求。

候选是不可变事实，因此消费者可在候选登记后稍晚审核；v190 不以当前 Lease 是否仍在软租期内决定消费者是否有权保存证据。该设计不授权消费者改写候选，也不让迟到审核覆盖未来平台决定。

## 4. 决定与证据

| decision | 含义 | 额外约束 |
|---|---|---|
| `accepted` | 消费者声明当前结果符合其预期 | 证据引用可以为空 |
| `rejected` | 消费者声明结果不符合预期 | 至少提供一个证据引用 |
| `disputed` | 消费者要求进入后续争议流程 | 至少提供一个证据引用 |

原因码只允许小写字母、数字、点、下划线和连字符。证据引用最多 16 个，服务端排序、去重后保存；表内只保存引用，不保存凭据正文、密钥或大体积证据内容。

## 5. 不可变回执

每份回执固定消费者审核 ID、v189 候选 ID/事件摘要、Lease/Job/Reservation/Claim 精确因果链、最终 v188 用量快照、Provider 候选 outcome、消费者决定、原因码、审核引用、证据引用摘要、请求摘要、事件摘要、操作者与服务端时间。

数据库以唯一候选、唯一 Lease 和消费者幂等范围阻止第二份不同审核，并以触发器禁止更新和删除。读取时会重新审计 v189 候选、冻结字段、规范请求和事件摘要。

回执效果固定为：

- `evidence_status: "consumer_attestation_only"`；
- `review_effect: "consumer_evidence_recorded"`；
- `verification_effect: "none"`；
- Lease、Job、Capacity、Reservation 均为 `unchanged`；
- `money_effect: "preauthorization_unchanged"`。

## 6. 尚未实现

- Cargo 编译、v190 迁移执行、HTTP 真实调用、并发与故障注入验证；
- 消费者审核 MCP 入口、结果工件真实取回与内容验证；
- 平台观测自动接线、独立验证器和多策略治理；v191 已提供管理员平台观测，v192 已提供管理员保守 Verification 决定，见 `docs/distributed-compute/attempt-platform-observation-api.md` 和 `docs/distributed-compute/attempt-verification-api.md`；
- rejected/disputed 的举证、仲裁、超时和追加式裁决流程；
- Execution Receipt、运行中终态推进、容量消费、扣款与 Provider 收益；
- 自动重试、替代交付和结算纠正回执。

## 7. 代码入口

- `server/src/store/compute_attempt_consumer_reviews.rs`
- `server/src/store/compute_attempt_consumer_reviews/`
- `server/src/compute_attempt_consumer_review_migration.rs`
- `server/src/compute_federation_attempt_service.rs`
- `server/src/compute_federation_attempt_api.rs`
