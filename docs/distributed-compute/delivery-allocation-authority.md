---
title: Delivery Allocation v228 权威
status: current
design_status: design_frozen
implementation_status: source_not_written
last_updated: 2026-08-12
---

# Delivery Allocation v228 权威

## 1. 裁决与边界

v228 以 **GO** 冻结一个非 staging 的最窄真实交付纵切面：Provider 把一份 exact v225 Capacity Commitment 全量、双边地授权给一个消费者的一个 exact quoted Job；消费者在交付窗口开始前显式行权；Store 在一个 `BEGIN IMMEDIATE` 中把既有 Commitment Claim 的全部 held 容量转成既有 Broker 能消费的标准 Reservation Claim，并原子登记预算、Reservation、Job 与 Broker reserve receipt。

设计状态固定为 `design_frozen`，实现状态固定为 `source_not_written`。本文件只冻结未来源码的权威合同，不构成 migration、Store、Service、HTTP、测试或运行证据。

该纵切面不是 Order、Trade、Position、ClearingReceipt 或买方可转售持仓，也不声明真实成交价、指数价、标记价、保证金、交割差额或结算成功。它只复用 v225 Commitment、v165-v168/v173 Claim/ledger、v172 Job、v174 Reservation 和 v175 Broker 的本地 `platform_balance_cny` 预授权链。

## 2. Whole-only 单一所有权

一份 Commitment 一生最多创建一份 Grant；一份 Grant 最多形成一个终态。P0 不允许部分数量、多 Job、换 Job、再次授权、撤销授权、转让或转售。

容量所有权只有两段：

| 阶段 | 唯一 live Claim | 状态与数量 |
|---|---|---|
| Grant 尚未行权 | v225 Commitment Claim | revision 1、`held`；完整 bucket/meter/quantity |
| Grant 已行权 | 标准 Reservation Claim | revision 1、`held`；与父 Claim 完整相等 |

不得创建 `delivery_allocation` Claim。Allocation 是不可变授权与谱系回执，不是第二份容量余额。父、子 Claim 必须绑定相同 Pool、epoch、交付窗口、Offer 和完整排序后的 bucket/meter/quantity；API 不接受数量、bucket、价格、窗口或 expiry。任一行缺失、多出或数值不同都整体拒绝。

## 3. 只新增两张不可变表

v228 只新增两张表，并重建既有 `compute_capacity_commitment_current` view；不新增数量表、余额表或 mutable current projection。

### `compute_delivery_allocation_grants`

每份 Grant 是 revision 1 的不可变 root：

- 信封：`grant_id` 主键、固定 `schema_version=v1`、`grant_revision=1`、`grant_status=granted`、`grant_digest`、JCS JSON 与 `digest_algorithm=sha256`；
- 来源：`commitment_id UNIQUE`、固定 `commitment_revision=1`、`commitment_digest`；
- 双边与 Job：`provider_owner_account_id`、`consumer_account_id`、nullable `project_id`、`job_id UNIQUE`、`job_revision`、`job_digest`；Provider owner 与 consumer 必须是不同账户；
- 时钟：`exercise_expires_at`、`created_at`，均由 Store 生成；`exercise_expires_at` 必须精确等于 Commitment 交付窗口 `starts_at`；
- 重放：`idempotency_scope`、`idempotency_key`、`request_digest`，scope/key 组合唯一。

表及 JCS JSON 禁止 UPDATE/DELETE。Grant 不复制 Commitment Claim lines、Offer、Pool、Snapshot、reference binding、SKU、instrument、价格或余额；读取时从 exact v225 Commitment、Claim、Job 及其历史依赖重建并校验。`grant_id`、`grant_digest`、`commitment_id`、`job_id` 与幂等 scope/key 分别唯一；Decline/Expire 后也不允许同一 Commitment 或 Job 再授权，需求方必须创建新 Job。

### `compute_delivery_allocation_terminal_receipts`

每份 Grant 最多一份 revision 2 终态回执：

