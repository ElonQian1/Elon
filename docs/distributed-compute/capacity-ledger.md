---
title: 分布式算力容量池与账本设计
status: current
reviewed_at: 2026-08-04
owners: backend, ai-economy
---

# 分布式算力容量池与账本设计

## 1. 解决的问题

一个节点可以为多个模型发布 Offer，一个集群也可以同时连接一龙、企业客户和外部矿池。市场不能把每条 Offer 当成独立物理容量；所有竞争同一资源的承诺必须落到同一个共享池和追加式账本。

```mermaid
flowchart LR
    P["CapacityPool / epoch"] --> B1["Window A · attempt_slot"]
    P --> B2["Window A · token 或 GPU-ms"]
    O1["Offer: LLM A"] --> B1
    O1 --> B2
    O2["Offer: LLM B"] --> B1
    O2 --> B2
    B1 --> L["原子多 meter Ledger"]
    B2 --> L
```

## 2. 对象边界

### CapacityPool

表示一组会互相争用的物理资源。Pool 保存 Provider、稳定身份、当前 epoch、版本摘要、资源档案、区域和支持的 meter，不保存实时余额。节点 GPU、受管集群分区或外部 Provider 配额都可以成为 Pool。

`capacity_epoch` 只在旧容量完全排水或退役后递增。旧 epoch 的 Offer、Claim 和迟到事件保留审计价值，但不能修改新 epoch 余额。

### DeliveryWindow 与 CapacityBucket

`DeliveryWindow` 使用稳定 ID、摘要和 UTC 半开区间。`CapacityBucket` 是 `(pool_id, capacity_epoch, delivery_window_id, meter)` 的唯一余额单元，并记录 meter 是 consumable 还是 reusable。

### ComputeOffer

Offer 引用精确 Pool binding；每条容量行再引用精确 bucket。Offer 只声明愿意出售的静态上限。创建、复制或续期 Offer 都不会修改 bucket 的 `issued` 或 `available`。

### CapacityClaim

Claim 把一组 meter 数量绑定到 Quote hold、Reservation、未来 Commitment、DeliveryAllocation 或 Attempt。它拥有稳定 ID、主体、状态、revision、可选 parent claim 和过期时间，是幂等释放与防止“释放别人的容量”的边界。

当前 Store 的 Hold 必须显式设置 `expires_at`，只允许在交付窗口结束前创建，且 TTL 不得越过窗口结束；一个多 meter Claim 的全部 bucket 必须共享完全相同的窗口边界。窗口结束、TTL 生效和 Expire 授权都以 Store 生成的 `recorded_at` 为权威，调用方 `occurred_at/cutoff_at` 不能伪装未来时间或提前到期。Release/Expire 只接受仍为 `held` 的 Claim，并从该 Claim 自己的 ledger legs 以 checked `i128` 证明每条 held 归属数量且 active 为零。`active` 容量只能由未来绑定 fencing 的 Attempt return/consume 路径推进。

### LedgerTransaction 与 LedgerLeg

Transaction 固定 pool、epoch、window、事件类型、幂等键、请求摘要、业务主体和因果引用，并包含一个或多个 meter line。每个 line 展开成等额双腿；Transaction/Leg 均不可更新或删除。

## 3. 账户与事件

| 事件 | 账户移动 | 典型主体 |
|---|---|---|
| `supply_added` | `issuance -> available` | Pool 容量发行 |
| `supply_withdrawn` | `available -> retired` | 排水后的供给撤出 |
| `reservation_held` | `available -> held` | Quote / Reservation |
| `attempt_activated` | `held -> active` | Attempt Lease |
| `attempt_returned` | `active -> available` | 可复用 slot 正常归还 |
| `usage_consumed` | `active -> consumed` | Token、GPU-ms 等消耗量 |
| `reservation_released` | `held -> available` | 取消或显式释放 |
| `reservation_expired` | `held -> available` | 到期恢复器 |

未来 CapacityCommitment 同样进入 `held`，但使用独立 claim kind。将 Commitment 切给具体 DeliveryAllocation 时先转移 Claim 归属；实际 Attempt 激活后才进入 `active`。

## 4. 原子 Reserve

本地供给的最终 Reserve 是市场线性化点：

1. 以 Job 范围幂等键查重并比对 request digest；
2. 重新读取精确 Offer、Provider、授权、交付窗口和价格来源；
3. 按稳定 meter 顺序对共享 bucket 做条件扣减；
4. 追加一笔多 meter ledger transaction，创建或推进 Claim；
5. 同事务冻结消费者预算并锁定 Quote 阶段已经登记的不可变 PriceSnapshot；
6. 创建 Reservation，把 Job 绑定到精确 Offer 与快照；
7. 写幂等结果并提交。

任何一步失败全部回滚。候选查询、Quote 和 ReadyCapability 只提供观察事实，不能跳过 Reserve 的再次检查。

