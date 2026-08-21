---
title: 联邦核心历史因果引用 Carrier ABI 权威
reviewed_at: 2026-08-21
status: current
owners: backend, node, ai-economy
design_status: design_frozen
design_scope: federation_core_historical_causal_reference_carrier_abi_v1
implementation_status: implementation_unwired
verification_status: design_review_only
---

# 联邦核心历史因果引用 Carrier ABI 权威

## 1. 唯一结论与状态边界

本页只冻结 Provider-neutral 的历史引用 primitive，以及 Execution Receipt 与 Settlement Receipt 两种只读因果
Carrier。它让 Store、Service、HTTP/MCP 和客户端未来可以传递同一组 owner-audited identity/version/digest，
但不统一重算任何既有对象摘要，不新增 current authority、writer、账本、状态机、表、migration、API 或经济效果。

当前状态逐字为：

```text
federation_core_historical_causal_reference_carrier_abi=design_frozen
carrier_profiles=execution_source_v1/settlement_source_v1
domain_implementation=absent
store_resolver=absent
service_http_mcp_client_adoption=absent
implementation=unwired/uncompiled/unrun
passed=0 failed=0
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

`lineage_kind` 必须是 `execution_source_v1` 或 `settlement_source_v1`；不得为 null、unknown 或 caller-defined。
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
释放、提现和外部付款各自仍需独立 receipt/authority。

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

## 9. 计划 ownership 与 source-written 门

未来 source实现须独占在新的 Provider-neutral Domain owner，例如
`server/src/compute_federation/federation_historical_causal_reference/`，只拥有 exact DTO、JCS与 Carrier digest。
各 native digest/helper继续属于现有 Provider/Pool/Offer/Snapshot/Job/Reservation/Claim/Attempt/Receipt owner。

Store integration只能新增只读 historical resolver；不得复制 SQL/digest公式或提供 public raw builder。聚合模块、Service、
HTTP/MCP/client adoption、source-contract、golden与negative tests须在后续独立 implementation batch登记。本批没有创建这些
目录、符号或 caller，也未编译、测试、执行 migration/SQLite/runtime/network。

任何 source-written 声明前至少必须同时具备：

1. 两种 profile exact DTO/canonical/from-json golden；
2. owner-by-owner full audit与 source Lease retained resolver；
3. v193/v195 deterministic reconstruction及跨对象 splice negatives；
4. raw DTO不能进入 writer的source contract；
5. legacy digest bytes与既有表/receipt零 diff，且全部经济效果为零。

## 10. 明确禁止

- 禁止把 Carrier digest写成 Provider/Offer/Job/Lease/Receipt native digest；
- 禁止用 carrier canonical通过代替 historical row、签名、currentness或 actor authorization；
- 禁止把 v194/v195 terminal refs冒充 v193 execution-source refs；
- 禁止把 SKU qualifier、Pool digest或 ID-only事实升级成未经 owner证明的 authority；
- 禁止 generic `kind + arbitrary causes[]`、nullable optional soup、unknown extension或 caller-supplied role；
- 禁止因本页 `design_frozen` 宣称 F0、真实 Attempt、结算、V279 Ready或V280市场/Runner已完成。
