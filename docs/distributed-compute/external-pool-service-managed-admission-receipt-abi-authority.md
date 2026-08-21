---
title: 外部矿池 service-managed admission receipt canonical 与 physical ABI 权威
reviewed_at: 2026-08-21
status: current
owners: backend, security, ai-economy
design_status: design_frozen
design_scope: admission_receipt_canonical_and_physical_schema_abi_v1
implementation_status: implementation_unwired
verification_status: design_review_only
---

# 外部矿池 service-managed admission receipt canonical 与 physical ABI 权威

## 1. 唯一结论与状态边界

本页只冻结 V280 admission receipt 的 canonical ABI、planned SQLite 单表 schema 与 historical/current readback 边界。
它不创建表、不注册 migration、不打开 V254 fence，也不实现 market writer、Gateway、validator 或 Runner。

当前状态向量固定为：

```text
admission_receipt_canonical_abi=design_frozen
admission_receipt_physical_schema_abi=design_frozen
market_projection_identity_abi=design_frozen
admission_receipt_table=absent
physical_migration_registration=absent
migration_registry_max=279
source=absent
implementation=unwired/uncompiled/unrun
passed=0
failed=0
```

Profile schema 由 [market profile authority](external-pool-service-managed-market-profile-authority.md) 维护；完整事务、Gateway、
task session 与恢复见 [V280 父权威](external-pool-service-managed-admission-runner-authority.md)。本页的验收见
[admission receipt ABI acceptance](external-pool-service-managed-admission-receipt-abi-acceptance.md)；receipt引用的legacy
projection identity由 [market projection identity ABI](external-pool-service-managed-market-projection-identity-abi-authority.md)维护。
`V280`只是阶段名，不预留migration编号；未来实现必须重读registry并使用届时next free物理编号。

## 2. 固定常量与 domain

```text
ADMISSION_RECEIPT_SCHEMA=compute_federation.external_pool_service_managed_admission_receipt.v1
ADMISSION_CANONICALIZATION=rfc8785_jcs
ADMISSION_DIGEST_ALGORITHM=sha256
ADMISSION_MAX_JSON_BYTES=4194304
ADMISSION_CONFIRMATION=confirm_external_pool_service_managed_admission
ADMISSION_ACTOR_KIND=platform_market_service
ADMISSION_ACTOR_ID=external_pool_service_managed_market
ADMISSION_IDEMPOTENCY_SCOPE=external_pool_service_managed_admission
ADMISSION_ID_PREFIX=external_pool_service_managed_admission_v1_
ADMISSION_ID_DOMAIN=ELON-EXTERNAL-POOL-SERVICE-MANAGED-ADMISSION-ID-V1
ADMISSION_REQUEST_DOMAIN=ELON-EXTERNAL-POOL-SERVICE-MANAGED-ADMISSION-REQUEST-V1
ADMISSION_INTEGRITY_DOMAIN=ELON-EXTERNAL-POOL-SERVICE-MANAGED-ADMISSION-INTEGRITY-V1
ADMISSION_RECEIPT_DOMAIN=ELON-EXTERNAL-POOL-SERVICE-MANAGED-ADMISSION-RECEIPT-V1
BUCKET_INVENTORY_DOMAIN=ELON-EXTERNAL-POOL-SERVICE-MANAGED-BUCKET-INVENTORY-V1
```

本页新定义的 ID/request/integrity/receipt/bucket-inventory digest 全部使用：

```text
SHA256(domain UTF-8 || 0x00 || RFC8785-JCS(value) UTF-8)
```

Profile、V249/V274/V277/V278、Pool、Offer、publication 与 v171 的 legacy/external digest 继续由所属 owner
重算，不能改用本页 domain。

## 3. Canonical envelope 与 exact key sets

Top-level exact 6 keys：

```text
schema
admission_receipt_digest
admission_integrity_digest
canonicalization
digest_algorithm
admission
```

`admission` exact 7 keys：

```text
identity
activation
route_runtime
policy
pool
offer_price
timing
```

Nested exact direct keys（7组共72个；`bucket_inventory`数组整体在`pool`中计1个direct key）：

