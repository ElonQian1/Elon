---
title: 联邦核心历史因果引用 Carrier ABI 权威
reviewed_at: 2026-08-22
status: current
owners: backend, node, ai-economy, pc
design_status: design_frozen
design_scope: federation_core_historical_causal_reference_carrier_abi_v1
implementation_status: implementation_uncompiled
verification_status: source_review_only
---

# 联邦核心历史因果引用 Carrier ABI 权威

## 1. 唯一结论与状态边界

本页冻结 Provider-neutral 的历史引用 primitive、Execution Receipt 与 Settlement Receipt 两种基础只读因果 Carrier，
以及它们的 additive read adoption。Domain、Store 只读 resolver、Service、HTTP/MCP 与 PC 客户端源码现已落盘；
这些入口只按历史 Lease 读取既有 owner 事实，不统一重算任何既有对象摘要，也不新增 current authority、writer、
账本、状态机、表、migration 或经济效果。

当前状态逐字为：

```text
federation_core_historical_causal_reference_carrier_abi=design_frozen
carrier_profiles=execution_source_v1/settlement_source_v1
domain_implementation=source_written
store_resolver=source_written
service=source_written
http_mcp=source_written
pc_client=source_written
migration/table=none
verification=source_review_only
compiled=0 run=0
passed=0 failed=0
native_digest_rewrites=0
state_or_money_effects=0
f0_exit_gate=not_met
```

本页不表示 F0 退出门槛已经完成。F0 顺序和退出条件仍见
[implementation roadmap](implementation-roadmap.md)，总体领域边界见 [architecture](architecture.md)，设计验收见
[对应 acceptance](federation-historical-causal-reference-abi-acceptance.md)。

## 2. 为什么只能新增 wrapper digest

既有 owner 的摘要合同不相同，且全部继续是唯一真源：

- Provider 摘要是 Store 保存的 Provider JSON 原始字节 SHA-256，不在 `ComputeProvider` 正文内；
- Offer 把 self digest 清空后按既有 serde JSON 计算；Job、Reservation 与 Lease 的 revision/digest 位于 Store
  receipt 或投影外层；
- Capacity Claim、Execution Receipt 与 Settlement Receipt 又分别使用不同 payload/purpose/self-blank 规则；
- Pool/SKU 的现有校验强度也不同，不能因出现一个 digest 字符串就宣称它们共享 content-addressed owner。

因此所有下层 `*_digest` 在本 ABI 中都是 **owner-native opaque exact value**。Carrier owner 只能调用所属 owner 的
historical audit 后，把 native value逐字投影到 §4 冻结的 role-specific key；禁止 trim、大小写折叠、重算、换算法、
猜字段映射，或用本页 JCS digest 冒充任一 legacy digest。Carrier key与 native field名称不同时，唯一映射必须以 §4
等式为准，不能把“值逐字相等”误读成“字段名也相同”。

Carrier digest 只保护“这组引用及其角色”本身。它不是签名、receipt、Verification、currentness、replay permit、
dispatch authority、settlement authorization 或付款证明。

## 3. Canonical envelope

常量逐字固定：

```text
SCHEMA=compute_federation.core_historical_causal_reference.v1
DIGEST_DOMAIN=ELON-COMPUTE-CORE-HISTORICAL-LINEAGE-V1
CANONICALIZATION=rfc8785_jcs
DIGEST_ALGORITHM=sha256
MAX_JSON_BYTES=262144
```

顶层必须是 exact 6 keys：

```text
schema
lineage_kind
lineage_digest
canonicalization
digest_algorithm
lineage
```

