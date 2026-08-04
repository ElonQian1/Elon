---
title: 分布式算力 Attempt 待结算回执
status: current
reviewed_at: 2026-08-04
owners: ai-economy, backend
---

# 分布式算力 Attempt 待结算回执

## 1. 当前实现

v195、追加式 Store、独立 Service 与 HTTP 路由已经写入代码，但尚未编译、执行迁移或运行接口验证，状态固定为 `implementation_uncompiled`。平台 `admin/owner` 只能基于精确的 v194 可信终态和 v193 Execution Receipt，为一个 Attempt 原子生成一次 Settlement Receipt。

这是 Attempt 链中第一项实际结清消费者平台人民币预授权、并为 Provider 与平台登记待结算收益的操作。Provider 收益只进入独立的 `pending` 余额，不是可提现余额，也不代表真实银行、支付机构或链上资金已经转移。

## 2. HTTP 路由

| 方法 | 路径 | 调用者 | 作用 |
|---|---|---|---|
| POST | `/api/admin/compute/attempt-leases/:lease_id/settlement-receipt` | 平台 `admin/owner` | 原子计算并登记首份待结算回执 |
| GET | `/api/me/compute/attempt-leases/:lease_id/settlement-receipt` | Attempt 消费者或 Provider 所有者 | 读取并重新审计已有回执 |

POST 必须提供精确的 v194 Finalization ID/摘要、v193 Execution Receipt ID/摘要、当前 `verification_pending` Job revision/digest、Broker 预算预授权 ID、Price Snapshot ID/摘要、稳定幂等键，并显式设置 `confirm_consumer_capture_and_provider_pending=true`。

## 3. 价格与用量规则

v195 只处理以下最窄合同：

- 币种必须为 `CNY`；
- Price Snapshot 的 `fee_rules` 必须为空；
- 消费者价格腿按 Execution Receipt 的 `verified_usage` 计算；
- Provider 价格腿按 `compensable_usage` 计算；
- meter 集合必须与价格组件完全一致，用量不得超过组件上限，且必须符合 `unit_size` 粒度；
- 消费者总价按 Price Snapshot 的 `floor`、`ceil`、`half_up` 或 `half_even` 规则舍入到人民币分；
- Provider 应得金额保留人民币微单位，平台价差等于舍入后的消费者扣结额减去 Provider 应得金额；
- 消费者扣结不得超过既有预授权或快照消费者上限，Provider 应得不得超过 Provider 上限，也不得形成负平台价差。

附加费用、奖励、罚款、第三方成本和多币种尚未接入，不能通过 v195 伪造为零以外的经济效果。

## 4. 原子资金与状态效果

单一 `BEGIN IMMEDIATE` 事务会：

1. 重新审计 v193、v194、Job、Reservation、Price Snapshot、Offer、Provider 和 v175 Broker 预授权的完整历史绑定；
2. 将消费者预授权按实际结算金额标记为 `settled`，未使用的人民币分退回原消费者平台余额；
3. 把 Provider 应得金额记入 Provider 独立账户的 `pending_micros`；
4. 把平台价差记入平台独立账户的 `pending_micros`；
5. 将 Job 从 `verification_pending` 推进为 `settled`；
6. 保存不可更新、不可删除的 Settlement Receipt、posting 和四条账本腿。

任何计算、余额、状态、posting、账本腿或回执写入失败，整个事务回滚。该事务不调用外部支付、银行、钱包、Sui 或矿池接口。

## 5. 幂等与审计

同一 Lease、Finalization、Execution Receipt 和消费者预算预授权各只允许绑定一份 v195 回执。相同管理员作用域下的幂等键不能用于不同请求。

每次读取都会重新核对：

- 规范请求摘要、数据库列、回执 JSON、Settlement Receipt 摘要和事件摘要；
- v193/v194 不可变上游证据及 Job 历史版本；
- Reservation、Price Snapshot、Offer、Provider 和 Broker 预授权绑定；
- 按历史价格与 verified/compensable usage 重算的双价格腿；
- 消费者预授权扣结与退款结果；
- posting、四条不可变账本腿和 Provider/平台当前 pending 投影。

历史、金额、投影或摘要不一致时读取失败关闭，不返回未经审计的资金结果。消费者挑战入口与期限见 `docs/distributed-compute/attempt-settlement-challenge-api.md`。

## 6. 尚未实现

- Cargo 编译、v195 迁移执行、HTTP 真实调用、并发与故障注入验证；
- `fee_rules`、多币种、外部成本、奖励、罚款和复杂舍入策略；
- 挑战撤回、裁决、纠正、退款、冲正和替换回执；
- pending 收益的挑战期届满释放、可用余额和提现；
- Provider 可提现余额、提现审批、真实支付、税务、对账和清算；
- 外部矿池结算、法币托管、Sui 链上凭证或链上资产；
- 自动结算调度和 NodeAgent 可信事件自动触发。

## 7. 代码入口

- `server/src/store/compute_attempt_settlements.rs`
- `server/src/store/compute_attempt_settlements/`
- `server/src/store/billing_reservations/compute_settlement.rs`
- `server/src/compute_attempt_settlement_migration.rs`
- `server/src/compute_federation_attempt_settlement_service.rs`
- `server/src/compute_federation_attempt_settlement_api.rs`

上游可信终态见 `docs/distributed-compute/attempt-finalization-api.md`；完整市场目标与后续清算边界见 `docs/distributed-compute/market-and-settlement.md`。
