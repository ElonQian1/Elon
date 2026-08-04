---
title: 分布式算力 Attempt accepted 挑战结算纠正
status: current
reviewed_at: 2026-08-05
owners: ai-economy, backend
---

# 分布式算力 Attempt accepted 挑战结算纠正

## 1. 当前实现

v199、追加式 Store、独立 Service 与 HTTP 路由已经写入代码，但尚未编译、执行迁移或运行接口验证，状态固定为 `implementation_uncompiled`。

v199 只处理 v197 已裁决为 `accepted` 的消费者挑战。平台管理员给出向下修正后的消费者费用、Provider 应得和平台价差，系统在单一事务中向消费者余额退款，并从 Provider 与平台 pending 余额冲减对应金额。原 v195 Settlement Receipt、v196 Challenge、v197 Resolution 和 `billing_reservation` 均保持历史原貌。

## 2. HTTP 路由

| 方法 | 路径 | 调用者 | 作用 |
|---|---|---|---|
| POST | `/api/admin/compute/attempt-leases/:lease_id/settlement-correction` | 平台 `admin/owner` | 对 accepted 挑战执行一次向下金额纠正 |
| GET | `/api/admin/compute/attempt-leases/:lease_id/settlement-correction` | 平台 `admin/owner` | 管理侧读取并重新审计纠正回执 |
| GET | `/api/me/compute/attempt-leases/:lease_id/settlement-correction` | 消费者或 Provider 所有者 | 参与方读取并重新审计纠正回执 |

写请求必须精确绑定 Challenge ID/事件摘要、Resolution ID/事件摘要和 Settlement Receipt ID/事件摘要，提供 8 至 1000 字说明、最多 16 条有界证据引用、稳定幂等键，并显式确认消费者退款和 pending 冲减效果。

## 3. 金额合同

管理员提交的是纠正后的金额，不是任意增减额。服务端强制：

1. 纠正后的消费者费用必须小于原已扣费用，且不得为负数；
2. 纠正后的 Provider 应得和平台价差分别不得高于原值；
3. 纠正后消费者费用微单位必须等于纠正后 Provider 应得加平台价差；
4. 消费者退款必须等于 Provider 冲减加平台冲减；
5. 消费者金额仍以人民币分保存，账本金额以整数微单位保存，禁止浮点数；
6. 一个 Challenge、Resolution、Settlement Receipt 和 Lease 只能生成一份纠正回执。

本版本只允许向下纠正，不能借挑战增加消费者收费或 Provider 收益。

## 4. 状态与原子账本

正常状态链为：

```text
open challenge
  -> accepted resolution
  -> accepted_corrected
  -> 72小时窗口结束后按纠正净额释放
```

`accepted` 会阻断 v198，因此正常路径中 accepted 挑战与已经释放的 available 收益互斥。纠正事务完成：

- 消费者 `user_balance` 增加纠正退款；
- Provider pending 借记冲减；
- 平台 pending 借记冲减；
- Correction Posting 与三条不可变账本腿写入；
- Correction Receipt、证据摘要、请求摘要和事件摘要写入。

任一步失败时全部回滚。Provider/平台余额使用 revision 和旧 pending 比较条件，消费者余额使用旧余额比较条件，防止并发覆盖。

## 5. 与 v198 的关系

纠正完成后，挑战门卫投影为 `accepted_corrected`，不再由 accepted 状态阻断。v198 不释放原结算金额，而是释放：

```text
Provider 可释放金额 = 原 Provider 应得 - Provider 纠正冲减
平台可释放金额 = 原平台价差 - 平台纠正冲减
```

纠正可以在 72 小时窗口内完成，但 v198 仍必须等到原 Settlement Receipt 的 72 小时窗口结束。

## 6. 审计

每次读取都会重新核对：

- 请求 JSON、回执 JSON、数据库列、证据摘要和事件摘要；
- v195 Settlement Receipt、v196 Challenge 与 accepted v197 Resolution；
- 原金额、纠正后金额、消费者退款和双侧冲减守恒；
- Correction Posting、三条账本腿和历史余额快照；
- Provider/平台当前 pending 是否可由原结算、全部纠正和全部释放账本重建。

消费者额外退款由独立纠正账本证明，不篡改 v195 已发生的预授权结清事实。

## 7. 尚未实现

- Cargo 编译、v199 迁移执行、HTTP 真实调用、并发和故障注入验证；
- 非金额类 accepted 挑战的补救任务或履约重做；
- 已进入 available 后发现新问题时的追索、负余额或保证金制度；
- 自动裁决、自动纠正或自动释放；
- available 提现、外部支付、银行、钱包、Sui 或多币种清算。

因此，本实现不能被描述为完整争议仲裁、真实退款到账或链上纠正已经上线。

## 8. 代码入口

- `server/src/store/compute_attempt_settlement_corrections.rs`
- `server/src/store/compute_attempt_settlement_corrections/`
- `server/src/compute_settlement_correction_migration.rs`
- `server/src/compute_federation_attempt_settlement_correction_service.rs`
- `server/src/compute_federation_attempt_settlement_correction_api.rs`

上游决议见 `docs/distributed-compute/attempt-settlement-challenge-resolution-api.md`；后续净额释放见 `docs/distributed-compute/attempt-settlement-release-api.md`。