本页两个基础入口的 `lineage_kind` 必须是 `execution_source_v1` 或 `settlement_source_v1`；不得为 null、unknown
或 caller-defined。复用同一 envelope/domain 的 endpoint-only additive `settlement_release_source_v1` 只由独立
[release authority](federation-settlement-release-causal-reference-abi-authority.md) 冻结；基础入口永不返回该 kind，
旧 Carrier bytes/digest零变化。
`lineage` 的 exact shape 由 kind 决定，禁止用 null、空对象、extra key 或 generic `causes[]` 表示另一 profile。
顶层 `schema/lineage_kind/lineage_digest/canonicalization/digest_algorithm` 必须是 JSON string，`lineage` 必须是
object；primitive 内全部 `*_id/*_digest` 是 non-null JSON string，revision/version/epoch/fencing 是 JSON integer，
`execution_lineage_digest` 也是 string。不得用数字字符串、JSON number ID、array、boolean或隐式默认值替代。

摘要 projection 保留全部 key，仅把 `lineage_digest` 设为空字符串：

```text
SHA256(
  DIGEST_DOMAIN UTF-8 || 0x00 || RFC8785-JCS(envelope_with_lineage_digest_empty) UTF-8
)
```

结果写回 64 位 lowercase hex `lineage_digest`，最终完整 JSON 再做 JCS；parse 后重新 JCS 必须与输入 UTF-8
bytes 逐字相等。拒绝 duplicate/unknown/missing key、float、非 I-JSON、trailing bytes、非 canonical whitespace、
删除 self-digest key或超过上限。所有 revision/version/epoch/fencing 数值必须是 `1..=2^53-1` JSON integer；超界
历史值不得有损转换，必须失败关闭并等待新 ABI。字符串不做 Unicode normalize、trim 或 case fold。

`lineage_id`、status、actor、time、current、replayed、effects 和 owner account 全部禁止出现；V1 没有 nullable 字段。

## 4. Exact historical reference primitives

这些 primitive 只表达 retained identity，不自行证明 owner row 存在或仍 current：

| primitive | exact keys | 唯一 source |
|---|---|---|
| `ProviderVersionRef` | `provider_id,policy_revision,provider_digest` | v169 Provider historical owner |
| `CapacityPoolVersionRef` | `pool_id,capacity_epoch,pool_revision,pool_digest` | v165 immutable Pool version；epoch 不得折叠进 revision |
| `OfferVersionRef` | `provider_id,offer_id,offer_version,offer_digest` | v170 Offer historical owner |
| `PriceSnapshotRef` | `price_snapshot_id,price_snapshot_digest` | v171 immutable Snapshot owner |
| `JobVersionRef` | `job_id,job_revision,job_digest` | v172 Job historical owner |
| `ReservationVersionRef` | `reservation_id,reservation_revision,reservation_digest` | v174 Reservation historical owner |
| `CapacityClaimVersionRef` | `claim_id,claim_revision,claim_digest` | v165-v173 Claim history owner |
| `AttemptLeaseSourceRef` | `lease_id,lease_revision,lease_digest,fencing_generation` | v189/v192 retained source Lease evidence |
| `ExecutionReceiptRef` | `execution_receipt_id,execution_receipt_digest` | v193 immutable Execution Receipt |
| `FinalizationRef` | `finalization_id,finalization_event_digest` | v194 trusted finalization receipt |
| `AttemptSettlementRef` | `settlement_receipt_id,settlement_receipt_digest,settlement_event_digest` | v195 inner Settlement Receipt + outer event |

每个 object 必须 exact 采用表中 keys；不得给 revision-less object 发明 revision，也不得把 ID-only row补成不存在的
digest。`AttemptLeaseSourceRef` 固定使用 v189 terminal candidate 的 `source_lease_revision/source_lease_digest`；v192
必须逐字相同，v194 finalization source Lease也必须相同。读取时 current Lease或 v194 resulting terminal Lease都不能
替代它。

role-specific key与 native source逐字段固定为：

