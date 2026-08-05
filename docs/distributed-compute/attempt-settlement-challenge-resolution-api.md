---
title: 分布式算力 Attempt 结算挑战决议
status: current
reviewed_at: 2026-08-05
owners: ai-economy, backend, pc-frontend
---

# 分布式算力 Attempt 结算挑战决议

## 1. 当前实现

v197、追加式 Store、独立 Service、HTTP 路由、消费者/管理员 open 挑战队列、角色历史队列、消费者撤回入口和 PC 角色工作区已经写入代码，但尚未编译、执行迁移、接口联调、页面验收或发布，状态固定为 `implementation_uncompiled`。

v197 为 v196 挑战增加一份不可覆盖的终态决议：原消费者可以撤回，平台 `admin/owner` 可以接受或驳回。一个挑战只能产生一种终态，撤回与管理员裁决并发时只有一个事务能成功。

## 2. HTTP 路由

| 方法 | 路径 | 调用者 | 作用 |
|---|---|---|---|
| POST | `/api/me/compute/attempt-leases/:lease_id/settlement-challenge/withdrawal` | 原 Job 消费者 | 撤回尚未决议的挑战 |
| GET | `/api/me/compute/settlement-challenges/open` | 原 Job 消费者 | 列出本人尚无任何 v197 终态的 open 挑战 |
| GET | `/api/me/compute/attempt-leases/:lease_id/settlement-challenge/resolution` | 消费者或 Provider 所有者 | 读取并重新审计决议 |
| POST | `/api/admin/compute/attempt-leases/:lease_id/settlement-challenge/resolution` | 平台 `admin/owner` | 接受或驳回挑战 |
| GET | `/api/admin/compute/settlement-challenges/open` | 平台 `admin/owner` | 列出全局尚无任何 v197 终态的 open 挑战 |
| GET | `/api/admin/compute/attempt-leases/:lease_id/settlement-challenge/resolution` | 平台 `admin/owner` | 管理侧读取并重新审计决议 |
| GET | `/api/me/compute/settlement-challenges/history` | 原 Job 消费者 | 按时间列出本人申诉及后续决议、纠正和释放证据链 |
| GET | `/api/admin/compute/settlement-challenges/history` | 平台 `admin/owner` | 按时间列出全局申诉及后续证据链 |

所有写入必须精确绑定 v196 Challenge ID/事件摘要，提供 8 至 1000 字说明、稳定幂等键，并显式确认不会退款、纠正或移动余额。

两类 open 队列先按角色筛选无决议挑战，再逐条重放 v196 Challenge 与 v195 Settlement Receipt 审计；消费者队列额外核对 `consumer_account_id`。队列只是待办入口，撤回或裁决 POST 仍在事务内重新检查唯一终态。

历史队列不是数据库 JSON 直出。它逐项重放 v195 Settlement、v196 Challenge、可选 v197 Resolution、可选 v199 Correction 和可选 v198 Release，并拒绝引用错位及不允许的状态组合。PC `/compute-challenges` 展示本人历史，`/compute-challenge-resolution` 展示全局历史；历史中的 available 仅表示平台内部余额，不证明银行、钱包、Sui 或其他外部付款。

## 3. 状态和释放门卫

| 挑战状态 | 后续 pending 释放门卫 | 当前资金效果 |
|---|---|---|
| 无挑战 | 不由挑战阻断 | 无 |
| `open` | 阻断 | 无 |
| `accepted` | 继续阻断，等待 v199 纠正回执 | 无 |
| `rejected` | 不再由挑战阻断 | 无 |
| `withdrawn` | 不再由挑战阻断 | 无 |

解除挑战阻断不等于立即释放。v198 释放入口还会检查 72 小时窗口、原结算审计、余额投影、幂等和其他风险门卫，边界见 `docs/distributed-compute/attempt-settlement-release-api.md`。

## 4. 权限与原子性

- 消费者只能提交 `withdrawn`，且操作人必须是挑战绑定的原 Job 消费者；
- 管理员只能提交 `accepted` 或 `rejected`；
- Provider 只能读取，不能撤回或裁决；
- 同一 Challenge、Settlement Receipt 和 Lease 各只允许一份决议；
- 决议表禁止更新和删除；
- 相同作用域幂等键不能用于不同请求；
- 首次成功后，另一种终态会失败关闭，不能覆盖历史。

## 5. 审计

每次读取都会重新核对决议数据库列、请求 JSON、回执 JSON、事件摘要、actor/action 组合，以及 v196 Challenge 和 v195 Settlement Receipt 的完整绑定。open 队列还会失败关闭任何已出现终态或消费者身份不匹配的候选。挑战或结算证据漂移时，不返回未经审计的决议或候选。

## 6. 尚未实现

- Cargo 编译、v197 迁移执行、HTTP 真实调用、并发和故障注入验证；
- PC 构建、接口联调、桌面与移动视口验收及发布；
- 证据自动核验、自动裁决与非金额补救；
- accepted 挑战的自动纠正、真实外部退款或替换 Settlement Receipt；
- available 余额的提现、外部转账与清算；
- 外部支付、银行、钱包、Sui 或矿池清算。

因此，`accepted` 只表示挑战成立并继续阻断释放，不表示退款已经发生；只有独立 v199 Correction Receipt 才证明消费者退款和 pending 冲减已执行。`rejected/withdrawn` 只解除挑战门卫，是否已经释放必须以独立 v198 Release Receipt 为准。

## 7. 代码入口

- `server/src/store/compute_attempt_settlement_challenge_resolutions.rs`
- `server/src/store/compute_attempt_settlement_challenge_resolutions/`
- `server/src/compute_settlement_challenge_resolution_migration.rs`
- `server/src/compute_federation_attempt_settlement_challenge_resolution_service.rs`
- `server/src/compute_federation_attempt_settlement_challenge_resolution_api.rs`
- `pc-frontend/src/features/compute-attempt/settlementChallengeResolutionContracts.ts`
- `pc-frontend/src/features/compute-attempt/settlementChallengeHistoryContracts.ts`
- `pc-frontend/src/features/compute-settlement/ComputeSettlementChallengePage.tsx`
- `pc-frontend/src/features/compute-settlement/ComputeSettlementChallengeResolutionPage.tsx`
- `pc-frontend/src/features/compute-settlement/SettlementChallengeHistoryList.tsx`

上游挑战见 `docs/distributed-compute/attempt-settlement-challenge-api.md`；完整市场目标见 `docs/distributed-compute/market-and-settlement.md`。

accepted 挑战纠正边界见 `docs/distributed-compute/attempt-settlement-correction-api.md`。
