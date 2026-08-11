---
title: Provider Capacity Commitment v225 权威
status: current
design_status: design_frozen
implementation_status: implementation_partially_verified
reviewed_at: 2026-08-12
owners: backend, ai-economy
---

# Provider Capacity Commitment v225 权威

## 1. 状态与最窄目标

v225 已写入 Provider 为一个 `capacity_future` Offer 的单一交付窗口锁定容量的最窄闭环。设计状态保持 `design_frozen`，实现状态为 `implementation_partially_verified`：领域模型、v225 两表与门卫、Store 原子 create/read/cancel/expire、通用 Claim 旁路封口、owner/admin Service 与 HTTP 路由均已接线；截至 2026-08-12，`elon-server` 生产目标编译及 3 项 Store/Service/进程内 HTTP 定向测试通过，新建临时 SQLite 可执行全量迁移，磁盘重开可恢复终态。后续 owner source 查询复用 v171 Snapshot 所有权门卫和 v223 binding 读取，PC 控制面嵌入现有 `/compute-supply` Offer 工作区，不创建第二套 Broker 或市场入口。真实 TCP、生产升级、并发压力、浏览器、后台任务和交付结算仍未验证。

该纵切面复用 v165-v168/v173 的 Claim 与容量账本、v169 Provider、v170 Offer、v171 Price Snapshot，以及 v223/v224 已审核的平台 reference binding。它不依赖节点插件/VFS、真实任务运行、verified metering、Delivery Allocation、资金或结算。

## 2. 单一权威与两表

v225 只新增两张表，不新增 Commitment 数量表、余额表或 mutable current projection：

### `compute_capacity_commitments`

不可变的 revision 1 `committed` 事实，字段冻结为：

- 信封：`commitment_id`、固定 schema、`commitment_revision=1`、`commitment_status=committed`、`commitment_digest`、JCS JSON；
- 主体：`owner_account_id`；
- Provider：`provider_id`、`provider_policy_revision`、`provider_digest`；
- Offer：`offer_id`、`offer_version`、`offer_digest`；
- Pool：`pool_id`、`capacity_epoch`、`pool_revision`、`pool_digest`；
- 窗口：`delivery_window_id`、`delivery_window_digest`、`starts_at`、`ends_at`；
- 价格：`price_snapshot_id`、`price_snapshot_digest`、`reference_binding_id`、`reference_binding_digest`、`instrument_id`；
- 容量因果：`claim_id`、`claim_revision=1`、`claim_digest`、创建 ledger transaction ID/digest/sequence/event kind；
- 重放：`idempotency_scope`、`idempotency_key`、`request_digest`；
- 时间：Store 生成的 `created_at`、且 `expires_at=ends_at`。

`commitment_id`、`commitment_digest`、`claim_id`、创建 transaction ID 和 `(idempotency_scope,idempotency_key)` 分别唯一。表及其 JSON 禁止 UPDATE/DELETE；精确 Provider/Offer/Pool/Snapshot/reference binding/Claim version/ledger transaction 均以外键或写后重审计固定。

### `compute_capacity_commitment_terminal_receipts`

每个 Commitment 最多一份不可变 revision 2 终态回执，字段冻结为：

- 信封：`terminal_receipt_id`、固定 schema、`terminal_revision=2`、`terminal_status IN (canceled,expired)`、receipt digest、JCS JSON；
- 前序：`commitment_id`、revision 1 commitment digest；
- Claim：`claim_id`、prior revision/digest、result revision/digest/state；
- 账本：terminal transaction ID/digest/sequence，event kind 与 causal transaction ID；
- 授权：`actor_kind`、`actor_id`、可选规范化 reason；
- 重放：`idempotency_scope`、`idempotency_key`、`request_digest`；
- 时间：Store 生成的 `occurred_at`、`recorded_at`。

`commitment_id`、receipt digest、terminal transaction ID 和幂等 scope/key 分别唯一，表及 JSON 禁止 UPDATE/DELETE。当前状态只能由 commitment LEFT JOIN terminal receipt 派生；不存在可被单独改写的状态列，也不存在 revision 3、重开或第二终态。

## 3. 状态与容量映射

| Commitment | Claim | 既有 ledger event | 容量移动 |
|---|---|---|---|
| create `committed` | `capacity_commitment`, `held`, revision 1 | `reservation_held` | `available -> held` |
| cancel `canceled` | `released`, revision 2 | `reservation_released` | `held -> available` |
| expire `expired` | `expired`, revision 2 | `reservation_expired` | `held -> available` |

Commitment 只用独立 claim kind 表达业务语义，不扩展 reducer、账户或 event enum。Claim 固定 `subject_kind=compute_capacity_commitment`、`subject_id=commitment_id`、无 parent；数量唯一权威是该 Claim 的不可变 lines，余额唯一权威是既有 ledger 与 balance projection，Commitment 两表不得复制 meter/quantity 或余额。

## 4. Store API 与事务边界