| object | exact keys |
|---|---|
| `identity` | `admission_receipt_id,admission_sequence,predecessor_admission_receipt_id,predecessor_admission_receipt_digest,provider_id,provider_policy_revision,provider_digest,provider_binding_id,provider_binding_digest,market_admission_actor_kind,market_admission_actor_id,confirmation,idempotency_scope,idempotency_key,request_digest` |
| `activation` | `activation_receipt_id,activation_receipt_digest,activation_root_digest,active_successor_receipt_id,active_successor_receipt_digest` |
| `route_runtime` | `route_renewal_receipt_id,route_renewal_receipt_digest,route_authorization_id,route_authorization_revision,route_authorization_digest,executor_id,stable_executor_binding_digest,route_adapter_projection_id,route_adapter_revision,route_adapter_digest,projected_v211_adapter_binding_digest` |
| `policy` | `market_profile_id,market_profile_revision,market_profile_json,market_profile_digest,capacity_scope,capacity_unit,allocation_total_units,capacity_allocation_id,capacity_allocation_digest` |
| `pool` | `pool_id,capacity_epoch,pool_revision,pool_digest,delivery_window_id,delivery_window_digest,bucket_inventory_count,bucket_inventory_digest,bucket_inventory,supply_transaction_id,supply_transaction_digest,supply_ledger_sequence,pool_activation_event_id,pool_activation_event_request_digest` |
| `offer_price` | `offer_id,draft_offer_version,draft_offer_digest,active_offer_version,active_offer_digest,publication_id,publication_digest,price_snapshot_id,price_snapshot_digest,price_quote_id,price_source_kind,price_source_id,price_source_version,price_source_digest` |
| `timing` | `checked_at,valid_from,expires_at,created_at` |

`policy.market_profile_json`的canonical value固定为JSON string：其字符串值逐字等于profile authority生成的完整canonical
profile JSON UTF-8文本；admission JCS中按JSON string规则转义。物理列`market_profile_json`保存解码后的原始canonical profile
JSON TEXT，不保存二次转义文本；read/write必须解码后byte-equal再由profile parser重证，object或重复编码string均拒绝。

所有 DTO 必须 `deny_unknown_fields`。输入先按 UTF-8 bytes 拒绝空值或超过 4 MiB，再要求 I-JSON、无重复 key、
无 float/NaN/Infinity、所有数值型 revision/version/sequence/count 为 `1..=9007199254740991`。Nullable 只允许
两个 predecessor 字段，且 V1 必须同时为 `null`。时间必须是 canonical UTC nanos；digest 必须 lowercase 64-hex；
identifier 必须 trim 后 1..240 bytes且无控制字符。解析后必须 deep validate、重算全部派生值并逐字比较重新生成的 JCS bytes。

## 4. ID、request、integrity 与 receipt preimage

V1 固定 `admission_sequence=1`。ID material exact 为：

```json
{"admission_sequence":1,"provider_id":"..."}
```

`admission_receipt_id=ADMISSION_ID_PREFIX || domain_digest(ADMISSION_ID_DOMAIN, material)`。
`idempotency_scope=ADMISSION_IDEMPOTENCY_SCOPE`，`idempotency_key=admission_receipt_id`。ID、scope、key、actor 与
confirmation 都由 server 常量派生；env、HTTP、Provider owner、Job caller 或 profile payload不得提交。

Request material exact 10 keys：

```text
admission_receipt_id
provider_id
admission_sequence
predecessor_admission_receipt_id
predecessor_admission_receipt_digest
market_admission_actor_kind
market_admission_actor_id
confirmation
idempotency_scope
idempotency_key
```

`request_digest=domain_digest(ADMISSION_REQUEST_DOMAIN, request material)`。它故意排除 V249/current Provider、
V274/V277/V278、profile、Pool、Offer、Snapshot 与时间，使 fresh/replay 可在任何 current-source read 前收敛。

