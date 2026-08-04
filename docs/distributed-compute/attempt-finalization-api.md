---
title: 分布式算力 Attempt 可信终态与容量收口
status: current
reviewed_at: 2026-08-04
owners: ai-economy, backend
---

# 分布式算力 Attempt 可信终态与容量收口

## 1. 当前实现

v194、追加式 Store、独立 Service 与 HTTP 路由已经写入代码，但尚未编译、执行迁移或运行接口验证，状态固定为 `implementation_uncompiled`。平台 `admin/owner` 只能基于由 accepted Verification 签发的精确 v193 Execution Receipt 应用一次可信终态。

这是 Attempt 链中第一项会同时修改业务状态和容量账本的操作。它本身不会扣除消费者预授权、释放 Provider 收益或生成 Settlement Receipt，不能描述为“任务已经完成付款”；后续 v195 待结算回执是独立事务，见 `docs/distributed-compute/attempt-settlement-api.md`。

## 2. HTTP 路由

| 方法 | 路径 | 调用者 | 作用 |
|---|---|---|---|
| POST | `/api/admin/compute/attempt-leases/:lease_id/trusted-finalization` | 平台 `admin/owner` | 原子应用可信终态和容量收口 |
| GET | `/api/me/compute/attempt-leases/:lease_id/trusted-finalization` | Job 消费者或 Provider 所有者 | 读取并重新审计终态回执 |

POST 必须提供精确 Execution Receipt ID 和摘要、当前 Lease/Job/Reservation/Claim 的 revision/digest、`fencing_generation`、稳定幂等键，并显式设置 `confirm_trusted_terminal_and_capacity=true`。

## 3. 原子效果

同一个 SQLite 立即事务会完成：

- 将当前 running Lease 推进为 terminal；
- 将 Job 从 running 推进为 `verification_pending`；
- 将 Reservation 推进为 `consumed`；
- 将 Capacity Claim 从 active 推进为 `consumed`；
- consumable meter 的 `compensable_usage` 从 active 转入 consumed，未使用部分归还 available；
- reusable meter 不形成永久消耗，全部从 active 归还 available，但其 compensable usage 仍保留在终态回执中供后续结算；
- 保存不可更新、不可删除的 v194 可信终态回执。

容量、Lease、Job、Reservation、Claim 或回执任一写入失败，整个事务回滚，不留下半完成状态。

## 4. 时间与并发边界

可信终态使用 Execution Receipt 的 `finished_at` 作为业务状态和容量账本的生效时间，并另存平台实际 `finalized_at`。该生效时间必须位于激活时间、Lease 硬截止和 Reservation 交付窗口允许的范围内。

平台提交后如果 Lease 被续租，revision、digest 或 fencing generation 将不再匹配，旧请求失败关闭。Job、Reservation 或 Claim 的当前版本发生变化时同样拒绝写入，调用者必须重新读取当前事实，不能用旧候选覆盖新状态。

## 5. 不可变审计

首次写入前会重新审计 v193 Execution Receipt、Provider 终态候选、Attempt 激活、精确历史版本和当前状态。每次读取还会核对：

- 源版本与终态版本是否只推进一次；
- Claim 账本的 consumed/returned 数量是否与 meter mode 和 compensable usage 一致；
- Job、Reservation、Claim 与 Lease 的当前投影是否仍指向回执记录的终态版本；
- 请求摘要、事件摘要、时间和效果字段是否可重建；
- `money_effect` 是否仍为 `preauthorization_unchanged`，`settlement_effect` 是否仍为 `pending`。

同一 Lease 只允许一份可信终态回执；相同幂等键不能绑定不同请求。任何历史、投影或账本腿不一致都会失败关闭。

## 6. 尚未实现

- Cargo 编译、v194 迁移执行、HTTP 真实调用、并发与故障注入验证；
- NodeAgent 到云端的可信事件传输、签名验证和自动触发；
- Lease 超时、重试、迟到结果、挑战及争议的完整状态机；
- v195 已另行形成首版 CNY Settlement Receipt、消费者预授权扣结/退款和 Provider pending 收益；复杂费用、争议、释放、提现和纠正回执仍未实现；
- 自动结算、可提现余额、多币种、外部矿池和链上资产。

## 7. 代码入口

- `server/src/store/compute_attempt_finalizations.rs`
- `server/src/store/compute_attempt_finalizations/`
- `server/src/compute_attempt_finalization_migration.rs`
- `server/src/compute_federation_attempt_finalization_service.rs`
- `server/src/compute_federation_attempt_finalization_api.rs`
