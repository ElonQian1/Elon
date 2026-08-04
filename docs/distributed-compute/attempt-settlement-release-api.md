---
title: 分布式算力 Attempt 待结算原子释放
status: current
reviewed_at: 2026-08-05
owners: ai-economy, backend
---

# 分布式算力 Attempt 待结算原子释放

## 1. 当前实现

v198、追加式 Store、独立 Service 与 HTTP 路由已经写入代码，但尚未编译、执行迁移或运行接口验证，状态固定为 `implementation_uncompiled`。

v198 在 v195 Settlement Receipt 创建满 72 小时且挑战门卫允许时，把该笔 Provider 与平台收益从 `pending` 原子转入 `available`。它保存独立 Release Receipt、Posting 和四条不可变账本腿，不改写 v195 Settlement Receipt、v196 Challenge、v197 Resolution 或 v199 Correction Receipt。

## 2. HTTP 路由

| 方法 | 路径 | 调用者 | 作用 |
|---|---|---|---|
| POST | `/api/admin/compute/attempt-leases/:lease_id/settlement-release` | 平台 `admin/owner` | 显式执行一次 pending 到 available 的释放 |
| GET | `/api/admin/compute/attempt-leases/:lease_id/settlement-release` | 平台 `admin/owner` | 管理侧读取并重新审计释放回执 |
| GET | `/api/me/compute/attempt-leases/:lease_id/settlement-release` | 消费者或 Provider 所有者 | 参与方读取并重新审计释放回执 |

写请求必须精确绑定 v195 Settlement Receipt ID、Settlement Event Digest、Posting ID 与 Posting Digest，提供稳定幂等键，并显式确认资金只在内部账本中从 `pending` 转入 `available`。

## 3. 释放条件

一次释放必须同时满足：

1. v195 Settlement Receipt 及其上游 v193/v194、价格快照、预授权和 Posting 可完整重审计；
2. 服务端当前时间不早于 `settled_at + 72 小时`；
3. 同一 Settlement Receipt、Posting 和 Lease 尚无 Release Receipt；
4. 挑战状态为 `none`、`rejected`、`withdrawn` 或带有有效 v199 回执的 `accepted_corrected`；
5. Provider 和平台 pending 余额分别足以覆盖该笔不可变结算金额；
6. 请求中的全部预期 ID、摘要、操作人和幂等作用域一致。

`open` 与尚未纠正的 `accepted` 挑战均失败关闭。`accepted_corrected` 只释放 v199 纠正后的 Provider/平台净额。释放成功后，消费者不能再为该 Settlement Receipt 创建新挑战，避免挑战截止点附近出现“先释放、后挑战”的竞态。

## 4. 原子账本

单一 `BEGIN IMMEDIATE` 事务完成：

- Provider pending 借记；
- Provider available 贷记；
- 平台 pending 借记；
- 平台 available 贷记；
- Release Posting 与四条账本腿写入；
- Release Receipt、请求摘要、挑战门卫快照和事件摘要写入。

任一步失败时全部回滚。账户更新使用 revision、旧 pending 和旧 available 的比较条件，防止并发覆盖。相同幂等请求返回经重新审计的原回执；相同幂等键用于不同请求时拒绝。

## 5. 审计与状态语义

每次读取都会重新核对：

- 请求 JSON、回执 JSON、数据库列和事件摘要；
- v195 Settlement Receipt、原 Posting、双价格腿金额以及可选 v199 纠正净额；
- 72 小时挑战截止时间和 v196/v197/v199 当前挑战门卫；
- Release Posting、四条账本腿、历史余额快照与账户 revision；
- Provider/平台当前 pending 与 available 是否可由不可变账本重建。

`available` 仅表示平台内部账本已度过本轮挑战释放门卫。v200 可由 Provider 所有者把本人 available 转入 withdrawn 提款保留区，但该申请仍不等于现金到账、银行付款、钱包转账或链上资产。

## 6. 尚未实现

- Cargo 编译、v198 迁移执行、HTTP 真实调用、并发和故障注入验证；
- 定时扫描并自动释放到期 Settlement Receipt；
- accepted 挑战的纠正、冲正、退款或替换 Settlement Receipt；
- 提款取消、拒绝、外部已付款证明、提现风控、外部支付或银行清算；
- Sui、代币、多币种、矿池或其他链上结算。

因此，本实现不能被描述为真实付款或可提现收益已经上线。

## 7. 代码入口

- `server/src/store/compute_attempt_settlement_releases.rs`
- `server/src/store/compute_attempt_settlement_releases/`
- `server/src/compute_settlement_release_migration.rs`
- `server/src/compute_federation_attempt_settlement_release_service.rs`
- `server/src/compute_federation_attempt_settlement_release_api.rs`

上游待结算、挑战和决议分别见 `docs/distributed-compute/attempt-settlement-api.md`、`docs/distributed-compute/attempt-settlement-challenge-api.md` 与 `docs/distributed-compute/attempt-settlement-challenge-resolution-api.md`。下游提款申请见 `docs/distributed-compute/settlement-withdrawal-request-api.md`。