`admission_integrity_digest=domain_digest(ADMISSION_INTEGRITY_DOMAIN, admission)`。Receipt digest 使用完整 envelope，
保留 `admission_receipt_digest` key 但将其值置为空串，再以 `ADMISSION_RECEIPT_DOMAIN` 计算。不得删除 self-digest key、
重复造 material digest、使用 raw serde JSON SHA 或把 integrity digest 当 receipt digest。

## 5. Bucket inventory 与时间公式

V1 `bucket_inventory` fixed为array length=1；唯一元素是下列exact 25-key object：

```text
ordinal,bucket_id,bucket_digest,pool_id,capacity_epoch,pool_revision,
delivery_window_id,delivery_window_digest,delivery_window_starts_at,delivery_window_ends_at,
meter,meter_mode,quantum_units,meter_policy_digest,status,issued_units,available_units,
held_units,active_units,consumed_units,retired_units,balance_revision,
through_ledger_sequence,created_at,updated_at
```

固定关系：

```text
bucket_inventory_count=1
ordinal=1
capacity_epoch=pool_revision=1
meter=attempt_slot
meter_mode=reusable
quantum_units=1
status=open
issued_units=available_units=allocation_total_units
held_units=active_units=consumed_units=retired_units=0
balance_revision=through_ledger_sequence=supply_ledger_sequence=1
delivery_window_starts_at=checked_at
delivery_window_ends_at=profile.inflight_execution_valid_until
bucket.created_at=bucket.updated_at=checked_at
```

`bucket_inventory_digest=domain_digest(BUCKET_INVENTORY_DOMAIN, bucket_inventory)`，preimage是完整单元素array而非裸object。Receipt
JSON保存该数组；planned
table 只把 count/digest 标量投影。Fresh INSERT trigger与precommit readback必须从canonical JSON取唯一bucket并逐项join初始行；
historical/replay只对真实bucket核immutable binding/config，并从immutable supply transaction+两条legs+sequence1重建准入时balance，
不得要求mutable status/balance revision/available/held/active/updated-at仍等于receipt snapshot。
`bucket_id/bucket_digest/resource_scope_digest/meter_policy_digest/delivery_window`、supply/event/Offer/publication/snapshot/quote
identity及所有legacy writer共享同一canonical `checked_at`的派生已由projection identity ABI冻结设计，但仍无writer源码。
本页只冻结其receipt位置与历史readback；price source observation window已由profile authority固定为
`[checked_at-1s,checked_at]`并必须进入snapshot digest。

时间固定：

```text
valid_from=created_at=checked_at
expires_at=price_snapshot.expires_at
price_snapshot.expires_at=min(checked_at+profile.quote_ttl_seconds,
                              profile.new_plan_accept_until,
                              profile.expires_at)
checked_at < expires_at < active_offer.valid_until
```

Current read必须使用server `read_at`满足`valid_from<=read_at<expires_at`，并另行重证catalog、active Provider/V249、V277
stable activation/executor/adapter lineage、Pool、Offer与Snapshot live roots；V274/V278只审计准入时historical pair，不要求
当前head相等。Stored `checked_at<expires_at` 只是fresh不变量，不能冒充动态currentness。过期不修改历史receipt。Tx-A后已seal Plan
的恢复仍以Plan自身hard deadline为准，不被admission/profile new-plan expiry回溯撤销。

## 6. Planned 77-column physical schema

唯一 planned table 名为 `compute_external_pool_service_managed_admissions WITHOUT ROWID`。列序是 ABI；Domain getters、
DDL、INSERT params、SELECT constant、row indices 与 source-contract 必须逐字一致：