```text
PriceSnapshotRef.price_snapshot_id = ComputePriceSnapshot.snapshot_id
PriceSnapshotRef.price_snapshot_digest = ComputePriceSnapshot.snapshot_digest
AttemptLeaseSourceRef.lease_revision = v189.source_lease_revision
AttemptLeaseSourceRef.lease_digest = v189.source_lease_digest
ExecutionReceiptRef.execution_receipt_id = v193 ComputeExecutionReceipt.receipt_id
ExecutionReceiptRef.execution_receipt_digest = v193 ComputeExecutionReceipt.receipt_digest
FinalizationRef.finalization_event_digest = v194 ComputeAttemptFinalizationReceipt.event_digest
AttemptSettlementRef.settlement_receipt_id = v195 inner ComputeSettlementReceipt.settlement_receipt_id
AttemptSettlementRef.settlement_receipt_digest = v195 inner ComputeSettlementReceipt.settlement_receipt_digest
AttemptSettlementRef.settlement_event_digest = v195 outer ComputeAttemptSettlementReceipt.event_digest
```

其余同名字段也必须逐字投影；不得因同名而跳过 owner audit。

SKU V1 只有嵌在 exact Offer/Price Snapshot 中的 `{schema,sku_id,sku_digest}` qualifier，没有独立 Registry、
revision 或通用 retained resolver，因此不是本页一等 historical ref。Offer/Snapshot owner仍须审计其 SKU 一致性，
但 Carrier不得把 SKU描述成独立 current/content-addressed authority。

## 5. `execution_source_v1` profile

`lineage` exact 9 keys，全部为 object 且顺序只由 JCS 决定：

```text
execution_receipt
provider
capacity_pool
offer
price_snapshot
job
reservation
capacity_claim
attempt_lease_source
```

字段分别采用 §4 对应 primitive。只有 exact v193 Execution Receipt 存在且 v189-v193 owner audit全部通过时才可
构造；不能只解析 `ComputeExecutionReceipt` JSON，因为该 JSON 没保存 Job/Reservation/Claim/Lease 的 revision/digest。

九个角色的唯一 source map 是：

```text
execution_receipt    <- v193 receipt self pair
provider             <- exact historical Offer registration metadata
capacity_pool        <- exact Offer.capacity_pool; must equal Claim.pool
offer                <- Reservation.offer = Job.selected_offer = Snapshot offer tuple
price_snapshot       <- Reservation.price_snapshot self pair
job                  <- v189/v192 Job tuple
reservation          <- v189/v192 Reservation tuple
capacity_claim       <- v189/v192 Claim tuple = Reservation.capacity_claim
attempt_lease_source <- v189 source Lease tuple + fencing_generation
```

Store resolver必须逐字证明：

1. historical Offer 的 Provider ID/revision/digest命中同一 Provider ref，Offer 的 Pool命中同一 Pool ref；
2. Snapshot 的 Provider、Offer triple、SKU 与 DeliveryWindow通过 v171 owner audit，并与 exact Offer一致；
3. Job selected Offer与 Snapshot、Reservation 的 Job/Offer/Snapshot/Claim bindings全部相等；
4. Claim是该 Reservation 的 reservation claim，Pool/DeliveryWindow与 Offer/Snapshot一致；
5. v189 与 v192 保存同一 Job/Reservation/Claim historical refs与同一 source Lease ref；v193 issuance/readback
   owner audit必须解析回并重证同一组 refs，不得声称这些 revision/digest存在于 v193 JSON；
6. Execution Receipt 的 Job、Reservation、Lease、attempt/fencing、Provider/executor 与 Offer triple逐字回指同一链；
7. v188-v192 evidence、v185 activation及 v193 receipt digest仍由各自 owner完整重算通过。

Carrier不复制完整 Workload、SKU、Pool body、usage、attestation、Verification、artifact、Plan、route、capability、
budget或账户。上述事实继续由 native owner证明；Carrier只携带本页冻结的闭合引用集合。

## 6. `settlement_source_v1` profile

`lineage` exact 9 keys：

```text
attempt_settlement
execution_receipt
execution_lineage_digest
finalization
price_snapshot
provider
source_job
terminal_job
terminal_reservation
```

