---
title: Capacity Instrument v238 权威
status: current
design_status: design_frozen
implementation_status: source_written
verification_status: not_run
reviewed_at: 2026-08-12
owners: backend, ai-economy
---

# Capacity Instrument v238 权威

## 1. 状态与目标

v238 以 **FREEZE** 冻结 `capacity_future` 的最窄标准合约权威：平台先登记一个不可变 `CapacityInstrument`，由另一名管理员激活，再把一个已经发布且仍为 current active 的 exact Offer 以不可变 adoption receipt 采用到该合约。此后新鲜报价、锁价、预留、容量承诺和 whole-only 交付必须同时绑定 exact Instrument、Offer version/digest 与 publication identity。

当前状态仅为 `source_written/implementation_uncompiled/implementation_unrun`、`passed=0`。领域合同、v238 migration、Store、Service、管理员 HTTP、下游 Store 门卫、数据库直写门卫和未执行测试源码已经写入；没有编译、执行 migration、运行测试、启动服务或生产部署证据。验收边界见 [`capacity-instrument-acceptance.md`](capacity-instrument-acceptance.md)。

这不是 Order、Trade、Position、ClearingReceipt、指数价、标记价、真实价格源、撮合、可信计量、任务执行、Provider 收益、外部付款或结算。它只回答“哪个标准合约被哪一版已发布 Offer 精确采用，以及 fresh `capacity_future` 消费能否继续”。

## 2. 不可变标准合约

`compute_capacity_instruments` 保存固定 revision 1、JCS/sha256 的 root，合同字段为：

- `instrument_id`、`instrument_revision=1`、`instrument_digest`；
- exact `sku_id/sku_digest`；
- 一个 UTC 半开 `delivery_window`，含 window ID/digest 与起止时间；
- 按 meter 严格升序、1–64 项的 `contract_units`，每项固定 `meter/unit_size/quantity_units`；
- `availability_sla_tier`、`region_or_data_zone`、`verification_tier`；
- 固定 `settlement_currency=CNY` 与 `settlement_unit=platform_balance_cny_micros`；
- registrar、固定确认、幂等 scope/key 与 Store 时间。

meter 不能重复，`unit_size` 与 `quantity_units` 必须为正，且合同数量必须整除到 unit size。root 一经登记不可 UPDATE、DELETE、replace，也不通过后续 Offer 或价格曲线变化重写。

v238 不修改既有 Offer、Price Snapshot、CapacityCommitment、DeliveryAllocation、Job 或 Reservation JSON/schema/digest。它通过新表、读取审计和下游门卫叠加权威，不追改历史对象。

## 3. 生命周期与 registrar 分离

current status 只能从追加式事实派生：

| 状态 | 形成事实 | fresh `capacity_future` 消费 |
|---|---|---|
| `registered` | 只有 immutable root | 拒绝 |
| `active` | 唯一 activation receipt | 允许继续精确审计 |
| `retired` | active 后唯一 retirement receipt | 拒绝 |

Activation、Retirement 与 Offer Adoption 的 actor 都必须不同于 registrar；合同不要求三名后续 actor 彼此不同。每类写入都使用独立幂等 scope/key、固定确认、expected revision/digest 和 immutable receipt。Retirement 必须发生在 Activation 之后，不能重开、撤销或形成第二个 current root。

`compute_capacity_instrument_current` 只是 root LEFT JOIN activation/retirement 的可审计投影，不是可修改余额或状态真源。

## 4. exact Offer Adoption

一个 Offer 只有满足以下条件，才可登记唯一 `compute_capacity_instrument_offer_adoptions` receipt：

1. Instrument 当前为 exact active 且尚未退休；
2. Offer 是 current active `capacity_future`，其 `instrument_id` 精确指向该 root；
3. Offer 的 SKU/digest、SLA、区域、验证等级和 CNY 合同与 Instrument 相等；
4. Offer 包含 exact delivery window；价格 component 的 meter/unit size 与合同集合相等；每个容量 bucket 的 quantum 相等且 reservable units 足以容纳一个合约单位；
5. adoption 绑定 exact Offer version/digest，以及发布该 active version 的 publication ID/digest；
6. adoption actor 与 registrar 分离，并提供固定确认和幂等键。

Offer 仅携带同名 `instrument_id` 不构成采用；Offer 发布也不自动采用。Adoption 不改变 Offer、publication 或 Instrument，只保存不可覆盖的精确关系。历史 adoption 可读取，但退休后不能继续为 fresh 消费授权。

## 5. fresh 消费门与历史收尾

Store 与 v238 SQLite trigger 同时失败关闭下列 `capacity_future` 新鲜路径：

- 新 Price Snapshot；报价候选与新 `quoted` Job；
- Broker 新 Reserve、`pending/active` Reservation 与 reserve receipt；
- 新 CapacityCommitment；
- 新 DeliveryAllocation Grant 与 Exercise。

