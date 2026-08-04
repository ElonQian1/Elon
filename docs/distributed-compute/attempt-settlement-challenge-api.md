---
title: 分布式算力 Attempt 结算挑战
status: current
reviewed_at: 2026-08-05
owners: ai-economy, backend, pc-frontend
---

# 分布式算力 Attempt 结算挑战

## 1. 当前实现

v196、追加式 Store、独立 Service、HTTP 路由、消费者待申诉队列和 PC `/compute-challenges` 工作区已经写入代码，但尚未编译、执行迁移、接口联调、页面验收或发布，状态固定为 `implementation_uncompiled`。

Job 消费者可在 v195 Settlement Receipt 创建后的固定 72 小时内，为同一结算提交一份不可覆盖的挑战。挑战只记录争议事实并阻断后续 pending 收益释放，不撤销原结算、不退款、不移动任何余额，也不等于平台已认可消费者主张。

## 2. HTTP 路由

| 方法 | 路径 | 调用者 | 作用 |
|---|---|---|---|
| POST | `/api/me/compute/attempt-leases/:lease_id/settlement-challenge` | Job 消费者 | 在挑战期内提交首份结算挑战 |
| GET | `/api/me/compute/attempt-leases/:lease_id/settlement-challenge` | 消费者或 Provider 所有者 | 读取并重新审计挑战 |
| GET | `/api/me/compute/settlement-receipts/pending-challenge` | Job 消费者 | 列出本人仍在 72 小时窗口内且未挑战、未释放的 v195 回执 |
| GET | `/api/admin/compute/attempt-leases/:lease_id/settlement-challenge` | 平台 `admin/owner` | 管理侧读取并重新审计挑战 |

POST 必须精确绑定 v195 Settlement Receipt、posting、Lease、Job 与消费者身份，并提供稳定幂等键、一个原因代码、8 至 1000 字摘要和最多 16 个证据引用。

候选 GET 先从不可变 ledger 的 `consumer_capture` 腿按当前用户筛选，再逐条重放 v195 审计、pending 状态、挑战门卫、释放记录和服务端时间边界。候选只是填写入口，不能替代 POST 的事务内权威复核。

## 3. 挑战规则

- 固定策略为 `consumer_challenge_72h_v1`；
- 期限从 v195 Settlement Receipt 的服务端记录时间起计算 72 小时；
- 只有原 Job 消费者可提交；Provider 与管理员只能读取；
- 原因代码限于 `amount`、`metering`、`price_snapshot`、`execution_evidence`、`provider_identity` 或 `other`；
- 证据引用会规范化、排序、去重并限制数量；
- 同一 Settlement Receipt、posting 和 Lease 各只允许一份挑战；
- 同一消费者作用域内的幂等键不能用于不同请求。
- PC 工作区只展示服务端返回的本人候选，不在浏览器自行推导身份、金额或挑战截止时间。

超过期限、上游绑定漂移、重复但不一致、未知原因代码或证据超限都会失败关闭。

## 4. 资金与状态边界

提交挑战不会修改：

- 消费者已经结清的预授权和已退余额；
- Provider 或平台的 `pending_micros`；
- Job、Reservation、Lease、Claim 或 Execution Receipt；
- 外部银行、支付机构、钱包、Sui 或矿池状态。

当前的唯一经济效果是：v196 查询能证明该 Settlement Receipt 存在挑战，v198 pending 释放必须结合 v197 决议门卫判断是否失败关闭。决议合同见 `docs/distributed-compute/attempt-settlement-challenge-resolution-api.md`。

## 5. 审计

每次读取挑战都会重新核对挑战数据库列、规范请求摘要、挑战 JSON、事件摘要、v195 Settlement Receipt 和 posting 摘要，以及 Lease、Job、消费者、Provider 的历史绑定。候选读取还会重新检查当前 pending 状态、消费者 ledger 腿、既有挑战、v198 释放和 72 小时窗口。任一记录缺失、摘要漂移或身份不一致时不返回未经审计的挑战或候选。

## 6. 尚未实现

- Cargo 编译、v196 迁移执行、HTTP 真实调用、并发和故障注入验证；
- PC 构建、接口联调、桌面与移动视口验收及发布；
- 挑战证据自动核验、自动裁决、非金额补救和 available 后追索；
- 无人值守到期释放、真实外部退款、付款与链上清算。

因此，v196 不能被描述为完整争议系统或资金冻结系统。

## 7. 代码入口

- `server/src/store/compute_attempt_settlement_challenges.rs`
- `server/src/store/compute_attempt_settlement_challenges/`
- `server/src/compute_settlement_challenge_migration.rs`
- `server/src/compute_federation_attempt_settlement_challenge_service.rs`
- `server/src/compute_federation_attempt_settlement_challenge_api.rs`
- `pc-frontend/src/features/compute-attempt/settlementChallengeContracts.ts`
- `pc-frontend/src/features/compute-settlement/ComputeSettlementChallengePage.tsx`
- `pc-frontend/src/features/compute-settlement/OpenSettlementChallengeDialog.tsx`

上游金额合同见 `docs/distributed-compute/attempt-settlement-api.md`；完整市场目标见 `docs/distributed-compute/market-and-settlement.md`。