其中 `attempt_settlement`、`execution_receipt`、`finalization`、`price_snapshot`、`provider`、两份 Job ref与
Reservation ref分别采用 §4 shape；`execution_lineage_digest` 是重新生成 exact `execution_source_v1` 后得到的本页
64 lowercase digest，不从数据库、caller或缓存照抄。

Store resolver必须逐字证明：

1. v195 inner Settlement Receipt ID/digest与 outer settlement event同一行并各自 owner-audit通过；
2. v195 引用的 v193 pair与重新生成的 `execution_source_v1` subject相等，且 digest等于
   `execution_lineage_digest`；
3. finalization ID/event与 Execution Receipt逐字一致；`source_job = v194.terminal_job = v195.source_job`，
   `terminal_job = v195.terminal_job`，`terminal_reservation = v194.terminal_reservation`；不得误取 v194 的 running
   source Job或 active source Reservation；
4. source Job是 v194 target/v195 `verification_pending` source，terminal Job是 v195 `settled` target；两者 ID
   相同、revision连续性和 native digest由 Job owner审计；
5. terminal Reservation只表示 v194 的 resulting terminal version，不能替换 execution carrier中的 v193 source
   Reservation/Claim/Lease；
6. Snapshot pair与 Settlement Receipt、v171 source一致；Provider ref逐字段满足：
   `provider.provider_id = v193 receipt.provider_id = exact historical Offer.provider_id`、
   `provider.policy_revision = v195 outer.provider_policy_revision = exact historical Offer metadata.provider_policy_revision`、
   `provider.provider_digest = v195 outer.provider_digest = exact historical Offer metadata.provider_digest`；
7. terminal Reservation逐字段满足：
   `terminal_reservation.reservation_id = v195 inner.reservation_id = v193 receipt.reservation_id`、
   `terminal_reservation.reservation_revision = v194 terminal_reservation.revision`、
   `terminal_reservation.reservation_digest = v194 terminal_reservation.digest`；
8. price calculation、money posting、pending balance与 challenge/release/correction门卫仍由 v195+ owner审计，
   Carrier不重新计算金额。

`pending` internal settlement不能从本 Carrier推导成 available、withdrawn、external paid或链上 finality。挑战、纠正、
释放、提现和外部付款各自仍需独立 receipt/authority；v198释放后的独立只读组合见
[`settlement_release_source_v1`](federation-settlement-release-causal-reference-abi-authority.md)。

## 7. Current 与 historical 必须分离

两个 profile都只能从 exact retained owner rows确定性生成。Canonical parser最多返回“bytes/digest自洽”的 untrusted
DTO；它不能产生 `ValidatedFederationHistoricalLineage`，也不能进入任何 writer。

未来 Store resolver必须：

- 按 carrier中每个 exact ID+revision/version+digest查找 retained source；0、multi、digest drift、JSON/column drift
  全部 integrity failure；
- 对 source Lease沿 v189/v192/v194 retained evidence解析，禁止假设存在通用 `lease_versions` 表；
- 禁止使用 current/latest fallback、猜 revision、把 current head改名为 historical，或因 source已过期/撤销而改写历史；
- 只返回 private-field、non-Clone、non-Serde 的 sealed validated view；raw DTO、HTTP body和 caller不能构造；
- 让任何 fresh action继续在自身事务内执行原有 currentness、authorization、TTL与CAS检查。Carrier跨 transaction/await
  后不保留 current authority。

同一 source每次生成必须得到逐字相同 JSON/digest；这只是 deterministic read，不登记 replay、行、幂等键或副作用。

## 8. 兼容与组合边界

- 不 backfill、update或重新摘要 v169-v195 任一 JSON/row；不向既有 receipt/Plan/command envelope塞入 Carrier digest。
- 旧 `NodeComputeRun` 缺少 exact Provider/Offer/Job/Reservation/Claim/Lease/Receipt 链时继续报告 partial，禁止伪造。
- 三类 Provider共用同一 core reference shape；user-node Ready/V279 roots、managed-cluster endpoint evidence、V280
  admission/route/wire roots只能由各自 owner在组合层另行审计，不得进入或改写本 V1。