- 信封：`terminal_receipt_id` 主键、固定 `schema_version=v1`、`terminal_revision=2`、`terminal_status IN (exercised,declined,expired)`、`terminal_receipt_digest`、JCS JSON 与 `digest_algorithm=sha256`；
- 前序：`grant_id UNIQUE`、`grant_digest`、`commitment_id`、`commitment_digest`；
- 主体：`actor_kind IN (consumer,admin)`、`actor_id`；
- 重放与时钟：`idempotency_scope`、`idempotency_key`、`request_digest`、`occurred_at`、`recorded_at`，scope/key 组合唯一；
- 仅 `exercised` 非空的父释放证据：`parent_claim_id`、prior/result revision 与 digest、固定结果状态 `released`，以及 release ledger transaction ID/digest/sequence/event 和原始 hold causal transaction ID；
- 仅 `exercised` 非空的子持有证据：`reservation_claim_id`、revision/digest、其 `parent_claim_id`，以及 hold ledger transaction ID/digest/sequence/event 和 causal parent release transaction ID；
- 仅 `exercised` 非空的 Broker 证据：`reservation_id`/revision/digest、source/reserved Job revision/digest、`budget_reservation_id`、`reserved_amount_fen` 和既有 Broker reserve request digest。

`declined|expired` 的全部 exercise-only 列必须为 NULL；`exercised` 的全部列必须齐全，父子 ID、event、revision、status、causal transaction 与 JCS 必须逐项相等。`exercised|declined` 的 actor 必须是 Grant 的 exact consumer，`expired` 的 actor 必须是已鉴权 platform admin。数据库 CHECK、外键、唯一约束和 immutable trigger 共同失败关闭；表及 JSON 禁止 UPDATE/DELETE。

## 4. 派生状态机

Grant root 永远是 revision 1 `granted`；唯一 terminal receipt 是 revision 2：

| 当前 Grant 状态 | 谁能形成 | Commitment current status | 容量效果 |
|---|---|---|---|
| `granted` | Provider 创建 | `committed` | 父 Claim 继续 held |
| `exercised` | exact consumer | `allocated` | 父完整 release，子完整 hold |
| `declined` | exact consumer | `committed` | 无账本效果 |
| `expired` | admin/Store due recovery | `committed` | 无账本效果 |

Provider 不得 revoke。消费者只能在未形成终态时 decline 或 exercise；平台只能在 Store 时间达到 `exercise_expires_at` 后 expire。当前状态一律由 Grant LEFT JOIN terminal receipt 派生，不存在可改写 current row、revision 3、重开或第二终态。

必须 drop/recreate `compute_capacity_commitment_current`：有 `exercised` receipt 时派生 revision 2、`current_status=allocated` 和父 Claim 的 released result；否则优先保留 v225 `canceled|expired` terminal，最后为 `committed`。view 同时公开 nullable grant/terminal ID 供审计。数据库门卫保证同一 Commitment 不会同时成为 `allocated` 与 v225 `canceled|expired`。

## 5. Snapshot TTL 权威例外

v174 的通用 Reservation 规则要求新 Reservation 引用未过期 Snapshot，且 `reservation.expires_at <= snapshot.expires_at`。v223 reference Snapshot TTL 上限为 3600 秒，因此它不能直接支持较长的未来交付窗口。v228 只冻结以下窄例外：

1. 创建 Grant 时 exact v171 Snapshot 必须仍未过期；Grant JCS 固定 exact Commitment、Snapshot 与 Job digest。
2. 行权可发生在 Snapshot 过期后，但 Store 时间必须严格早于 `exercise_expires_at=window.starts_at`。
3. 继续引用原 immutable v171 Snapshot；禁止 UPDATE、延长或复制其 `expires_at`，也禁止补造新 Snapshot。
4. Store-private `DeliveryAllocationReservationAuthority` 只能由同一事务中 exact active Grant 行权候选构造，或由 persisted `exercised` receipt 为历史读取/重审计恢复；不得由 Service/API/泛用 Store 调用方构造。它同时携带 exact pre-held child Reservation Claim authority，不能从普通参数拼装。
5. 该 authority 仅允许忽略 Snapshot quote TTL 与 exact current Snapshot version，并令 `Reservation.expires_at=Job.deadline_at <= window.ends_at`；它仍要求 exact 历史 Offer/Snapshot、current Provider、current Offer `active|draining`、安全 Pool、SKU/window/instrument/full meter 与 Job 限额全部一致。
6. 泛用 `register_compute_reservation(_on)` 继续执行旧 TTL/currentness 规则，不得接受该例外、parented Claim 或 caller-supplied authority。

不得给 `ComputeReservation v1` 增字段，否则会改变历史 JSON/digest；不得修改 v171/v174 历史行或旧 migration。