当前 P0 Store 只暴露：`create_compute_capacity_commitment`、`cancel_compute_capacity_commitment`、`expire_due_compute_capacity_commitments`、单条读取和本人有界列表。任何写操作由 Store 拥有一个 `BEGIN IMMEDIATE`；Service/API 不拼接跨事务步骤。

Create 请求只接受精确 Provider/Offer/Pool/Snapshot/reference binding、`instrument_id`、完整 `meter -> quantity`、幂等键和固定确认短语。调用方不得提交 bucket ID、服务器时间、`expires_at`、部分 meter 或自行生成摘要。事务顺序固定为：

1. 先按 scope/key 查 immutable commitment；同 request digest 返回原 revision 1 写结果，不同 digest 拒绝；
2. 重审计当前 Provider、Offer、Pool、v171 Snapshot 与 v223 binding；
3. 建立唯一窗口的完整 meter 映射并校验数量与 Offer 局部上限；
4. 通过仅供外层事务使用的 Hold kernel 创建 Claim 与 `reservation_held` ledger transaction；
5. 插入 immutable commitment，精确回读 Commitment、Claim revision 1、Claim lines、ledger 与全部依赖后提交。

任何一步失败整笔回滚，不允许先写 staging、pending、provisional 或 outbox。

## 5. Exact currentness

Create 的同一事务必须同时满足：

- Provider 是当前登录用户所有，状态 `active`，当前 policy revision/digest 与请求及 Offer 绑定完全一致，且 kind 不是 `external_pool`；
- Offer 是数据库 current exact version/digest，状态 `active`，Store 时间位于 `[valid_from,valid_until)`，Provider、Pool、SKU、窗口、`capacity_future` 与 `instrument_id` 均精确一致；
- Pool 是数据库 current exact `(pool_id,capacity_epoch,pool_revision,pool_digest)`，状态 `active`，归属同 Provider，region/profile/meter policies 与 Offer 精确兼容；即使 epoch 相同，旧 revision/digest 也必须拒绝；
- v171 Snapshot 通过既有完整摘要审计，绑定同 Offer/SKU/窗口/组件/费用/`instrument_id`，`pricing_mode=capacity_future`、CNY、`trade_id=None`，且 Store 时间早于 Snapshot `expires_at`；
- v223 binding 通过既有 application/binding 全链审计，review 为 approved、application 为 applied，并精确绑定该 Snapshot；`source_kind=fallback_curve`、`sample_count=0`；
- v223 没有 mutable latest-curve 指针，因此只要求已批准并应用的 exact binding 与未过期 Snapshot，不臆造“最新 curve version”。

v225 需要 Store-private、transaction-local 的 snapshot-ID lookup 来复用上述 v223 读审计；不得复制 v223 五张表、DTO、canonicalization 或第二套价格真源。

## 6. 全 meter、窗口与 Offer 上限

Store 先以 Snapshot 的 exact 窗口筛选 Offer capacity rows，再按 meter 建唯一映射；禁止复用跨全部窗口聚合 meter 的 Broker helper。SKU metering units、该窗口 Offer rows、Snapshot components 与请求 quantities 的 meter 集合必须完全相等。

每个 quantity 必须大于零，同时整除 bucket `quantum_units` 与 Snapshot component `unit_size`，且不超过 component `max_units` 和该 Offer row 的 `reservable_units`。同一 IMMEDIATE 事务再以 checked `i128` 汇总同 `offer_id + bucket_id` 的所有 live `held|active` Quote/Reservation/Commitment Claim；已有量加新量不得超过当前 Offer `reservable_units`。共享 Pool 的全局防超卖仍由既有 reducer 和 available 余额最终裁决。

## 7. Cancel、Expire 与严格重放

Cancel 先查 terminal receipt 幂等键，再检查 owner、expected revision 1/digest、尚无终态、Claim 仍 held，并要求 Store 时间严格早于窗口开始；不要求创建后 Provider/Offer/Pool/Snapshot 仍 current 或 active。专用 exact-subject wrapper 在同一事务追加 `reservation_released`、把 Claim 推进为 released revision 2，并插入 `canceled` receipt。

Expire 只供平台 admin 恢复入口使用。候选来自 commitment LEFT JOIN 无终态 receipt，且 Store 时间不早于强制 `expires_at=ends_at`；调用方不能传 occurred/cutoff 时间。Expire 的 `occurred_at` 固定取该 Commitment 的 `expires_at`，`recorded_at` 取本事务 Store 时间。每个候选用确定性 key 单独执行一个 IMMEDIATE 事务，追加 `reservation_expired`、把 Claim 推进为 expired revision 2，并插入 `expired` receipt。批量限制 `1..100`，是显式调用的部分成功恢复，不要求后台 scheduler。

每个操作都必须在 currentness、时间与状态门卫之前查自己的 immutable 结果。同 key + 同 request digest 返回原始 Commitment/terminal receipt、Claim version 与 ledger transaction，不读取 mutable current Claim/余额来伪造首次响应；同 key + 不同 digest 拒绝。创建后已终态或来源已过期也不破坏历史重放。Cancel/Expire 竞争由 Claim CAS 与 terminal receipt 的 `UNIQUE(commitment_id)` 保证仅一方成功。