- [v212 Plan](attempt-execution-plan-v1.md)、[v193 Execution Receipt](attempt-execution-receipt-api.md)、
  [v194 finalization](attempt-finalization-api.md) 和 [v195 Settlement Receipt](attempt-settlement-api.md) 仍是原业务
  权威；Carrier不替代 Plan seal、Verification、fencing、账本 posting或 receipt digest。
- [V279 UserNode binding](user-node-provider-binding-authority.md) 与
  [V280 service-managed admission/Runner](external-pool-service-managed-admission-runner-authority.md) 继续由各自
  owner证明 kind-specific roots；本 Carrier不吸收其 current authority。
- 不预留 migration编号，不新建 unified current-head table、universal resolver table、第二本账或通用 causal graph。

## 9. Additive read adoption

### 9.1 Historical by-Lease root 与私有 scope

公开读入只接受 `lease_id`，且必须先通过 Lease ID shape gate；禁止让 caller 另传
`attempt_id/provider_id/project_id/consumer_account_id/execution_receipt_id/settlement_receipt_id`、revision、digest、
kind 或 currentness 提示。Store 的两个唯一 by-Lease facade 固定为：

```text
resolve_compute_execution_source_lineage_for_lease(lease_id)
resolve_compute_settlement_source_lineage_for_lease(lease_id)
```

execution resolver 必须从该 Lease 的 retained v189 terminal candidate 出发，闭合 v189-v193 并返回 exact
`execution_source_v1`；settlement resolver 必须从同一 source Lease 追溯唯一 v193→v194→v195 根，重建
execution carrier 后再返回 exact `settlement_source_v1`。必须逐字证明
`rebuilt_v193.attempt_lease_id == v194.lease_id == v195.lease_id`；0、multi、任一 owner/native digest/JSON/column
drift 或 source Lease 不同均失败关闭；不存在 attempt/current/latest 或 receipt-ID fallback。

resolver 成功后才可从已重审的 owner 对象封存 private-field、non-Clone、non-Serde access scope，
唯一等式为：

```text
scope.consumer_account_id
  = historical execution-source Job.job.consumer_account_id
scope.project_id
  = historical execution-source Job.job.project_id
scope.provider_owner_account_id
  = historical Provider.provider.owner_account_id

settlement.scope = rebuilt execution.scope
settlement.source_job.consumer_account_id = settlement.terminal_job.consumer_account_id
settlement.source_job.project_id = settlement.terminal_job.project_id
```

settlement 还必须证明上述 source/terminal Job 的 consumer/project 与 rebuilt execution scope 全部相等。
scope 不进入 Carrier、response、Debug、serde 或 client，也不跨 transaction/await 保留 authorization。

### 9.2 Read response exact-5 ABI

成功响应必须 exact 5 keys，无 null、extra key、嵌套 scope 或摊平 Carrier：

```text
schema
lineage_kind
lineage_digest
canonical_carrier_json
read_effect
```

常量和等式逐字固定：

```text
READ_SCHEMA=compute_federation.core_historical_causal_reference.read.v1
READ_EFFECT=none
response.lineage_kind = parsed canonical_carrier_json.lineage_kind
response.lineage_digest = parsed canonical_carrier_json.lineage_digest
```

5 个值全部是 JSON string。`canonical_carrier_json` 必须是 Store sealed view 返回的完整规范 JCS
字符串，不得 parse 后重排、摘录、替换 digest 或把 `lineage` 展开到 response。响应不得增加
scope、actor、identity、time、current、replay、status、authorization、owner 或 effect 字段；唯一效果声明是
`read_effect="none"`。

### 9.3 Service、HTTP 与 MCP

Service 只暴露四个 read facade：

```text
read_execution_for_participant
read_settlement_for_participant
read_execution_for_admin
read_settlement_for_admin
```

participant 成功 predicate 精确为：

