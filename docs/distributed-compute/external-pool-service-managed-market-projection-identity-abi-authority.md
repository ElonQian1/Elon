---
title: 外部矿池 service-managed market projection identity ABI 权威
reviewed_at: 2026-08-21
status: current
owners: backend, security, ai-economy
design_status: design_frozen
design_scope: market_projection_identity_and_legacy_owner_mapping_abi_v1
implementation_status: implementation_unwired
verification_status: design_review_only
---

# 外部矿池 service-managed market projection identity ABI 权威

## 1. 唯一结论与状态边界

本页冻结 V280 genesis market transaction 中 CapacityPool、bucket、supply ledger、Pool activation、draft/active
Offer、publication 与 v171 price snapshot 的 deterministic identity、单一时钟和 legacy owner digest 映射。它不选择首个
profile 的价格、容量、runtime 或 deadline 载荷，不注册 migration，不打开 V254 fence，也不实现 writer、Gateway、ELTP
validator 或 Runner。

当前状态固定为：

```text
vertical_slice_architecture=design_frozen
market_profile_schema_abi=design_frozen
market_profile_inventory_approval_evidence_abi=design_frozen
admission_receipt_physical_schema_abi=design_frozen
market_projection_identity_abi=design_frozen
gateway_builder/fence/task_session/validator_internal_abi=design_frozen
external_adapter_semantic_wire_profile=unselected
initial_profile_inventory=unselected
initial_profile_approval_evidence=unselected
current_profile_authority=unconstructible
physical_migration_registration=absent
migration_registry_max=280
migration_registry_last_owner=erp_managed_rollout
planned_physical_migration=unassigned
v280_market_writer_source=absent
implementation=unwired/uncompiled/unrun
passed=0
failed=0
```

纵切编排见 [V280 父权威](external-pool-service-managed-admission-runner-authority.md)，profile 与 admission receipt 分别见
[profile ABI](external-pool-service-managed-market-profile-authority.md)和
[admission receipt ABI](external-pool-service-managed-admission-receipt-abi-authority.md)；本页验收见
[projection identity acceptance](external-pool-service-managed-market-projection-identity-abi-acceptance.md)。`V280`只是阶段名，
不预留物理 migration 280；未来实现必须使用届时 registry 的 next-free 编号。

## 2. 输入、输出与排除项

唯一 Store-private orchestrator 只能从同一 `BEGIN IMMEDIATE` 内取得：

- pre-current 可派生的 `admission_receipt_id/request_digest` typed identity；
- fresh typed active external_pool Provider、exact V249 binding、current profile authority及其 allocation；
- 同一 transaction 只采样一次的 server `checked_at`。

输出是 non-authorizing、non-Serde、non-Clone 的 planned market projection material；它只允许 owner-local `_on` kernels按父权威
固定顺序消费，不授予 direct SQL、HTTP、Provider owner、fixture 或其他 Store caller 写权限。

`Tx-B fence_digest`明确排除在本页之外；它已由
[Gateway/session/validator internal ABI](external-pool-service-managed-gateway-session-validator-abi-authority.md)冻结为Plan+seal
派生值。Production不能复用conformance fixture、Pool、route credential、admission digest或随机值；external semantic wire
profile仍未选择。

Market admission actor继续逐字使用 admission ABI 的：

```text
market_admission_actor_kind=platform_market_service
market_admission_actor_id=external_pool_service_managed_market
```

该 service actor只表示机械执行者与 deterministic namespace seed，不得写入 legacy `approved_by_user_id`。Profile
`review_source`必须显式携带经
[approval evidence ABI](external-pool-service-managed-market-profile-approval-evidence-abi-authority.md)审计的真实
`approved_by_user_id`；publication 只绑定该用户，且审批 evidence
必须覆盖profile ABI定义的exact ID/revision/review-material digest，final profile digest再传递绑定source digest。首个 inventory
尚未选择，所以当前没有可用approver或正向writer。

## 3. Canonical checked-at 与时间投影

Fresh transaction只允许一次 server clock read，格式固定为
`YYYY-MM-DDTHH:MM:SS.nnnnnnnnnZ`（UTC、9位小数、`Z`）。所有 owner kernel接收同一个 borrowed checked-at，不得调用
`Utc::now()`、`now()`、环境变量或 caller time。Replay从 stored admission receipt读取原 checked-at，禁止采新时钟。

逐字时间投影固定为：