## 8. Generic bypass 封口

当前 v225 已验证以下三道 generic bypass 门卫；这不把相邻交付与结算链升级为生产闭环：

- public generic Hold 拒绝 `capacity_commitment` claim kind 或 `compute_capacity_commitment` subject；只有 Create 外层事务可调用 private Hold kernel；
- public generic Finish 像 Reservation 一样拒绝 Commitment；只有校验 exact commitment/Claim/原 held causal binding 的专用 wrapper 可 release/expire；
- generic Claim expiry recovery 明确排除 Commitment；只允许专用 Expire 同事务写 terminal receipt，禁止出现 Claim 已 expired 而 Commitment 仍 committed。

读取也必须从两张 Commitment 表、exact Claim versions/lines 和 ledger 重建并重审计；任何摘要、revision、subject、event、数量或状态不一致都失败关闭。

## 9. HTTP P0

- `GET/POST /api/me/compute/providers/:provider_id/capacity-pools/:pool_id/capacity-commitments`；
- `GET /api/me/compute/providers/:provider_id/capacity-pools/:pool_id/capacity-commitments/:commitment_id`；
- `POST /api/me/compute/providers/:provider_id/capacity-pools/:pool_id/capacity-commitments/:commitment_id/cancel`；
- `GET /api/me/compute/providers/:provider_id/capacity-pools/:pool_id/offers/:offer_id/price-snapshots/:snapshot_id/capacity-commitment-source`，只为 owner 返回 exact Snapshot 与已审核 v223 binding；
- `POST /api/admin/compute/capacity-commitments/expire-due`，要求固定确认短语、`limit=1..100`，时间完全由服务端生成。

P0 不要求 MCP、PC 页面或后台 worker。后续 PC 接线只消费上述 owner HTTP 和既有 Offer/Snapshot API；所有响应只公开安全投影和 exact immutable receipt，不公开内部路由、凭据或 adapter 配置。

## 10. 文件预算与 P0 禁线

P0 仍只包含领域合同、v225 migration、Store create/read/terminal、通用 Claim seam、薄 Service 与 HTTP API 这些逻辑边界。为遵守单一职责和叶文件尺寸门卫，migration、Store 与 replay 实现按职责拆成多个 `<450` 行叶文件；这种机械拆分不扩大业务范围。中央 migration/module/router 只做小幅注册，MCP、PC、worker、测试运行或相邻市场对象不得借文件拆分混入本批。

P0 明确禁止：`external_pool`；DeliveryAllocation；Order/Trade/Position/ClearingReceipt；资金预授权、收费、Provider 收益、保证资源、处罚、结算或清算；Job/Reservation/Attempt/Lease 修改；真实价格/index/mark/trade 声明；节点插件、VFS、artifact、route、派发或 verified metering 接线；staging/provisional/saga；调用方 bucket/time/expiry/部分 meter；新 reducer/event/余额权威。

当前已形成生产目标编译、临时 SQLite 全量迁移、Store/Service/进程内 HTTP、磁盘重开，以及 PC 静态生产构建证据，详见 [`capacity-commitment-acceptance.md`](capacity-commitment-acceptance.md)。仍缺真实 TCP、跨连接并发、已有生产数据库升级、浏览器、异常恢复和生产运行证据，因此只能标记为 `implementation_partially_verified`，不得描述为容量市场、交付或结算全链可用。

## 11. v228 后继边界

v225 P0 禁止 DeliveryAllocation 仍是该批次的历史事实；v228 不回写 v225 两表、JCS、digest 或 migration。后继 [`delivery-allocation-authority.md`](delivery-allocation-authority.md) 只允许一份 Commitment 创建一份 whole-only 双边 Grant，并由 exact consumer 在同一 IMMEDIATE 事务中把父 Commitment Claim 全量 release、建立 exact parented 标准 Reservation Claim、登记既有 Broker 结果和 immutable exercised receipt。

`compute_capacity_commitment_current` 在 v228 migration 中只能重建 view：exact exercised receipt 派生 revision 2、`current_status=allocated`；Grant 仍 active、declined 或 expired 时仍为 `committed`；既有 v225 terminal 仍派生 `canceled|expired`。active/exercised Grant 阻止 v225 Cancel/Expire/recovery，v228 反向拒绝既有 v225 terminal，因此不存在 allocated 与 canceled/expired 双终态。

Snapshot TTL 例外、`parent_claim_id`、父 release→子 hold 的因果链和泛用入口旁路门卫全部只属于 v228 sealed Store-private authority。v225 自身的 verified 边界与验收结论不因后继设计而升级；v228 当前状态为 `design_frozen/implementation_uncompiled/implementation_unrun`：源码已写入，但尚未编译、执行 migration、运行测试或启动服务。