```text
user_id == scope.consumer_account_id
  || user_id == scope.provider_owner_account_id
```

HTTP 只新增四个 `GET`，路径参数只有 `lease_id`，不接受 query/body：

```text
/api/me/compute/attempt-leases/:lease_id/execution-source-lineage
/api/me/compute/attempt-leases/:lease_id/settlement-source-lineage
/api/admin/compute/attempt-leases/:lease_id/execution-source-lineage
/api/admin/compute/attempt-leases/:lease_id/settlement-source-lineage
```

MCP 只新增四个工具；input schema 必须是 properties 只有 string `lease_id` 的 exact object，
`required=["lease_id"]`、`additionalProperties=false`：

```text
compute_get_my_execution_source_lineage
compute_get_my_settlement_source_lineage
compute_admin_get_execution_source_lineage
compute_admin_get_settlement_source_lineage
```

普通 MCP 在 participant predicate 成功后还必须满足
`scope.project_id == Some(caller_project_id)`；无 current project 或 project 不等都拒绝。HTTP `/api/me`
不从 header/query 伪造 project scope。Admin HTTP 依赖已有 platform-admin role gate；admin MCP 先继承现有
project-scoped MCP transport 的 membership gate，再执行 platform-admin role gate。Project membership只授予到达
该 MCP transport 的资格，不能单独授予 admin lineage authority。这些入口都只读，不调用 Carrier parser 来构造权限，不审计成
replay，不更新 last-seen/current head。

### 9.4 Participant/admin 失败与脱敏

错误顺序和对外结果固定为：

| caller / condition | code | HTTP | 对外边界 |
|---|---|---:|---|
| unauthenticated / any request | `FEDERATION_LINEAGE_UNAUTHENTICATED` | 401 | 不解析 Lease 或 owner |
| authenticated HTTP caller after route role gate / query 或非空 body | `FEDERATION_LINEAGE_INVALID_REQUEST_INPUT` | 400 | 不查 owner |
| authenticated caller after route role gate / invalid Lease ID shape | `FEDERATION_LINEAGE_INVALID_LEASE_ID` | 400 | 不查 owner |
| participant / missing、owner drift、scope 未形成 | `FEDERATION_LINEAGE_NOT_VISIBLE` | 404 | 不区分缺失与 integrity |
| participant / scope 成功但用户非 consumer/provider owner | `FEDERATION_LINEAGE_NOT_VISIBLE` | 404 | 防枚举 |
| ordinary MCP / participant 成功但 project 缺失或不等 | `FEDERATION_LINEAGE_PROJECT_FORBIDDEN` | —（JSON-RPC tool error） | 不返回期待 project |
| admin MCP / project membership gate失败 | 既有 MCP project-access error | 403 | lineage tool/Service 未执行 |
| admin / role 非 platform admin | `FEDERATION_LINEAGE_ADMIN_FORBIDDEN` | 403 | 不解析成功 payload |
| admin / historical root missing | `FEDERATION_LINEAGE_NOT_FOUND` | 404 | 不返回 scope/owner |
| admin / owner/native digest/JSON/column drift | `FEDERATION_LINEAGE_INTEGRITY_CONFLICT` | 409 | 只返回稳定 code |

除 ordinary MCP 在 participant predicate 成功后的 project tool error 外，participant 路径不得通过不同
status/code/detail 暴露 Lease 是否存在、归属哪个 Provider/project、哪个 owner 发生 drift 或哪层 receipt 失配。
Admin 的 409 也不回显 row、native JSON、digest期望/实际值、account/project/provider/receipt ID 或 SQL。
所有成功路径都只返回 exact-5 response。表中数字 HTTP status 只适用于四个 GET 或 MCP transport 的认证/project
前置拒绝；进入 `tools/call` 后的 MCP failure以 JSON-RPC tool error承载同一稳定 lineage code，不伪装成 HTTP
403/404/409。

### 9.5 PC runtime validation 与双响应闭合

PC 不信任 HTTP/MCP adapter 已正确处理 Carrier。每个响应在显示前必须在 runtime：