每条路径必须重审 exact active Instrument、immutable adoption、Offer version/digest、publication identity 和消费者自身的 Snapshot/Offer/instrument 绑定。CapacityCommitment 还必须是完整合约单位的共同整数倍：所有 meter 的 claim quantity 都分别整除合同 `quantity_units`，且所得 multiplier 完全相同；不能拼出各 meter 倍数不一致的“合约”。

Grant 仍要求 current exact active Offer。Exercise 可继续使用 Grant 已绑定的 exact historical Offer：该 Offer 后续保持 stable Provider/Pool/SKU 身份进行 `active -> active` 升版，或再由任一后继 active 版本进入 `draining`，都不改写 Grant 的历史绑定；但 Instrument 仍必须 active 且未退休，adoption/publication 必须精确匹配。

Retirement 只切断 fresh admission，不锁死已有对象的安全收尾。既有 Commitment 的 Cancel/Expire、Grant 的 Decline/Expire，以及已形成 Reservation 的取消、到期、退款和容量归还必须继续按原 immutable authority 运行；这些终态不创建新报价、持仓、计量或结算。

所有门只作用于 `pricing_mode=capacity_future`。`spot`、`index_locked`、`capacity_forward` 即使历史字段中携带 `instrument_id`，也不由 v238 自动升级或拦截。

## 6. 管理员 HTTP 边界

所有入口只允许平台 `admin|owner`：

- `GET/POST /api/admin/compute/capacity-instruments`；
- `GET /api/admin/compute/capacity-instruments/:instrument_id`；
- `POST /api/admin/compute/capacity-instruments/:instrument_id/activate`；
- `POST /api/admin/compute/capacity-instruments/:instrument_id/retire`；
- `GET /api/admin/compute/capacity-instruments/:instrument_id/currentness`；
- `GET/POST /api/admin/compute/offers/:offer_id/capacity-instrument-adoption`。

请求拒绝未知字段；actor、receipt ID、digest、发生时间和幂等 scope 由 Service/Store 生成，调用方只能提交业务合同、expected identity、幂等 key、固定布尔确认及退休原因。读取不存在返回 not found；无效合同与 immutable 冲突保持不同错误语义。

## 7. v238 数据库与旁路门卫

Migration 新增 root、activation、retirement、adoption 四张追加式表、current view 与只读 historical-exercise authority view，并为 exact publication identity 建唯一索引。投影 trigger 对 JCS JSON 与列逐字段核对；source trigger 对 registrar 分离、时间顺序、Offer/Instrument/publication 字段绑定做数据库级复核；所有 v238 表禁止 UPDATE/DELETE/replace。Publication 的 canonical digest 仍由 Store 读取审计重算；SQLite 没有独立 SHA-256 验证器，因此拥有任意数据库写权限的进程并不因 v238 trigger 自动成为可信发布者，不能把字段一致性门卫描述为独立密码学证明。

下游 trigger 与 Store 门卫是双层保护：绕过 HTTP/Service 直接插入 Snapshot、Job、Reservation、Broker receipt、Commitment 或 DeliveryAllocation 也不能省略 adoption/currentness。历史 Exercise 的例外只接受 Store 既有的 parent Commitment release→child Reservation hold 因果链、whole-only lines/ledger legs，并审计 adopted historical active Offer 与 exact current active 后继版本；若已进入 `draining`，还须存在从 stable active 后继版本到 exact 首个 draining 版本的 lifecycle event，并逐字段审计 exact current draining 后继版本。普通 Broker/API 路径不能借此例外继续。任何缺失、过期、退休、版本漂移、摘要漂移、窗口/SKU/meter 不一致或 Commitment 非共同倍数都必须整笔失败，不得退化为“先写后补”的 staging。和 Publication digest 一样，这些关系型 trigger 不是对掌握任意 SQLite 写权限进程的签名或 authorizer：旧 Claim/Ledger schema 不具备不可伪造调用栈证明，故本批只宣称合法 Store/API producer 与已知直接漏写的失败关闭，不宣称抵抗数据库所有者伪造。

## 8. 冻结边界

v238 只建立标准合约和采用权威，不发行可转让资产，也不移动容量或资金。Contract units 是下游数量整齐性的模板，不是余额；Offer reservable units 和 CapacityPool ledger 仍分别承担出售上限与全局防超卖。Price Snapshot 仍只是锁价合同；`fallback_curve/sample_count=0` 仍不是真实 market price。

后续 Order/Trade/Position/Clearing、真实指数/标记价、可信 runtime metering、Adapter/Runner、保证资源、违约赔付和外部支付必须建立独立 authority 与验收，不能扩展 v238 receipt 或把 `active` Instrument 解释成市场、交付或结算已完成。
