---
title: 分布式算力共享容量池与追加式账本 V1
status: accepted
date: 2026-08-04
owners: backend, ai-economy
implementation_status: lifecycle_and_audit_store_uncompiled
---

# 分布式算力共享容量池与追加式账本 V1

## 背景

同一用户节点、GPU 集群或外部矿池可以同时发布多个模型和 SKU Offer。如果每个 Offer 独立复制节点并发或未来窗口容量，市场会把同一份物理资源重复出售。不可变 Offer 也不能保存随 Reservation 变化的实时 `committed` 或剩余数量。

## 决定

### 1. Offer 必须绑定共享容量池

`CapacityPool` 是可互相争用的物理资源边界。稳定 `pool_id` 之上使用单调 `capacity_epoch` 隔离退役后重新发行的容量，并用不可变 pool revision/digest 描述当期资源包络。共享同一物理资源的所有 Offer 必须引用同一 pool 与 epoch；发布 Offer 不增加任何余额。

### 2. V1 使用单池、单窗口 Reservation

一份 Reservation V1 只绑定一个 CapacityPool、一个精确 `DeliveryWindow` 和多个 meter。窗口采用规范 UTC 的半开区间 `[starts_at, ends_at)`；同一 pool/epoch 的可售 bucket 不重叠。需要跨池或跨窗口的任务由 Planner 拆成多个 Reservation，再由上层 Job 组合。

### 3. Offer 容量行绑定真实 bucket

每个 `ComputeOfferCapacity` 必须引用确定的 `capacity_bucket_id` 和 `delivery_window_id`。Offer 的 `total_units`、`reservable_units` 是该版本最多愿意出售的静态上限；实时可用量只能从共享账本投影得出。

### 4. 使用多 meter 双腿追加式账本

一笔 `CapacityLedgerTransaction` 可以原子包含多个 meter line。每行用正整数数量从一个账户搬到另一个账户，并生成等额正负两条 ledger leg；同一 transaction 的每个 pool/window/meter 净变化必须为零。所有加总使用 checked `i128` 中间值，持久化仍使用有界 `i64`。

账户最小集合为：

- `issuance`：容量发行来源，只用于 `supply_added`；
- `available`：当前可被新承诺占用；
- `held`：Quote、Reservation 或未来 Commitment 已锁定；
- `active`：已经分配给正在交付的 Attempt；
- `consumed`：不可重复使用的计量单位已经消耗；
- `retired`：明确撤出市场且不能重新出售。

### 5. Claim 防止重复释放

每次 Quote hold、Reservation、CapacityCommitment 或 DeliveryAllocation 都创建稳定 `CapacityClaim`，并保存按 meter 的数量。Claim 以 revision 和状态推进，释放、到期或消费都必须引用原 Claim；不能只按汇总余额加回，从而误释放另一笔请求的容量。

V1 Claim 不做部分拆分。未来 Commitment 分配给多个 Job 时创建带 parent claim 的子 Claim，并保证所有子项之和不超过父项未分配数量。

### 6. 余额投影可变，账本不可变

`CapacityBucketBalance` 是为原子条件更新准备的物化投影，保存 balance revision 和各账户非负余额。Ledger transaction 与 leg 只追加，禁止更新或删除。每次业务事务必须同时写 ledger、推进 Claim 和更新余额；任一步失败全部回滚。投影可以从账本重建，不能反过来覆盖历史账本。

### 7. 幂等键绑定请求摘要

目标语义是写操作使用 `(idempotency_scope, idempotency_key)` 唯一键并保存 Store 规范化的 `request_digest`：同键同摘要返回不可变首次结果，同键不同摘要失败关闭。当前隔离 Store 已不再接收 Supply Add/Withdraw 与 Claim Hold/Finish 的调用方摘要。Supply 使用 V1 typed payload；Hold V2 绑定完整 pool/window、主体、排序 bucket 数量、规范 UTC 时间、Claim 类型与完整 causal binding；Finish 则绑定 claim ID、expected revision、终态 action 和规范 UTC 发生时间，资源与主体经不可替换的 claim ID 间接固定。幂等范围和键作为查重信封，不进入内容摘要。