1. 验 exact-5 keys、全 string type、`READ_SCHEMA`、`READ_EFFECT`与 kind 白名单；
2. 将 `canonical_carrier_json` 按 UTF-8 字节执行 strict parse，再做 RFC 8785 JCS byte-equal；
3. 按 §3 把 inner `lineage_digest` 置空，重算 `DIGEST_DOMAIN || 0x00 || JCS` SHA-256，并证明重算值等于
   inner `lineage_digest`；
4. 证明 inner schema/canonicalization/digest-algorithm 常量正确，inner kind/digest 与 response 的
   `lineage_kind/lineage_digest` 逐字相等，且 inner shape 精确命中对应 profile；任一失配整个卡片失败关闭，
   不展示部分链。

同一笔 settlement 历史展示必须并行读取该 Lease 的 execution 与 settlement response，分别证明
`execution_source_v1` 与 `settlement_source_v1` exact kind，再证明
`settlement carrier.lineage.execution_lineage_digest == execution response.lineage_digest`。任一 endpoint 缺失、
kind 错误、JCS/domain digest 失配或跨响应等式不成立，client adoption 就不成立。PC 不持久 Carrier、
不刷新 authority、不生成链接/下载或结算动作。Lease、scope 或 Provider 选择变化必须立即使在途 generation 失效并
清空旧证据；旧响应不得写回新的 subject/scope 页面。

### 9.6 零 writer、migration 与 effect

本 adoption 只组装 read response：Store 只使用 Deferred/read transaction，Service/HTTP/MCP/PC 都没有
create/update/delete/CAS/replay/audit/last-seen writer。不新增或修改 migration、table、view、index、trigger、UDF，
不回填 Carrier，不更改 v169-v195 owner JSON/digest。`read_effect="none"` 是响应合同而不是一条
持久 effect row。结果不产生 current authority、Ready、route、Offer、Job、Lease、Receipt、posting、balance、
release、withdrawal、external payment 或链上效果。

## 10. 计划 ownership 与 source-written 门

source实现独占在新的 Provider-neutral Domain owner：
`server/src/compute_federation/federation_historical_causal_reference/`，只拥有 exact DTO、JCS与 Carrier digest。
各 native digest/helper继续属于现有 Provider/Pool/Offer/Snapshot/Job/Reservation/Claim/Attempt/Receipt owner。

Store integration只新增只读 historical resolver；不复制 SQL/digest公式或提供 public raw builder。本批已写入 Domain、
Store resolver、source-contract、golden与negative test，以及本页冻结的 Service、四 HTTP、四 MCP 与 PC client
采用源码；没有创建表或 migration，也未编译、测试或执行 SQLite/runtime/network。

本次 `source_written` 静态审查中以下五类源码同时存在，但均未编译或运行，不能写成 verified：

1. 两种 profile exact DTO/canonical/from-json golden；
2. owner-by-owner full audit与 source Lease retained resolver；
3. v193/v195 deterministic reconstruction及跨对象 splice negatives；
4. raw DTO不能进入 writer的source contract；
5. resolver、Service、HTTP/MCP、PC 无 writer/migration入口，legacy digest bytes、既有表与既有 receipt
   JSON/digest合同零改写，且本批状态或资金效果为零。

## 11. 明确禁止

- 禁止把 Carrier digest写成 Provider/Offer/Job/Lease/Receipt native digest；
- 禁止用 carrier canonical通过代替 historical row、签名、currentness或 actor authorization；
- 禁止把 v194/v195 terminal refs冒充 v193 execution-source refs；
- 禁止把 SKU qualifier、Pool digest或 ID-only事实升级成未经 owner证明的 authority；
- 禁止 generic `kind + arbitrary causes[]`、nullable optional soup、unknown extension或 caller-supplied role；
- 禁止因本页 `design_frozen` 宣称 F0、真实 Attempt、结算、V279 Ready或V280市场/Runner已完成。
