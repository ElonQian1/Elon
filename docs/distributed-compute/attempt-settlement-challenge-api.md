---
title: 分布式算力 Attempt 结算挑战
status: current
reviewed_at: 2026-08-04
owners: ai-economy, backend
---

# 分布式算力 Attempt 结算挑战

## 1. 当前实现

v196、追加式 Store、独立 Service 与 HTTP 路由已经写入代码，但尚未编译、执行迁移或运行接口验证，状态固定为 `implementation_uncompiled`。

Job 消费者可在 v195 Settlement Receipt 创建后的固定 72 小时内，为同一结算提交一份不可覆盖的挑战。挑战只记录争议事实并阻断后续 pending 收益释放，不撤销原结算、不退款、不移动任何余额，也不等于平台已认可消费者主张。

## 2. HTTP 路由

| 方法 | 路径 | 调用者 | 作用 |
|---|---|---|---|
| POST | `/api/me/compute/attempt-leases/:lease_id/settlement-challenge` | Job 消费者 | 在挑战期内提交首份结算挑战 |
| GET | `/api/me/compute/attempt-leases/:lease_id/settlement-challenge` | 消费者或 Provider 所有者 | 读取并重新审计挑战 |
| GET | `/api/admin/compute/attempt-leases/:lease_id/settlement-challenge` | 平台 `admin/owner` | 管理侧读取并重新审计挑战 |

POST 必须精确绑定 v195 Settlement Receipt、posting、Lease、Job 与消费者身份，并提供稳定幂等键、一个原因代码、8 至 1000 字摘要和最多 16 个证据引用。

## 3. 挑战规则

- 固定策略为 `consumer_challenge_72h_v1`；
- 期限从 v195 Settlement Receipt 的服务端记录时间起计算 72 小时；
- 只有原 Job 消费者可提交；Provider 与管理员只能读取；
- 原因代码限于 `amount`、`metering`、`price_snapshot`、`execution_evidence`、`provider_identity` 或 `other`；
- 证据引用会规范化、排序、去重并限制数量；
- 同一 Settlement Receipt、posting 和 Lease 各只允许一份挑战；
- 同一消费者作用域内的幂等键不能用于不同请求。

超过期限、上游绑定漂移、重复但不一致、未知原因代码或证据超限都会失败关闭。

## 4. 资金与状态边界

提交挑战不会修改：

- 消费者已经结清的预授权和已退余额；
- Provider 或平台的 `pending_micros`；
- Job、Reservation、Lease、Claim 或 Execution Receipt；
- 外部银行、支付机构、钱包、Sui 或矿池状态。

当前的唯一经济效果是：v196 查询能证明该 Settlement Receipt 存在未解决挑战，后续 pending 释放实现必须先调用该门卫并失败关闭。

## 5. 审计

每次读取都会重新核对挑战数据库列、规范请求摘要、挑战 JSON、事件摘要、v195 Settlement Receipt 和 posting 摘要，以及 Lease、Job、消费者、Provider 的历史绑定。任一记录缺失、摘要漂移或身份不一致时不返回未经审计的挑战。

## 6. 尚未实现

- Cargo 编译、v196 迁移执行、HTTP 真实调用、并发和故障注入验证；
- 消费者撤回挑战；
- 平台接受或驳回挑战；
- 纠正、退款、冲正与替换 Settlement Receipt；
- 挑战期届满后的 pending 自动释放；
- Provider 可用余额、提现与真实外部资金清算。

因此，v196 不能被描述为完整争议系统或资金冻结系统。

## 7. 代码入口

- `server/src/store/compute_attempt_settlement_challenges.rs`
- `server/src/store/compute_attempt_settlement_challenges/`
- `server/src/compute_settlement_challenge_migration.rs`
- `server/src/compute_federation_attempt_settlement_challenge_service.rs`
- `server/src/compute_federation_attempt_settlement_challenge_api.rs`

上游金额合同见 `docs/distributed-compute/attempt-settlement-api.md`；完整市场目标见 `docs/distributed-compute/market-and-settlement.md`。