```text
01 admission_receipt_id
02 admission_receipt_schema
03 admission_receipt_digest
04 admission_receipt_json
05 admission_integrity_digest
06 canonicalization
07 digest_algorithm
08 admission_sequence
09 predecessor_admission_receipt_id
10 predecessor_admission_receipt_digest
11 provider_id
12 provider_policy_revision
13 provider_digest
14 provider_binding_id
15 provider_binding_digest
16 market_admission_actor_kind
17 market_admission_actor_id
18 confirmation
19 idempotency_scope
20 idempotency_key
21 request_digest
22 activation_receipt_id
23 activation_receipt_digest
24 activation_root_digest
25 active_successor_receipt_id
26 active_successor_receipt_digest
27 route_renewal_receipt_id
28 route_renewal_receipt_digest
29 route_authorization_id
30 route_authorization_revision
31 route_authorization_digest
32 executor_id
33 stable_executor_binding_digest
34 route_adapter_projection_id
35 route_adapter_revision
36 route_adapter_digest
37 projected_v211_adapter_binding_digest
38 market_profile_id
39 market_profile_revision
40 market_profile_json
41 market_profile_digest
42 capacity_scope
43 capacity_unit
44 allocation_total_units
45 capacity_allocation_id
46 capacity_allocation_digest
47 pool_id
48 capacity_epoch
49 pool_revision
50 pool_digest
51 delivery_window_id
52 delivery_window_digest
53 bucket_inventory_count
54 bucket_inventory_digest
55 supply_transaction_id
56 supply_transaction_digest
57 supply_ledger_sequence
58 pool_activation_event_id
59 pool_activation_event_request_digest
60 offer_id
61 draft_offer_version
62 draft_offer_digest
63 active_offer_version
64 active_offer_digest
65 publication_id
66 publication_digest
67 price_snapshot_id
68 price_snapshot_digest
69 price_quote_id
70 price_source_kind
71 price_source_id
72 price_source_version
73 price_source_digest
74 checked_at
75 valid_from
76 expires_at
77 created_at
```

Text IDs/digests/times、INTEGER revision/sequence/count/total必须用 SQLite `typeof` 与 exact shape CHECK；
`admission_receipt_json`必须`json_valid`、object、UTF-8 blob length `<=4194304`，`market_profile_json`必须`json_valid`、
object、UTF-8 blob length `<=1048576`。固定值与关系由 CHECK 锁定：schema/JCS/algorithm、sequence1/null predecessor、actor、
confirmation/scope/key、capacity scope/unit、epoch/revisions、draft1/active2、bucket count1、supply sequence1、fallback source、
time equality/order。`admission_receipt_id`为 PK；UPDATE/DELETE/REPLACE/backfill/seed均由原名 immutable guards拒绝。

## 7. Exact parent keys、FK、UNIQUE 与 indexes

SQLite composite FK 的 parent key 必须 exact UNIQUE。V277 triple、V274 pair、V278 pair 已存在；下列10组required
parent keys中，`compute_offer_publications(publication_id,publication_digest)`必须复用既有
`ux_compute_offer_publications_exact`，其余9组由未来service-managed admission物理migration（使用届时next free编号）在建child
table前按下列固定name/tuple添加exact UNIQUE index；不能重复建
publication index，也不能用两个独立FK冒充一对证据：

```text
ux_compute_provider_versions_v280_exact -> compute_provider_versions(provider_id,policy_revision,provider_digest)
ux_compute_external_pool_adapter_registry_provider_bindings_v280_exact -> compute_external_pool_adapter_registry_provider_bindings(provider_binding_id,provider_binding_digest)
ux_compute_route_authorization_receipts_v280_exact -> compute_route_authorization_receipts(route_authorization_id,route_authorization_revision,route_authorization_digest)
ux_compute_route_adapter_versions_v280_exact -> compute_route_adapter_versions(adapter_id,adapter_revision,adapter_digest)
ux_compute_capacity_pool_versions_v280_exact -> compute_capacity_pool_versions(pool_id,capacity_epoch,pool_revision,pool_digest)
ux_compute_capacity_ledger_transactions_v280_exact -> compute_capacity_ledger_transactions(transaction_id,transaction_digest,ledger_sequence)
ux_compute_capacity_pool_lifecycle_events_v280_exact -> compute_capacity_pool_lifecycle_events(event_id,request_digest)
ux_compute_offer_versions_v280_exact -> compute_offer_versions(offer_id,offer_version,offer_digest)
ux_compute_offer_publications_exact -> compute_offer_publications(publication_id,publication_digest) [existing]
ux_compute_price_snapshots_v280_exact -> compute_price_snapshots(snapshot_id,snapshot_digest)
```

Child FK exact mapping固定为：