```text
Pool root created_at/updated_at = checked_at
Pool version created_at = checked_at
bucket created_at/updated_at（create与supply CAS后） = checked_at
supply occurred_at/recorded_at = checked_at
ledger leg created_at = checked_at
Pool activation occurred_at/recorded_at 与 active CAS updated_at = checked_at
Offer JSON created_at/valid_from = checked_at
Offer root first/current-version/recorded time 与两版 physical created_at = checked_at
delivery_window.starts_at_utc = checked_at
delivery_window.ends_at_utc = profile.inflight_execution_valid_until
publication published_at/created_at = checked_at
snapshot quoted_at/physical created_at = checked_at
snapshot observation window = [checked_at-1s, checked_at]
snapshot expires_at = min(checked_at+quote_ttl, profile.new_plan_accept_until, profile.expires_at)
admission valid_from/created_at = checked_at
```

Supply request 的既有 owner helper会把同一 instant 规范化成其 legacy RFC3339 表示后再计算 request digest；这只是 legacy
preimage 规则，不允许改变持久化字段的 nanos checked-at，也不能把该 raw-serde digest重解释为本页 domain digest。

## 4. V280-only deterministic identity 与 request material

本页新 ID/request 使用：

```text
domain_digest(domain,value)=SHA256(domain UTF-8 || 0x00 || RFC8785-JCS(value) UTF-8)
```

Material必须 exact key set、I-JSON、safe integer、deny unknown；digest为64位小写hex。固定 domain/prefix：

```text
RESOURCE_SCOPE_KEY_DOMAIN=ELON-EXTERNAL-POOL-SERVICE-MANAGED-RESOURCE-SCOPE-KEY-V1
RESOURCE_SCOPE_KEY_PREFIX=external_pool_service_managed_resource_scope_v1_
POOL_ID_DOMAIN=ELON-EXTERNAL-POOL-SERVICE-MANAGED-CAPACITY-POOL-ID-V1
POOL_ID_PREFIX=external_pool_service_managed_capacity_pool_v1_
WINDOW_ID_DOMAIN=ELON-EXTERNAL-POOL-SERVICE-MANAGED-DELIVERY-WINDOW-ID-V1
WINDOW_ID_PREFIX=external_pool_service_managed_delivery_window_v1_
BUCKET_ID_DOMAIN=ELON-EXTERNAL-POOL-SERVICE-MANAGED-CAPACITY-BUCKET-ID-V1
BUCKET_ID_PREFIX=external_pool_service_managed_capacity_bucket_v1_
SUPPLY_TRANSACTION_ID_DOMAIN=ELON-EXTERNAL-POOL-SERVICE-MANAGED-CAPACITY-SUPPLY-TRANSACTION-ID-V1
SUPPLY_TRANSACTION_ID_PREFIX=external_pool_service_managed_capacity_supply_transaction_v1_
LEDGER_LEG_ID_DOMAIN=ELON-EXTERNAL-POOL-SERVICE-MANAGED-CAPACITY-LEDGER-LEG-ID-V1
LEDGER_LEG_ID_PREFIX=external_pool_service_managed_capacity_ledger_leg_v1_
POOL_ACTIVATION_EVENT_ID_DOMAIN=ELON-EXTERNAL-POOL-SERVICE-MANAGED-CAPACITY-POOL-ACTIVATION-EVENT-ID-V1
POOL_ACTIVATION_EVENT_ID_PREFIX=external_pool_service_managed_capacity_pool_activation_event_v1_
POOL_ACTIVATION_REQUEST_DOMAIN=ELON-EXTERNAL-POOL-SERVICE-MANAGED-CAPACITY-POOL-ACTIVATION-REQUEST-V1
PUBLICATION_ID_DOMAIN=ELON-EXTERNAL-POOL-SERVICE-MANAGED-OFFER-PUBLICATION-ID-V1
PUBLICATION_ID_PREFIX=compute_offer_publication_v1_
```

Exact materials：