## 6. HTTP API 冻结

Provider owner：

- `POST /api/me/compute/providers/:provider_id/capacity-pools/:pool_id/capacity-commitments/:commitment_id/delivery-allocation-grant`；
- `GET /api/me/compute/providers/:provider_id/capacity-pools/:pool_id/capacity-commitments/:commitment_id/delivery-allocation-grant`。

POST body 只允许 `idempotency_key`、expected Commitment revision/digest、`consumer_account_id`、`job_id`、expected Job revision/digest 和 `confirm_grant=true`。Grant ID、时间与 expiry 均由 Store 生成。

Consumer：

- `GET /api/me/compute/delivery-allocation-grants?status=&limit=`；
- `GET /api/me/compute/delivery-allocation-grants/:grant_id`；
- `POST /api/me/compute/delivery-allocation-grants/:grant_id/exercise`，body 只允许 `reservation_id`、`idempotency_key`、expected Grant revision/digest、`confirm_financial_action=true`；
- `POST /api/me/compute/delivery-allocation-grants/:grant_id/decline`，body 只允许幂等键、expected Grant revision/digest 与固定确认。

Admin：`POST /api/admin/compute/delivery-allocation-grants/expire-due`，要求固定确认、`limit=1..100`，时间完全由 Store 生成。

任何 API 均不接受 caller-supplied quantity、bucket、meter、price、Snapshot、window、expiry、occurred time、Claim ID、ledger transaction ID 或预算金额。响应只公开安全投影与 exact immutable receipt。

## 7. Grant 事务

`create_delivery_allocation_grant` 独占一个 `BEGIN IMMEDIATE`：

1. 先按 scope/key 读 immutable Grant；同 request digest 返回原 revision 1，不同 digest 拒绝。
2. 重审计 exact v225 committed root/detail、无 v225 terminal、无既有 Grant，父 Commitment Claim 为 revision 1 `held`。
3. Store 时间必须早于 Snapshot expiry 和交付窗口 `starts_at`；Provider、Offer 为 current `active`，Pool 可预留且不处于 quarantine/draining 冲突状态。
4. Job 必须仍为消费者或其 exact project 的 current `quoted` revision；其 Offer/Snapshot/SKU/window/instrument、完整 meter 和 Job limit 必须与 Commitment 全量相等。
5. 插入 Grant，按 JCS/digest、依赖、Claim lines 和唯一约束精确回读后提交。

Grant 创建不移动 ledger、预算或 Job 状态，也不创建 Reservation。任何检查失败均零副作用。

## 8. Exercise 事务

`exercise_delivery_allocation_grant` 独占一个 `BEGIN IMMEDIATE`，顺序固定：

1. 先按 terminal scope/key 查 immutable result；同 digest 返回原完整结果，不同 digest 拒绝。
2. 重审计 Grant 仍 `granted`、actor 是 exact consumer、expected revision/digest 匹配、无 v225 terminal；父 Commitment Claim 仍为 exact revision 1 `held`。
3. Store 时间严格早于 window `starts_at`；Job 仍是 exact current `quoted` revision；current Provider、Offer 稳定 ID 且状态 `active|draining`，Pool 仍安全。
4. 按既有 v175 Broker 规则冻结 `platform_balance_cny` 预算；金额和 expiry 从 exact Snapshot/Job 派生，调用方不能提交。
5. 以 private v228 wrapper 把父 Claim 全量 `held -> released`、ledger `held -> available`，事件为 `reservation_released`。
6. 在同一事务且任何外部读取不可见前，以 full-equality lines 创建标准 Reservation Claim；其 `parent_claim_id` 固定为父 Claim，事件为 `reservation_held`、ledger `available -> held`，causal transaction 固定为第 5 步 parent release transaction。
7. 通过 Store-private `reserve_compute_job_with_preheld_claim_on(connection, DeliveryAllocationReservationAuthority)` 或等价单职责 kernel，adopt 第 6 步的标准 child Reservation Claim，登记既有 Reservation `pending -> active`、Job `quoted -> reserved` 和既有 Broker reserve receipt；该 kernel 不调用 generic Hold，也不得另建 Claim。
8. 插入 `exercised` terminal receipt，精确回读预算、父子 Claim、两笔 ledger transaction、Reservation、Job、Broker receipt 与全部摘要后提交。