```text
(provider_id,provider_policy_revision,provider_digest) -> compute_provider_versions(provider_id,policy_revision,provider_digest)
(provider_binding_id,provider_binding_digest) -> compute_external_pool_adapter_registry_provider_bindings(provider_binding_id,provider_binding_digest)
(activation_receipt_id,activation_receipt_digest,activation_root_digest) -> compute_external_pool_adapter_atomic_activation_receipts(activation_receipt_id,activation_receipt_digest,activation_root_digest)
(active_successor_receipt_id,active_successor_receipt_digest) -> compute_external_pool_adapter_provider_active_successor_receipts(active_successor_receipt_id,receipt_digest)
(route_renewal_receipt_id,route_renewal_receipt_digest) -> compute_external_pool_adapter_route_renewal_receipts(route_renewal_receipt_id,route_renewal_receipt_digest)
(route_authorization_id,route_authorization_revision,route_authorization_digest) -> compute_route_authorization_receipts(route_authorization_id,route_authorization_revision,route_authorization_digest)
(route_adapter_projection_id,route_adapter_revision,route_adapter_digest) -> compute_route_adapter_versions(adapter_id,adapter_revision,adapter_digest)
(pool_id,capacity_epoch,pool_revision,pool_digest) -> compute_capacity_pool_versions(pool_id,capacity_epoch,pool_revision,pool_digest)
(supply_transaction_id,supply_transaction_digest,supply_ledger_sequence) -> compute_capacity_ledger_transactions(transaction_id,transaction_digest,ledger_sequence)
(pool_activation_event_id,pool_activation_event_request_digest) -> compute_capacity_pool_lifecycle_events(event_id,request_digest)
(offer_id,draft_offer_version,draft_offer_digest) -> compute_offer_versions(offer_id,offer_version,offer_digest)
(offer_id,active_offer_version,active_offer_digest) -> compute_offer_versions(offer_id,offer_version,offer_digest)
(publication_id,publication_digest) -> compute_offer_publications(publication_id,publication_digest)
(price_snapshot_id,price_snapshot_digest) -> compute_price_snapshots(snapshot_id,snapshot_digest)
```

V274 pair在fresh checked-at必须是当时current successor并与V277/V278 stable lineage一致；写入后只作historical audit，
后续refresh不要求该pair继续current。
`route_authorization_*`只是V278 row的denormalized historical/source-trigger projection；route seal仍从immutable V278 receipt pair
追溯。两者都不能供Tx-B重建或替代fresh route authority，因此不另复制`route_seal_*`列。
Compiled market profile/allocation没有 DB parent，必须由 pure profile validator、allocation re-derive 与 ordered plan 审计。
Price source四元组、publication/Offer/Pool/bucket状态和所有跨根相等关系由 BEFORE INSERT exact-source trigger再次 join；FK
本身不能证明 currentness。

Child table的PK/UNIQUE/index name与tuple固定为：

```text
PRIMARY KEY(admission_receipt_id)
ux_compute_external_pool_service_managed_admissions_receipt_digest(admission_receipt_digest)
ux_compute_external_pool_service_managed_admissions_integrity_digest(admission_integrity_digest)
ux_compute_external_pool_service_managed_admissions_request_digest(request_digest)
ux_compute_external_pool_service_managed_admissions_provider(provider_id)
ux_compute_external_pool_service_managed_admissions_idempotency(idempotency_scope,idempotency_key)
ux_compute_external_pool_service_managed_admissions_provider_binding(provider_binding_id,provider_binding_digest)
ux_compute_external_pool_service_managed_admissions_allocation_id(capacity_allocation_id)
ux_compute_external_pool_service_managed_admissions_allocation_digest(capacity_allocation_digest)
ux_compute_external_pool_service_managed_admissions_pool(pool_id)
ux_compute_external_pool_service_managed_admissions_supply(supply_transaction_id,supply_transaction_digest,supply_ledger_sequence)
ux_compute_external_pool_service_managed_admissions_event(pool_activation_event_id,pool_activation_event_request_digest)
ux_compute_external_pool_service_managed_admissions_offer(offer_id)
ux_compute_external_pool_service_managed_admissions_publication(publication_id,publication_digest)
ux_compute_external_pool_service_managed_admissions_snapshot(price_snapshot_id,price_snapshot_digest)
ux_compute_external_pool_service_managed_admissions_quote(price_quote_id)
ux_compute_external_pool_service_managed_admissions_receipt_exact(admission_receipt_id,admission_receipt_digest)
idx_compute_external_pool_service_managed_admissions_current(expires_at,provider_id) -- non-unique selector only
```