| output | exact material keys |
|---|---|
| `resource_scope_key` | `admission_receipt_id,provider_id,capacity_allocation_id,capacity_allocation_digest` |
| `pool_id` | `admission_receipt_id,provider_id,capacity_allocation_id,capacity_allocation_digest,resource_scope_digest` |
| `delivery_window_id` | `pool_id,capacity_epoch,pool_revision,pool_digest,starts_at_utc,ends_at_utc` |
| `bucket_id` | `pool_id,capacity_epoch,pool_revision,pool_digest,delivery_window_id,delivery_window_digest,meter,meter_mode,quantum_units,meter_policy_digest` |
| `supply_transaction_id` | `admission_receipt_id,pool_id,capacity_epoch,pool_revision,pool_digest,delivery_window_id,delivery_window_digest,ledger_sequence,bucket_id,bucket_digest,quantity_units` |
| each `leg_id` | `transaction_id,transaction_digest,line_no,leg_role,bucket_id,meter,account,delta_units,created_at` |
| activation `event_id` | `admission_receipt_id,pool_id,capacity_epoch,pool_revision,pool_digest,previous_status,target_status` |
| activation `request_digest` | `event_id,admission_receipt_id,pool_id,capacity_epoch,pool_revision,pool_digest,previous_status,target_status,reason_code,subject_kind,subject_id,idempotency_scope,idempotency_key,occurred_at` |
| `publication_id` | `admission_receipt_id,offer_id,source_offer_version,source_offer_digest,active_offer_version,active_offer_digest` |

每个 ID 等于对应prefix加domain digest。Activation request只有digest、没有prefix。不存在互相循环：admission ID只由
Provider+sequence预派生；transaction digest不含leg ID；event ID不含request digest；publication ID先于publication digest。

## 5. Capacity、supply 与 lifecycle legacy owner 映射

新 domain只生成上节列出的V280-only identity。以下 digest 必须调用现有所属 owner抽出的纯 helper，并用 golden/source-contract
锁 raw `serde_json` 字段顺序；V280 orchestrator不得复制一份“等价JSON”：

- `resource_scope_digest`：legacy `SHA256(serde_json({provider_id,resource_scope_key}))`；
- `resource_profile_digest`：profile ABI固定三key value
  `{runtime_policy,resource_profile,resource_ceiling}` 的 legacy serde bytes SHA；
- meter policy：唯一 `attempt_slot/reusable/1`，digest preimage为
  `{meter,meter_mode,quantum_units}`；
- Pool digest：exact
  `{schema,pool_id,capacity_epoch,pool_revision,provider_id,resource_scope_digest,resource_profile_digest,region_or_data_zone,meter_policies}`；
- window digest：exact `{schema,window_id,starts_at_utc,ends_at_utc}`；
- bucket digest：exact
  `{schema,bucket_id,pool,delivery_window,meter,meter_mode,quantum_units,meter_policy_digest}`。该legacy helper先构造
  `serde_json::Value`；当前default map按词典序编码（window为`binding,ends_at_utc,starts_at_utc`，binding为
  `window_digest,window_id`）。必须调用owner helper并用golden锁实际bytes，不得按Rust struct声明顺序手拼。

Legacy schema逐字固定为：

```text
Pool = compute_federation.capacity_pool.v1
bucket = compute_federation.capacity_bucket.v1
delivery window = compute_federation.delivery_window.v1
add-supply request = compute_federation.capacity_add_supply_request.v1
capacity transaction = compute_federation.capacity_transaction.v1
```

Pool physical `resource_profile_json`是exact两key `{digest,profile}` wrapper；`supported_meters_json`是exact length-1 array，
唯一元素exact四key `{meter,meter_mode,quantum_units,policy_digest}`。Fresh/historical owner reader必须审canonical bytes与无extra key，
不能依赖会吞unknown field的普通DTO deserialize。

V1固定 epoch/revision=1、初始 Pool=`registering`、bucket=`open`；resource scope key不落表。Pool region取
`profile.sku.region_or_data_zone`，且必须通过legacy owner的trim/nonempty/最多80 Unicode scalar约束。Bucket create先写零余额/revision0/sequence null；supply CAS后才形成 admission receipt中的
issued=available=allocation、revision=sequence1 snapshot。

Supply固定：

```text
ledger_sequence=1
event_kind=supply_added
claim_effect=null
causal_binding={offer:null,job_id:null,reservation_id:null,attempt_lease_id:null,fencing_generation:null}
idempotency_scope=external_pool_service_managed_capacity_supply
idempotency_key=supply_transaction_id
subject_kind=platform_market_service
subject_id=external_pool_service_managed_market
causal_transaction_id=null
movements.len=1
movement line_no=0, quantity=allocation_total_units, issuance -> available
```

`request_digest`必须复用 `add_supply_request_digest`：top-level字段顺序
`schema,pool,delivery_window,subject_kind,subject_id,lines,occurred_at`，line按bucket ID/quantity排序，time沿owner legacy
normalization。`transaction_digest`必须复用 `finalize_transaction_digest` 的17-key legacy payload：
`schema,transaction_id,pool,delivery_window,ledger_sequence,event_kind,claim_effect,causal_binding,idempotency_scope,
idempotency_key,request_digest,subject_kind,subject_id,causal_transaction_id,movements,occurred_at,recorded_at`。