当前容量 Store 已把 Claim Hold/Finish/Activation 拆出不自行提交的事务内 kernel。公开 standalone 方法仍以 `BEGIN IMMEDIATE` 包住 Hold/Finish kernel 并负责 commit，但拒绝 Reservation 主体或绑定；v175/v176 Broker 在自己持有的同一事务中调用这些 kernel。Hold V2 摘要固定完整 causal binding，Reservation Claim 强制绑定 Offer、Job 与同主体 Reservation；Finish 继承原始 held 绑定并精确引用因果前序。v175 第一版 Broker 组合余额预授权、Job 登记和 Reservation 登记；v176 对尚未激活 Attempt 的 active Reservation 原子完成退款、held Claim Release/Expire、Job canceled/failed 和 Reservation released/expired。v185 通过仅供外层事务使用的 Activation kernel 将既有 Claim `held -> active`，并把 Attempt Lease ID 和 fencing generation 写入容量因果链，再与 Job/Reservation 新版本和不可变激活回执一同提交。v186 只更新 Lease 状态投影和追加续租回执，不触碰容量账本。上述状态均为 `implementation_uncompiled`，只覆盖 `platform_balance_cny`；v185/v186 不发送节点命令、不新增扣款，也不覆盖 active 容量归还、实际用量或运行中结算。

## 5. Attempt 与容量

v185 的首次 Attempt 已接受登记在独立事务中把 Reservation Claim 从 `held` 推到 `active`，同时检查：

- 当前时间位于所选交付窗口；
- hard deadline 不晚于窗口结束；
- Provider/Offer 仍为 active 或仅处于安全 draining，原 Broker 预算仍有效；
- Pool 的 `attempt_slot` 等可复用 meter 仍与 Claim 一致；
- 当前仅接受首次 `attempt_no=1` 与 `fencing_generation=1`；后续单调递增路径尚未实现。

当前 v185 不写派发 outbox，也不发送网络命令；调用者必须显式声明外部执行器已接受，并提供接受证明引用。未来真实派发应只写 outbox，网络发送发生在提交后；失败重发同一 command ID/digest，不能因一次 WebSocket 超时创建新 Attempt。

## 6. 余额守恒

每个 bucket 持有可原子更新的余额投影：

```text
consumable: issued = available + held + active + consumed + retired
reusable:   issued = available + held + active + retired
```

更新前后都检查非负和守恒，算术使用 checked `i128`。`balance_revision` 每次成功事务递增；调用方可以携带预期 revision 做 CAS，但数据库事务仍是最终权威。

## 7. 到期、恢复与对账

- Quote hold、Reservation 和 Commitment 都有明确 expires_at；
- 恢复器按 Claim 而非汇总余额追加 release/expired transaction；standalone 入口拒绝保留的 `compute_reservation` 主体，通用容量恢复器按 held 账本中真实的 Reservation causal binding 跳过 Broker 管理的 Claim，通用余额到期器也排除 v175 Broker 回执引用的预授权，避免任一资源腿在统一 Broker 终态之外单独归还；
- 相同 effect 使用唯一幂等键，不会重复归还；
- Supply Add/Withdraw 与 Claim Hold/Finish 的 request digest 已由 Store 按版本化字段、排序 bucket 数量和规范 UTC 计算；代码级重放仍返回当前 Claim/余额，尚未保存不可变首次响应，因此不能描述为严格幂等闭环；
- 账本与余额投影不一致时停止新 Reserve，重放 ledger 重建投影并形成审计报告；
- 旧 epoch、旧 fencing generation 和晚到终态只能追加审计，不能影响当前余额。

## 8. 外部矿池

远程 reserve 使用 pending saga。数据库先保存本地 provisional hold 与 adapter outbox，Worker 用固定远程幂等键调用；成功后激活，失败或超时追加释放并发出补偿 cancel。任何网络请求都不在 SQLite 写事务内执行。

## 9. 当前实现边界

2026-08-04 本文与 ADR 已接受；领域合同、checked-i128 reducer、v165-v186 SQLite schema，以及隔离的本地 Store 已经形成。Store 当前覆盖池版本与 bucket 登记、多 meter 供给发行/撤出、窗口与 TTL 有界的 Claim hold、Claim-local held-only 释放/到期、Store-canonical request digest、双分录落库、余额 CAS、Claim 历史、只读账本重算、有界到期批处理、状态门卫、追加式生命周期、排空后的 epoch 轮换、Provider/Offer/Price Snapshot/Job/Reservation 注册和历史审计。v175 Broker 统一原子 Reserve；v176 统一未执行任务退款与终态；v185 统一首个已接受 Attempt 的 held/active 容量、reserved/running Job、active Reservation 和不可变 Lease 回执，并保持预算预授权不变；v186 增加 Lease 状态投影和外部心跳声明续租，但不改变容量或资金。上述路径状态为 `implementation_uncompiled`，尚未执行迁移、调度、并发验证或真实容量操作；节点派发、接受/心跳证明验证、Lease 超时归还、运行中实际用量结算、受控自动修复和完整运行协议仍未接线。