任何实现不得把pair/triple拆成独立UNIQUE、缩成只ID或改成另一tuple。最后一个non-unique index只服务只读current selector，
不是authority。

## 8. Insert guard、replay 与 readback

每个 connection open 必须注册 deterministic、innocuous、arity-1 UDF
`elon_v280_external_pool_service_managed_admission_is_exact`；它只做 deny-unknown parse、deep validation、本页可由receipt
material重算的ID/request/integrity/receipt/bucket-inventory与embedded profile/allocation digest，以及canonical byte equality。
V249/V274/V277/V278/Provider/Pool/Offer/publication/snapshot等外部pair只做shape/cross-field检查，必须由source trigger和
所属owner Store join重证。Migration installer和正常Store connection缺任一注册都失败关闭。Market事务另用父权威定义的
non-deterministic ordered one-shot pending-plan UDF；两类 UDF 不得合并。

Fresh/replay在同一个 `BEGIN IMMEDIATE` 开头，先按 `provider_id`、`admission_receipt_id`、
`(idempotency_scope,idempotency_key)` 三路查询：

- 三路均无 row 才允许读取 current Provider/V249/V277/V274/V278/profile并准备 fresh market事务；
- 任一路命中时，所有命中必须收敛到同一 row，request digest与 canonical/scalar readback逐字一致，随后0 current read、0 write返回；
- split identity、同Provider不同request、同scope/key不同material或canonical mismatch全部拒绝。

原名guard固定为`trg_compute_external_pool_service_managed_admissions_no_update`、
`trg_compute_external_pool_service_managed_admissions_no_delete`、
`trg_compute_external_pool_service_managed_admissions_no_replace`与
`trg_compute_external_pool_service_managed_admissions_exact_source`。SQLite不保证多个BEFORE INSERT trigger顺序，因此
no-replace与exact-source自身都必须在三路replay identity或任一上述named UNIQUE target已存在时失败关闭；独立no-replace只是
同义防线，不能依赖REPLACE触发DELETE，
也不能假定它先于pending-UDF。任一执行顺序或abort都由RAII清空plan。Exact-source还必须同时要求exact pending plan、
canonical UDF、77列↔JSON投影（含raw profile TEXT↔receipt内
decoded string byte equality）、所有 parent historical row与
同事务新建 Pool/ledger/event/Offer/publication/snapshot exact join、bucket inventory重算及 fresh checked-at currentness。
INSERT 后 fresh Store 必须按固定77列readback、从JSON重建typed receipt、重算本页派生digest、逐列比较、由owner join重证
外部pair与`read_at` currentness，并确保plan fully
consumed后才commit。Exact replay只做canonical/scalar与immutable historical source audit，禁止重新要求mutable market head current。

## 9. Planned owner 与实施门

计划 owner：

```text
compute_federation/external_pool_service_managed_admission/{types,canonical,validated}.rs
store_migrations/compute_external_pool_service_managed_admission/{tables,guards,precheck}.rs
store/compute_external_pool_service_managed_admissions/{read,write,currentness,plan}.rs
```

Domain DTO private fields、deny-unknown；canonical parser可用于readback，current authority必须non-Clone/non-Serde且只由
Store transaction构造。Source-written 前还必须实现已冻结projection identity ABI、落实fixed source observation window、提交首个
byte-exact profile payload，并冻结production fence所在的Gateway/session/validator ABI，与完整 V280 writer/Runner 同批落盘。
本批不声明任何 symbol/table/UDF已存在，不执行编译、测试、migration、
SQLite、runtime 或 network；正式计数保持 `passed=0/failed=0`。