唯一movement展开成两条exact leg：`from/issuance/-allocation`后`to/available/+allocation`；两者line_no=0、meter=attempt_slot，
created-at同checked-at。Leg无digest，ID按上节逐条派生。

Activation event固定：

```text
previous_status=registering
target_status=active
reason_code=external_pool_service_managed_market_admission
subject_kind=platform_market_service
subject_id=external_pool_service_managed_market
idempotency_scope=external_pool_service_managed_capacity_pool_activation
idempotency_key=event_id
```

真实表没有event digest；admission只保存`event_id/request_digest` pair，不得把request digest改名成event digest。

## 6. Offer、publication 与 v171 snapshot 映射

Offer ID必须复用/抽取既有 `deterministic_offer_id` legacy算法，不新增JCS domain。Exact raw-serde material为
`{purpose:"compute_offer_draft_create",user_id,provider_id,pool_id,idempotency_key}`；V280映射
`user_id=market_admission_actor_id`、`idempotency_key=admission_receipt_id`，结果保持`offer_draft_`+64hex。这里的
`user_id`只是不持久化的legacy namespace slot，不表示该service actor是审核用户。

Draft exact version1/status=draft；SKU digest调用 `compute_sku_digest`，Offer digest调用 `compute_offer_digest`，即clone后只清空
self digest再按typed struct serde bytes SHA。Active exact clone draft，只改version=2、status=active并重算digest；其余字段包括
Offer JSON created-at逐字相等。Profile→Pool/SKU/runtime/resource/capacity/price/authorization映射继续由profile ABI约束。
其中`price_terms.curve_id=profile.price.curve_id`、`curve_version=Some(profile.price.curve_revision)`且instrument null；snapshot的
source ID/version/digest是另一组字段，不能用curve pair代替。

Publication：

```text
idempotency_scope="compute_offer_publish:" + offer_id
idempotency_key=admission_receipt_id
approved_by_user_id=profile.review_source.approved_by_user_id
published_at=created_at=checked_at
```

Profile review `approved_at<=profile.valid_from<=checked_at`，但审批与投影时间不得互换：前者是历史产品审批时间，后者是本次
legacy projection落盘时间。

Publication digest继续调用所属 owner 的13-key legacy helper：
`schema,publication_id,offer_id,provider_id,pool_id,source_offer_version,source_offer_digest,active_offer_version,
active_offer_digest,provider_policy_revision,provider_digest,approved_by_user_id,published_at`。`price_snapshot_effect=none`保持
不变；ephemeral receipt还固定`offer_effect=active,capacity_effect=none,funds_effect=none`，fresh为`replayed=false`、exact replay为
`replayed=true`。这些effect/replay flag不进入13-key persisted digest；initial snapshot是其后的独立受管写入。

Snapshot/quote ID必须复用/抽取既有 owner-facing deterministic helper，不复用v223 batch/entry helper，也不新增domain。
Exact raw-serde material为`{purpose,user_id,provider_id,pool_id,offer_id,idempotency_key}`；purpose分别为
`price_snapshot`与`quote`，`user_id=market_admission_actor_id`、key=admission receipt ID，输出保持
`price_snapshot_`/`quote_`+64hex。

Snapshot必须绑定active Offer version2及其唯一window，price source的kind/id/version/digest/sample-count逐字取profile，
components、fee rules、max amounts、rounding也逐字取profile；observation window与expiry按第3节。Digest继续调用
`compute_price_snapshot_digest`（只清空self digest后typed serde SHA），不能改用profile/admission domain或public
`fallback_source`重新推价。

## 7. Owner-local `_on` seams、replay 与 readback

现有 facades各自开启transaction并调用`now()/new_id()`，不能嵌入父事务。实施时必须在各所属 owner提取或扩展
transaction-aware `_on` kernel：

- Pool/bucket owner接收外层connection、planned IDs、legacy material与checked-at；
- ledger/posting owner接收planned transaction/两条leg IDs，复用request、sequence、digest、reducer并强制sequence1；
- lifecycle owner接收planned event ID/request/recorded-at；
- Offer owner接收planned projection/version physical times；
- publication owner接收planned ID、exact approver与checked-at，并补全scope/key/approver/time replay审计；
- snapshot owner接收planned snapshot与physical created-at；current-time只做live check，不能自己取时钟。

