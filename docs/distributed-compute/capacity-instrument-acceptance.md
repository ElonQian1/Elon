---
title: Capacity Instrument v238 验收边界
status: current
design_status: design_frozen
implementation_status: source_written
verification_status: not_run
acceptance_status: pending
reviewed_at: 2026-08-12
owners: backend, ai-economy
---

# Capacity Instrument v238 验收边界

## 1. 当前结论

v238 的领域合同、migration、Store、Service、管理员 HTTP、下游 Store/SQLite 门卫及测试夹具源码已经写入。当前只能报告 `source_written/implementation_uncompiled/implementation_unrun`、`passed=0`：未编译 Rust，未执行 v238 migration，未运行 Store/Service/HTTP/SQLite 测试，未启动服务，也未做文件重开、历史库升级、真实 TCP 或生产部署。

此前 v225 CapacityCommitment 与 v228 DeliveryAllocation 的编译、临时 SQLite 和专项测试是 v238 进入基线前的历史证据，只证明旧纵切面；它们没有执行 v238 新 migration、Instrument lifecycle/adoption 或新增下游门卫，不能作为 v238 的验证指纹，也不能把本批状态升级为 `implementation_partially_verified`。

## 2. 已写入的源码证据

- 领域合同与 canonical validation：`server/src/compute_federation/capacity_instrument/`；
- 管理员 Service/HTTP：`server/src/compute_federation/capacity_instrument_service.rs`、`server/src/compute_federation/capacity_instrument_api.rs`；
- Store root/lifecycle/adoption/currentness：`server/src/store/compute_capacity_instruments/`；
- v238 tables、projection/source guard 与 downstream direct-SQL guard：`server/src/store_migrations/compute_capacity_instrument/`；
- fresh consumer 接线：Price Snapshot、quote candidate、quoted Job、Broker Reservation、CapacityCommitment 与 DeliveryAllocation 的 Store validation；
- 未执行测试/夹具：`server/src/compute_federation/capacity_instrument_api_tests.rs`、`server/src/compute_federation/capacity_instrument_test_support.rs` 及复用该 support 的 Commitment/Allocation 测试。

源码存在不是运行证据。文档模块化与 `git diff --check` 即使通过，也只证明文档/补丁静态整洁，不证明 Rust 类型、SQLite trigger 或业务事务正确。

## 3. 后续必须通过的专项

| 能力 | 必须观察的结果 | 当前 |
|---|---|---|
| Register/重放 | exact root 只写一次；同键异参和非规范合同零写入 | 未运行 |
| Activation/Retirement | actor 与 registrar 分离；registered 不可退休；单一 current 状态 | 未运行 |
| Offer Adoption | 只接受 active exact future Offer、publication、SKU/window/meter；漂移零写入 | 未运行 |
| 管理员 HTTP | 未登录/非管理员拒绝；未知字段、缺确认、stale digest 与 limit 错误语义稳定 | 未运行 |
| fresh consumer gate | 无 adoption、registered/retired instrument、stale Offer/publication 均阻止 Snapshot→quote→Broker→Commitment→Grant/Exercise | 未运行 |
| Commitment 整倍数 | 多 meter 只能采用同一正整数 multiplier；缺 meter、多 meter 倍数不同与 SQL 直写均失败 | 未运行 |
| 历史收尾 | Retirement 后既有 Cancel/Expire/Decline、退款与容量归还仍可完成且不新增经济事实 | 未运行 |
| 兼容路径 | 非 `capacity_future`，包括携带 instrument_id 的 `capacity_forward`，行为不变 | 未运行 |
| 耐久与竞争 | 新库、历史升级、两次重开、并发 adopt/retire/consume、崩溃边界保持唯一事实 | 未运行 |

## 4. 验收失败表现

出现以下任一情况即拒绝本批：

- 未激活、未采用或已退休 Instrument 仍可形成 fresh future Snapshot、Job、Reservation、Commitment、Grant 或 Exercise；
- 仅凭 Offer 中的 `instrument_id` 即跳过 exact publication adoption；
- Store 拒绝但 raw SQLite 写入成功，或反之形成不同权威；
- Commitment 各 meter 使用不同合约倍数；
- Retirement 把历史取消、到期、退款或容量归还锁死；
- v238 改变既有 Offer/Snapshot/Commitment/DeliveryAllocation JSON、digest 或非 future 行为；
- 把 Instrument/Adoption 描述为真实成交、价格发现、可信计量、执行、收益或结算。

## 5. 当前剩余边界

v238 验收完成后，也只证明标准合约采用门。它仍不提供 Order、Trade、Position、ClearingReceipt、真实 index/mark/trade price、可转售持仓、可信 runtime metering、Adapter/Runner、保证资源、违约赔付、Provider 可提现收益或外部付款。整体容量市场与可信执行—计量—结算链继续保持未完成。
