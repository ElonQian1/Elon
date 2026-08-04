---
title: 分布式算力 Capacity Supply 本人控制面
status: current
reviewed_at: 2026-08-04
owners: backend, node, ai-economy
implementation_status: implementation_uncompiled
---

# 分布式算力 Capacity Supply 本人控制面

## 1. 当前状态

本人 Capacity Supply 追加控制面已写入代码，但尚未编译、执行 v165 迁移或运行 HTTP/MCP 验证，状态固定为 `implementation_uncompiled`。它允许用户向本人 Provider 当前 Pool 版本、同一交付窗口的一组 open Bucket 原子追加 self-declared 供给，通过既有账本完成 `issuance -> available` 双分录；它不激活 Provider/Pool、不发布 Offer，也不代表容量已经被平台验证或消费者可以预留。

## 2. HTTP 与 MCP

HTTP 要求一龙用户 Bearer 会话：

| 方法 | 路径 | 作用 |
|---|---|---|
| POST | `/api/me/compute/providers/:provider_id/capacity-pools/:pool_id/supply` | 向同一窗口的 1 至 64 个 Bucket 原子追加供给 |

请求提供稳定 `idempotency_key`、Bucket/数量明细和 `confirm_supply=true`。MCP 工具为 `compute_add_my_capacity_supply`，同样要求显式确认，并标记为有副作用的幂等操作。

服务端固定账本主体和首次发生时间。首次响应丢失后，重试会读取已保存的首次时间；两个并发首次请求同时到达时，失败一方只在发现相同幂等事件后用首次时间安全重放。相同幂等键绑定不同 Pool、窗口、Bucket 或数量时仍失败关闭。

## 3. 写入前检查

- Provider 与 Pool 必须属于当前用户；
- Provider 只允许 `registering` 或 `active`，Pool 状态继续由 Store 门卫检查；
- 每个 Bucket 必须属于当前 Pool 的精确 epoch/revision；
- Bucket 必须为 open，且全部属于同一交付窗口；
- 数量必须为正整数，并且是各自 meter 最小量子的整数倍；
- 首次追加时交付窗口不能已经结束；原样重放不受后来时间流逝影响；
- 请求最多包含 64 个不重复 Bucket，并按 Bucket ID 规范排序。

Store 根据版本化字段和首次时间生成规范请求摘要，在一个 `BEGIN IMMEDIATE` 事务中追加多 meter LedgerTransaction/Leg、检查 checked-i128 守恒、更新余额 revision，并返回当前余额。

## 4. 信任与经济边界

`available` 表示该 Provider 所有者写入的可用容量声明，不等于平台观测或验证的硬件事实。由于本人入口不提供 Provider/Pool 激活，现有 Broker 的 Hold 门卫仍拒绝在 registering Pool 上预留；因此本接口不会单独产生消费者债权、Provider 收益、人民币余额或 Sui 链上资产。

未来只有节点绑定、证据验证、受控激活、Offer 发布及 Reserve 再检查全部完成后，供给才可能进入真实交易。

## 5. 尚未实现

- Cargo 编译、v165 迁移执行和 HTTP/MCP 真实调用验证；
- 本人 Supply Withdraw 与供给修正控制面；
- Provider/Pool 证据验证、审批和激活；
- Offer/Price Snapshot 发布、自动撮合和消费者预留；
- Attempt、实际用量、Provider 收益和链上结算。