Active Offer currentness校验也必须接收同一checked-at；V280不得直接调用现有隐式`Utc::now()`分支。Public facade可自行
采样一次时钟再传入owner validator，但不能把该入口暴露给V280 sealed plan。

Store内核为`pub(in crate::store)`；若纯 legacy helper位于crate sibling，只能`pub(crate)`并由source-contract锁现有owner与唯一
V280 adapter caller。不得公开 raw constructor、Clone authority、standalone V280 facade或第二次commit。

V280授权入口不能只接裸`ComputeCapacityPool`/`ComputeCapacityBucket`等普通DTO；它必须消费private-field、non-Serde、
non-Clone的sealed planned projection token，或由所属owner从该token重新构造并重算全部ID/digest/time。现有basic shape
validator不构成authority，Store sibling也不能用generic `_on`伪造V280写入。

Fresh transaction先做admission三路replay lookup。Admission不存在时，任一 deterministic Pool/bucket/supply/event/Offer/
publication/snapshot identity已存在都属于orphan、旁路写或污染，必须fail-close；V280 wrapper不得继承generic leaf
“存在即幂等返回”并继续补齐后续行。只有整份admission exact replay可0 current read、0 write返回。

Fresh每个 owner kernel必须`replayed=false`、逐步消费ordered one-shot plan，最终按admission ABI重算bucket inventory、77列及全部
legacy owner digests，exact readback后才commit。Historical/replay只审计immutable legacy rows与准入时balance reconstruction，
不得把mutable bucket余额、rolling V274/V278 head或新时钟当历史恒等。

Readback必须使用owner-specific full-column `_on` reader，不能拼接现有public/current view：它要读取并审计Pool root/version的
全部identity、digest和physical time，bucket binding/config及create time，supply transaction的request/actor/idempotency/causal/
time全列，两条leg的deterministic ID与created-at，以及activation event的完整request/time。`resource_profile_json`必须是exact
两key wrapper `{digest,profile}`，`meters`必须是exact length-1 array且元素无extra key；普通Serde吞掉unknown field不得算通过。
Fresh readback还逐字核初始/供给后mutable head；historical则由immutable sequence-1 transaction与两条legs重建genesis snapshot。

未来physical migration还必须为本阶段external_pool行安装静态历史保护：Pool root禁止delete，并永久锁provider、resource
scope、created-at与`current_capacity_epoch=1`；既有append-only Pool version继续锁`epoch=1/revision=1`。Root只允许两种
ordered update：registry projection步骤保持status=`registering`且只把updated-at写为checked-at；其后lifecycle步骤在exact event后
执行`registering→active`并写同一updated-at。两者都不得改identity；draining/retired/quarantined后移独立authority。
Bucket禁止delete，并永久锁bucket/pool/window/meter/policy/digest/create-time等binding/config；只有ledger/claim所属owner可CAS其
明确列举的status、balance、revision、through-sequence与updated-at。Guard不得阻断合法available→held→active守恒转换，也不得
把这些mutable head误当历史receipt恒等；任何其他UPDATE、REPLACE或direct SQL均失败关闭。

所有 owner-facing Pool/bucket/supply/withdraw/Offer create-revise-revoke/publication与owner/v223 snapshot seam必须显式拒绝
external_pool；只有持有sealed ordered plan的V280 `_on` caller可写。Direct SQL、fixture、seed、admin curve、Provider owner与
market actor常量都不能绕过source guards。

## 8. Source-written 门与非目标

Planned source owner继续沿父权威分配给capacity registry/ledger/posting/lifecycle、Offer registry/publication、v171 snapshot与
`store/compute_external_pool_service_managed_admissions`集成模块。Source-written 前还必须：

1. 按approval evidence ABI审批并提交首个byte-exact profile/evidence pair，包括distinct真实submitter/approver与全部经济/资源载荷；
2. 实现本页owner helpers、planned `_on` seams、orphan guards、golden vectors与source-contract；
3. 按已冻结admission ABI实现Domain/DDL/Store/UDF/trigger、full-column readback及Pool/bucket immutable-column guards，并在届时
   next-free migration登记；
4. 实现已冻结的Gateway/session/validator内部ABI，选择external semantic wire profile，并与完整Runner纵切同批集成；
5. 保持V254 #13/#15 absolute deny，只按父权威开放四个trigger的五次ordered命中。

本批不新增Rust/SQL/schema/API，不编译、不测试、不执行migration/SQLite/runtime/network。静态审计不计入正式
`passed/failed`，也不能把design freeze称为implemented、source-written或production reachable。