Reservation Claim 在 Hold V2 中必须固定 Offer ID/version/digest、Job ID 和同主体 Reservation ID，Attempt/fencing 仍禁止提前绑定；因果标识拒绝首尾空白，摘要字节与逐字段持久化值一致，同键换绑失败关闭。Hold 回执返回持久化后的规范 `recorded_at` 与 `expires_at`，供外层 Reservation 精确复用。Finish 的 Reservation 专用入口再次核对预期绑定，再从原始 held 账本事务继承，终态业务时间不得早于 held 记录时间。Hold V2 仍只是容量 Claim 的业务绑定摘要，不等于包含预算授权、Price Snapshot 与完整 Reservation 合同的最终 Broker request digest。普通重放仍返回当前 Claim/余额，Reservation-bound Hold 只重放仍未到期的初始 held 版本；尚未保存通用不可变首次结果，因此严格保证仍未闭环。上述字节规则只由 Store 计算，不是跨语言 JCS 合同；外部矿池如需自行验签摘要，必须使用共享 SDK 或另立显式版本。容量、预算、Reservation 和 Job 绑定的最终 Reserve 必须处于同一个本地数据库线性化事务。

### 8. 事务内禁止外部网络

SQLite 路径使用 `BEGIN IMMEDIATE`、稳定 meter 顺序和条件更新。公开 standalone API 继续拥有连接、事务与 commit，但拒绝 `compute_reservation` 主体或任何 Reservation causal binding；供 Store sibling 调用的 Claim kernel 只接受调用方已有的 `Transaction`，内部不得重新取连接、开启嵌套事务或提交。最外层 Broker 将来负责把 Job、预算、Reservation 与该 kernel 组合成唯一线性化点。节点 WebSocket、价格远程源或外部矿池 reserve 不得在数据库事务内调用。外部 Provider 使用 pending Reservation + outbox + 补偿动作；重放始终沿用同一远程幂等身份。通用容量到期恢复器依据 held 账本中持久化的 Reservation causal binding 跳过 Broker 管理的 Claim，不信任可伪造的主体字符串；这类 Claim 必须由未来 Broker 在同一事务内同步推进预算、Reservation 与 Job。

## 核心不变量

- 任意 bucket 账户余额都不得为负；
- 可消耗 meter：`issued = available + held + active + consumed + retired`；
- 可复用 meter：`issued = available + held + active + retired`，Attempt 返回时 `active -> available`；
- 同一物理资源的全部 Offer 必须落到同一 pool/epoch/bucket；
- Reservation 不能超过 Offer 静态上限、共享 bucket 实时余额或 Offer `execution_limits`；
- 提前 hold 可以发生在窗口开始前，但 Attempt 只能在窗口内启动，hard deadline 不得晚于窗口结束；
- Hold 必须有不晚于窗口结束的到期时间，窗口和到期授权以 Store 当前记录时间为准；`release/expire` 只能 `held -> available`，`active` 只能由 Attempt 路径归还或消费；
- 到期和取消通过追加 `released/expired` transaction 归还原 Claim，不删除旧记录。

## 后果

- 多模型、多 SKU、多销售渠道可以共享同一物理供给而不重复出售；
- 现货 Reservation 与未来 Commitment 使用同一容量真源；
- 账本对象增加，但审计、恢复、对账和未来清算拥有稳定基础；
- Broker 必须把候选观察与最终 Reserve 分开，候选阶段的余额从不构成承诺。

## 验证状态

本决定已接受。领域合同、纯状态投影和 v165 SQLite schema 已写入；本地 Store 还形成了池版本与 bucket 登记、供给发行/撤出、窗口有界 Claim hold、Claim-local held-only 释放/到期、四类写入的 Store-canonical request digest，以及共用的双分录落库和余额 CAS。只读审计可从账本重算余额投影，有界批处理可逐 Claim 恢复到期容量；状态门卫、v167 追加式生命周期、v168 epoch 轮换、v169 Provider Registry、Offer 规范合同校验、v170 Offer Registry、v171 不可变 Price Snapshot Registry、v173 Claim 历史与 v174 Reservation Registry 也已形成代码。Claim Hold/Finish、Job 登记与 Reservation 登记已提供不自行提交的事务内入口；Hold V2 固定 causal binding，Reservation Claim 强制绑定 Offer/Job/Reservation，终态继承并审计原始 held 绑定。上述新增路径均未编译、执行迁移、调度或并发验证。不可变 Claim 首次响应、消费者预算与 Reservation/Price Snapshot 的统一 Reserve、完整事务内 Broker API、Attempt 激活、自动修复、外部 Provider saga 和运行接线仍未实现。
