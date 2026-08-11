---
title: Delivery Allocation v228 本地验收
status: current
design_status: design_frozen
implementation_status: implementation_partially_verified
last_updated: 2026-08-12
---

# Delivery Allocation v228 本地验收

## 既有已证明范围

本次验收在临时 SQLite 新库上执行当前全部 migration，并编译完整 `elon-server` 测试二进制。v228 的 Store/Service 纵切面不再处于 `implementation_uncompiled/implementation_unrun`，但仍不是生产能力。

专项命令：

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\validate-rust.ps1 -Domain compute-delivery-allocation -- test --manifest-path server\Cargo.toml -p elon-server --bin elon-server delivery_allocation_service::tests:: -- --nocapture
```

验证指纹：`39387cc147cece25fafa4d5dfce2282da7d94a5175e70a90c7c36ea0e5c0f314`。

结果：3 项通过、0 项失败、1667 项过滤，测试本体耗时 7.97 秒。

## 已验证不变量

1. Provider 显式创建 Grant；缺少确认零写入；相同请求幂等重放只保留一份 immutable Grant。
2. Consumer 行权在同一事务内完成预算预授权、父 Commitment Claim `held -> released`、标准 parented Reservation Claim `held`、Job `quoted -> reserved`、Reservation `active` 和 immutable `exercised` receipt。
3. 子 Claim 的 causal transaction 精确引用父 release transaction；父子数量保持 whole-only，账面容量在事务外没有 available 间隙。
4. 相同行权重放不重复扣减余额、不重复创建 Grant 或 terminal receipt；Commitment current view 派生 `allocated`。
5. 余额不足时预算、父 Claim、子 Claim、Reservation、Job 和 terminal receipt 整体回滚；Job 保持 `quoted`、Commitment Grant 保持 `granted`。
6. Consumer Decline 可幂等重放，不冻结预算、不移动容量，terminal receipt 不携带 exercise evidence。

## 本批行权后到期恢复源码

2026-08-12 已写入管理员有界恢复入口及其 Store/Service/HTTP 和专项测试源码，用于选择 immutable terminal 已为 `exercised`、current Reservation 已到期且仍 `active`、Job 仍 `reserved`、parented child Claim 仍 `held`、尚无 Broker finish receipt 的记录，并逐项复用既有 Broker `Expire` 事务。调用方不能提交 cutoff、consumer、发生时间、revision/digest、金额或结果状态；若存在 dispatch command，仍须通过 exact no-start proof，远端状态未知时失败关闭。

预期成功效果仅为：退回 `platform_balance_cny` 预授权；child Claim `held r1 -> expired r2` 并归还容量；Job `reserved -> failed`；Reservation `active -> expired`；追加既有 Broker finish receipt。v228 terminal 不改写，仍为 `exercised`；v225 Commitment 仍为 `allocated`。恢复不产生 verified usage、Provider 收益、settlement、处罚或新经济权威。

本节是源码冻结记录，不是新增通过证据：本恢复不新增 migration，源码未编译、未运行测试或服务，`passed=0`，无新增验证指纹或运行回执。上方 3 项旧专项和其指纹不得用于声称本恢复已验证。

## 源码证据

- 行为测试：`server/src/compute_federation/delivery_allocation_store_tests.rs`。
- Service 边界：`server/src/compute_federation/delivery_allocation_service.rs`。
- Store 编排：`server/src/store/compute_delivery_allocations/`。
- 到期恢复 Store 叶文件：`server/src/store/compute_delivery_allocations/reservation_expiry_recovery.rs`。
- 到期恢复未执行 HTTP 合同测试：`server/src/compute_federation/delivery_allocation_reservation_expiry_api_tests.rs`。
- Claim、Broker、Reservation 私有入口：`server/src/store/compute_capacity_claims/delivery_allocation.rs`、`server/src/store/compute_broker_reservation/orchestrate/delivery_allocation.rs`、`server/src/store/compute_reservation_contract_validation/delivery_allocation.rs`。

## 尚未证明

- 本批行权后 Reservation 到期恢复不新增 migration，源码未编译或运行；未证明 selector、幂等重放、部分成功、no-start 失败关闭与零副作用。
- 未执行 HTTP 路由、会话鉴权、跨账户隐藏，以及 Grant/Reservation 两类 admin 到期入口验收。
- 未执行同 Grant 并发行权/Decline/Expire 竞争、文件重开或历史数据库升级。
- 未运行真实 TCP、PC、MCP、worker、dispatch、Attempt、Lease 或生产部署。
- `platform_balance_cny` 仅为本地预算预授权；没有真实支付、提现、Provider 收益、链上资金或 Sui 提交。
- 不包含 partial、多 Job、转让、转售、Order、Trade、Position、Clearing 或真实未来交付结算。

因此，v228 整体状态只能表述为 `implementation_partially_verified`，其中本批 downstream Reservation due-expiry recovery 必须单独标记 `source_written/implementation_uncompiled/implementation_unrun`。后续验收必须继续引用 [`delivery-allocation-authority.md`](delivery-allocation-authority.md) 的 whole-only、单终态和 Store-private authority 边界。