父 release 与子 hold 必须同事务成功或回滚，因此数据库外不存在可抢占的 available 间隙。这里复用 ledger 的既有事件/账户，不新增 reducer 或余额权威。

## 9. Decline、Expire 与 v225 竞争

Decline 由 exact consumer 调用：重放优先，Grant 必须 active、来源 Commitment/父 Claim 仍 committed/held、Store 时间早于窗口开始；只插入 immutable `declined` receipt，不移动 ledger、预算或 Job。

Expire 候选是已到 `exercise_expires_at` 且仍无 terminal 的 Grant。每个候选单独使用一个 IMMEDIATE 事务、确定性幂等键与 Store 时间，重审计来源仍 committed/held 后插入 `expired` receipt；不移动 ledger。批次部分成功，limit 为 `1..100`，不要求后台 scheduler。

active `granted` 或 `exercised` Grant 阻止 v225 Cancel/Expire 及其 recovery selector；`declined|expired` Grant 允许原 v225 规则继续工作：消费者拒绝或错过行权并不替 Provider 释放容量，Provider 仍可在窗口开始前 Cancel，平台仍在窗口结束时 Expire。反向地，v228 Grant/Exercise 必须拒绝已有 v225 terminal。

## 10. P0 旁路门卫

- public generic Hold 同时拒绝 Commitment 和 DeliveryAllocation subject，不能设置 `parent_claim_id`；只有 v228 exercise wrapper 可创建 exact parented Reservation Claim。
- 子 Hold 使用版本化 request digest，固定父 Claim、parent release transaction、完整 lines、Job、Reservation、Offer/Snapshot；任何字段漂移均不是重放。
- public generic Finish 继续拒绝 Commitment。private parent release 必须与 `exercised` receipt、child Hold、Broker result 同事务提交，否则整体回滚。
- generic Reservation registration 不能获得 Snapshot TTL 例外或 parented Claim；未来历史读取只能从 persisted exercised authority 重建。
- generic Broker Reserve 继续自行创建普通 Reservation Claim，不能接收 pre-held Claim；只有 v228 private pre-held kernel 能 adopt exact child Claim，且不得形成第二个 Claim。
- v225 terminal insert、Cancel、Expire 与 recovery selector 必须排除 active/exercised Grant；v228 反向排除已有 v225 terminal。
- 所有 grant、terminal、Claim、transaction、Reservation、budget ID 与幂等结果都以唯一约束、CAS、写后重审计失败关闭；list/get/status filter 必须识别 `allocated`。
- Settlement 仍只信既有 Broker receipt、预算与后续 verified usage；Grant/Allocation receipt 不是 usage、价格、支付或结算真源。

## 11. P0 文件预算与禁线

未来实现预算固定为：1 个 v228 migration 入口叶文件和最多 3 个 schema/view/trigger 叶文件；DeliveryAllocation 领域模型最多 2 个叶文件；Store 编排、query/replay、Claim transfer authority 最多 5 个叶文件；Service 与 HTTP 各最多 2 个叶文件；定向测试最多 4 个叶文件。每个新增源码叶文件目标 `<450` 行；中央 migration/module/router 只允许小幅注册。超过预算必须先拆职责并更新本权威，不得把多个语义塞进既有巨型文件。

P0 明确禁止：partial/multi-Job/regrant/transfer/resale；Order/Trade/Position/Clearing；真实 price/index/mark；保证金、交割罚金、Provider 收益或新结算 ABI；`external_pool`、remote saga、staging/provisional；MCP、PC、worker、dispatch；Attempt/Lease、verified metering、生产部署或运行。不得修改 v171/v174/v225 历史表、JSON、digest 或 migration。

P1 才可讨论部分分配、多 Job、可转让 Position、真实市场价格、保证资源、自动清算、external pool 与生产结算；它们不能通过扩展 v228 请求体、状态 enum 或 Claim kind 旁路进入 P0。

## 12. 冻结结论

v228 的完整性来自单一事务内的父 Claim 全量释放、标准子 Reservation Claim 全量持有、既有 Broker 预算/Job/Reservation 登记和 immutable exercised receipt。只要任一环未同事务接线，就必须保持 `source_not_written` 或失败关闭；不得降级成只写 Grant/receipt 的 staging 能力，也不得宣称未来交付、执行或结算已经生产可用。
