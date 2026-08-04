---
title: 分布式算力 Capacity Supply 本人控制面
status: current
reviewed_at: 2026-08-05
owners: backend, node, ai-economy
implementation_status: implementation_uncompiled
---

# 分布式算力 Capacity Supply 本人控制面

## 1. 当前状态

本人 Capacity Supply 追加与撤回控制面已写入代码，但尚未编译、执行 v165 迁移或运行 HTTP/MCP 验证，状态固定为 `implementation_uncompiled`。它允许用户向本人 Provider 当前 Pool 版本、同一交付窗口的一组 open Bucket 原子追加 self-declared 供给，也允许把尚在 `available` 的供给原子撤入 `retired`。两类操作复用既有双分录账本；它们不激活 Provider/Pool、不发布 Offer，也不代表容量已经被平台验证或消费者可以预留。

PC `/compute-supply` 已写入单 Bucket 追加和撤出表单源码，提交前检查正整数、最小量子、当前 available 上限和显式确认，并为同一弹窗重试保留稳定幂等键。当前页面只操作单 Bucket，后端 1 至 64 个同窗口 Bucket 的原子批量能力仍保留给未来批量界面和 MCP；页面尚未构建、运行或发布。

## 2. HTTP 与 MCP

HTTP 要求一龙用户 Bearer 会话：

| 方法 | 路径 | 作用 |
|---|---|---|
| POST | `/api/me/compute/providers/:provider_id/capacity-pools/:pool_id/supply` | 向同一窗口的 1 至 64 个 Bucket 原子追加供给 |
| POST | `/api/me/compute/providers/:provider_id/capacity-pools/:pool_id/supply/withdraw` | 从同一窗口的 1 至 64 个 Bucket 原子撤回 available 供给 |

追加请求提供稳定 `idempotency_key`、Bucket/数量明细和 `confirm_supply=true`，MCP 工具为 `compute_add_my_capacity_supply`。撤回请求使用同样的幂等键与明细结构，但要求 `confirm_withdrawal=true`，MCP 工具为 `compute_withdraw_my_capacity_supply`。两项工具均标记为有副作用的幂等操作。

服务端分别为追加与撤回生成隔离的本人幂等范围，并固定账本主体和首次发生时间。首次响应丢失后，重试会读取已保存的首次时间；两个并发首次请求同时到达时，失败一方只在发现相同事件后用首次时间安全重放。相同范围内的幂等键绑定不同 Pool、窗口、Bucket 或数量时仍失败关闭。

## 3. 写入前检查

- Provider 与 Pool 必须属于当前用户；
- 追加时 Provider 只允许 `registering` 或 `active`，撤回仍以本人 Pool 所有权和 Store 状态门卫为准；
- 每个 Bucket 必须属于当前 Pool 的精确 epoch/revision；
- Bucket 必须为 open，且全部属于同一交付窗口；
- 数量必须为正整数，并且是各自 meter 最小量子的整数倍；
- 首次追加时交付窗口不能已经结束；原样重放不受后来时间流逝影响；
- 请求最多包含 64 个不重复 Bucket，并按 Bucket ID 规范排序。

Store 根据版本化字段和首次时间生成规范请求摘要，在一个 `BEGIN IMMEDIATE` 事务中追加多 meter LedgerTransaction/Leg、检查 checked-i128 守恒、更新余额 revision，并返回当前余额。撤回只能执行 `available -> retired`；数量超过 available 或试图间接触碰 held/active 时，整笔事务失败。`registering` Pool 允许撤回，供用户在激活前纠正自己的容量声明；retired Pool 仍禁止写入。

## 4. 信任与经济边界

`available` 表示该 Provider 所有者写入的可用容量声明，不等于平台观测或验证的硬件事实。撤回只形成不可删除的 `retired` 审计事实，不是消费者退款、Provider 收益结算或 Sui 链上代币销毁。由于本人入口不提供 Provider/Pool 激活，现有 Broker 的 Hold 门卫仍拒绝在 registering Pool 上预留；因此本接口不会单独产生消费者债权、Provider 收益、人民币余额或 Sui 链上资产。

未来只有节点绑定、证据验证、受控激活、Offer 发布及 Reserve 再检查全部完成后，供给才可能进入真实交易。

## 5. 尚未实现

- Cargo 编译、v165 迁移执行和 HTTP/MCP 真实调用验证；
- Provider/Pool 证据验证、审批和激活；
- Offer/Price Snapshot 发布、自动撮合和消费者预留；
- Attempt、实际用量、Provider 收益和链上结算。
