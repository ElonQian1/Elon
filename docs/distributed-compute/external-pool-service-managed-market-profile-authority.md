---
title: 外部矿池 service-managed market profile canonical ABI 权威
reviewed_at: 2026-08-21
status: current
owners: backend, security, ai-economy
design_status: design_frozen
design_scope: market_profile_schema_and_canonical_abi_v1
implementation_status: implementation_unwired
verification_status: design_review_only
---

# 外部矿池 service-managed market profile canonical ABI 权威

## 1. 唯一结论与状态边界

本页只冻结 V280 server-owned market profile 的 schema/canonical ABI、纯验证边界和 current-selection 规则。它不选择首个
production profile 的价格、容量、SKU、runtime 或时限载荷，也不授权写 Pool、Offer、Snapshot、Plan、Start、Lease 或任何
经济效果。

当前状态向量固定为：

```text
vertical_slice_architecture=design_frozen
market_profile_schema_abi=design_frozen
initial_profile_inventory=unselected
current_profile_authority=unconstructible
admission_receipt_physical_schema_abi=design_frozen
market_projection_identity_abi=design_frozen
implementation=unwired/uncompiled/unrun
passed=0
failed=0
```

完整纵切的编排、事务、Runner 与恢复仍由 [V280 父权威](external-pool-service-managed-admission-runner-authority.md)
和 [父验收](external-pool-service-managed-admission-runner-acceptance.md)负责；本页的验收见
[market profile acceptance](external-pool-service-managed-market-profile-acceptance.md)。Pool/Offer/Snapshot 的 deterministic identity、
legacy owner helper与单一checked-at映射见
[market projection identity ABI](external-pool-service-managed-market-projection-identity-abi-authority.md)。

## 2. 来源边界与禁止默认值

仓库当前没有可信的 external-pool 产品级 capacity、price、SKU/runtime 或 numeric execution ceiling source。以下来源
一律不能升格为 enabled profile：

- V270 readiness、Provider declaration、Offer 自报或历史 receipt；
- `test_support`、fixture、seed、示例、测试中的价格/资源数值；
- user-node plugin manifest、route credential 或 V277 stable executor 身份；
- `0`、`1`、最小合法值或“免费”作为所谓安全默认；它们仍是经济或资源承诺。

已有正式合同只授权复用字段形状与结构规则。V280 固定 `spot/CNY/fallback_curve/sample_count=0`、空 fee rules、
`trade_id=None`、`instrument_id=None` 和 `half_up`；v223 platform reference curve 固定 `half_even`，两者不能互称同一
producer 或共享未经转换的 policy root。

## 3. Canonical envelope 与 digest

固定常量：

```text
PROFILE_SCHEMA=compute_federation.external_pool_service_managed_market_profile.v1
PROFILE_CANONICALIZATION=rfc8785_jcs
PROFILE_DIGEST_ALGORITHM=sha256
PROFILE_MAX_JSON_BYTES=1048576
PROFILE_ID_DOMAIN=ELON-EXTERNAL-POOL-SERVICE-MANAGED-MARKET-PROFILE-ID-V1
PROFILE_DIGEST_DOMAIN=ELON-EXTERNAL-POOL-SERVICE-MANAGED-MARKET-PROFILE-V1
PROFILE_REVIEW_MATERIAL_DOMAIN=ELON-EXTERNAL-POOL-SERVICE-MANAGED-MARKET-PROFILE-REVIEW-MATERIAL-V1
CAPACITY_ALLOCATION_DOMAIN=ELON-EXTERNAL-POOL-SERVICE-MANAGED-CAPACITY-ALLOCATION-V1
LEASE_ISSUER_POLICY_SCHEMA=compute_federation.external_pool_service_managed_lease_issuer_policy.v1
LEASE_ISSUER_POLICY_DOMAIN=ELON-EXTERNAL-POOL-SERVICE-MANAGED-LEASE-ISSUER-POLICY-V1
LEASE_REF_DOMAIN=ELON-EXTERNAL-POOL-SERVICE-MANAGED-LEASE-DELIVERY-REF-V1
LEASE_HINT_DOMAIN=ELON-EXTERNAL-POOL-SERVICE-MANAGED-LEASE-DELIVERY-HINT-V1
```

本页新定义的 profile ID/review-material/final digest、allocation digest与lease policy/ref/hint digest都使用：

```text
SHA256(domain UTF-8 || 0x00 || RFC8785-JCS(value) UTF-8)
```

Profile envelope exact key set：

```text
schema
profile_id
profile_revision
profile_digest
canonicalization
digest_algorithm
profile
```

`profile_revision` 是 `1..=9007199254740991` 的 JSON-safe `i64`。`profile_id` 固定为
`external_pool_market_profile_v1_` 加上 `PROFILE_ID_DOMAIN` 对 exact
`{"profile_revision": revision}` 投影的 64-hex。`profile_digest` 使用 `PROFILE_DIGEST_DOMAIN` 对完整 envelope 计算，
计算时保留 `profile_digest` key 并把值置为空串；不得删除字段、改用普通 serde JSON、双重编码或无 domain SHA-256。

审批链必须无环。先以完整7/17-key envelope为投影，保留所有key，只把top-level `profile_digest`和nested
`review_source.source_digest`同时置为空串，使用`PROFILE_REVIEW_MATERIAL_DOMAIN`计算非持久化
`profile_review_material_digest`。Typed approval evidence必须绑定exact
`{profile_id,profile_revision,profile_review_material_digest,approved_by_user_id,approved_at}`；其owner digest再填入
`review_source.source_digest`，最后才按上一段计算final `profile_digest`。禁止让approval evidence绑定final digest后又把自身digest
放回final preimage，也禁止删除任一blank key或把material/evidence/final三种digest互换。

`profile` exact key set：

```text
owner
review_source
valid_from
new_plan_accept_until
expires_at
inflight_execution_valid_until
capacity
sku
runtime_policy
resource_profile
resource_ceiling
price
workload_policy
offer_policy
transport_policy
deadline_policy
lease_issuer_policy
```

所有 DTO 必须 `deny_unknown_fields`；输入先按 UTF-8 bytes 拒绝空值或超过 1 MiB，再要求 I-JSON、无重复 key、无
float/NaN/Infinity、无超出 safe-integer 范围的整数。除显式允许为0的price micros、max amount与固定`sample_count=0`
外，数值型revision/version/ordinal/count/duration都必须为正safe integer；尤其`curve_revision/source_version>0`。
`runtime_policy.runtime_version`是与既有`ComputeRuntimeRef`逐字投影的1..160-byte identifier string，不是整数。时间一律为
canonical UTC nanos；identifier、enum、meter、runtime、
media type 与actor/source字段必须trim后1..160 bytes且无控制字符，digest必须lowercase 64-hex。解析后必须完成 deep
validation、重算全部派生 digest，再逐字比较输入与重新生成的 RFC8785-JCS bytes。

## 4. Exact nested key sets

| 对象 | exact keys |
|---|---|
| `owner` | `owner_kind,owner_id` |
| `review_source` | `source_kind,source_id,source_revision,source_digest,approved_by_user_id,approved_at` |
| `capacity` | `capacity_scope,capacity_unit,allocation_total_units,buckets` |
| `capacity.buckets[]` | `ordinal,meter,meter_mode,quantum_units,issued_units` |
| `sku` | `sku_id,task_kind,model_family,model_digest,tokenizer_digest,runtime_family,precision,context_or_shape_bucket,verification_tier,sla_tier,region_or_data_zone,delivery_window_class,metering_units` |
| `runtime_policy` | `runtime_family,runtime_version,precision,model_allowed,plugin_release_allowed` |
| `resource_profile` | `accelerator_kind,accelerator_count,vram_bytes,ram_bytes` |
| `resource_ceiling` | `accelerator_count,max_cpu_millicores,max_memory_bytes,max_vram_bytes,max_disk_bytes,max_processes,max_runtime_seconds,max_output_bytes,max_concurrent_attempts,allow_network_egress` |
| `price` | `pricing_mode,currency,rounding_mode,curve_id,curve_revision,source_kind,source_id,source_version,source_digest,quote_ttl_seconds,components,fee_rules,consumer_max_amount_micros,provider_max_amount_micros,sample_count,trade_id,instrument_id` |
| `price.components[]` | `meter,unit_size,consumer_unit_price_micros,provider_unit_price_micros,max_units` |
| `workload_policy` | `allowed_task_kinds,input_artifacts_allowed,model_allowed,result_artifact_required,checkpoint_mode,allowed_output_media_types` |
| `offer_policy` | `public,allowed_account_ids,allowed_project_ids,allowed_data_classes,policy_revision` |
| `transport_policy` | `ledger_json_max_bytes,outer_request_max_bytes,upstream_request_max_bytes,response_max_bytes,observation_max_bytes,events_per_batch_max,exchange_ordinal_max,exchange_timeout_ms` |
| `deadline_policy` | `retry_max_attempts,retry_initial_backoff_ms,retry_max_backoff_ms,reconcile_poll_interval_ms,event_poll_interval_ms,pre_start_cleanup_grace_seconds,task_session_max_seconds` |
| `lease_issuer_policy` | `schema,issuer_policy_id,issuer_revision,issuer_policy_digest,authority_kind,delivery_mode,audience_kind,required_scopes,ref_prefix,hint_prefix` |

Nullable 只允许 `sku.model_family/model_digest/tokenizer_digest`、`price.trade_id/instrument_id`；V1 均必须为 `null`。
所有数组必须按其 canonical identifier 升序、唯一；V1 的 `fee_rules=[]`，不得以 `null` 代替空数组。

## 5. 结构性不变量

Capacity V1 固定为单 bucket 结构，但不替产品选择实际容量：

```text
capacity_scope=per_provider_genesis
capacity_unit=attempt_slot
buckets.len=1
buckets[0].ordinal=1
buckets[0].meter=attempt_slot
buckets[0].meter_mode=reusable
buckets[0].quantum_units=1
buckets[0].issued_units=allocation_total_units=max_concurrent_attempts
allocation_total_units in 1..=9007199254740991
```

每个 Reservation/Claim exact 占 1 `attempt_slot`；同 Provider 的 held+active 总数不得超过 allocation total。该数值是
per-Provider genesis allocation，不是 global pool；不同 Provider 必须得到不同 allocation identity。
Offer单条capacity的`total_units=reservable_units=allocation_total_units`；execution limits的
`max_concurrent_attempts=allocation_total_units`且`max_attempt_runtime_seconds=resource_ceiling.max_runtime_seconds`。

Resource ceiling 九个整数均在 `1..=9007199254740991`，`allow_network_egress` 是显式产品布尔值。
`resource_profile.accelerator_count=resource_ceiling.accelerator_count`、`vram_bytes=max_vram_bytes`、
`ram_bytes=max_memory_bytes`；`accelerator_kind`须由产品显式选择。SKU/runtime 必须满足
`sku.runtime_family=runtime_policy.runtime_family`、`sku.precision=runtime_policy.precision`，并在将来生成的 Offer、Job 与
capability 中逐字保持；V1 `model_allowed=false`、`plugin_release_allowed=false`、
`sku.metering_units=[attempt_slot]`。Profile不保存legacy `sku_digest`；Store投影时使用既有owner helper按legacy
canonical规则生成，不能把它重解释为本页JCS domain。
`sku.region_or_data_zone`除本页1..160-byte identifier规则外，还必须满足legacy CapacityPool owner的trim/nonempty与
`chars().count()<=80`；81个Unicode scalar或更长值在inventory审批前即失败关闭。

`resource_profile_digest`不是caller字段，也不使用本页新domain。为兼容既有Pool current read，V280必须构造 exact
`resource_profile_json.profile={"runtime_policy":...,"resource_profile":...,"resource_ceiling":...}` value，并沿
Pool owner现有规则计算`hex(SHA256(serde_json::to_vec(profile_value)))`；该值逐字投影到Pool
`resource_profile_digest`与Offer `declared_profile_digest`。V280 owner audit必须锁exact三key shape且禁止extra，不能把
legacy digest重解释为JCS/domain digest。Provider observed/verified hardware digest仍取fresh typed source，不得写死进profile
或用profile digest冒充观测事实。

Price V1 固定：

```text
pricing_mode=spot
currency=CNY
rounding_mode=half_up
source_kind=fallback_curve
sample_count=0
trade_id=null
instrument_id=null
fee_rules=[]
components.len=1
components[0].meter=attempt_slot
components[0].unit_size=1
components[0].max_units=1
0 <= provider_unit_price_micros <= consumer_unit_price_micros <= MAX_SAFE_INTEGER
provider_max_amount_micros=provider_unit_price_micros
consumer_max_amount_micros=consumer_unit_price_micros
```

Price/Snapshot是单Job、单Reservation、单Claim的预算上限；Provider级`max_concurrent_attempts`只控制Pool/Offer可同时保留的
slot总量，不能把每个Job的`max_units`放大N倍。零价格仍需显式产品审批，不能由 validator 补默认值。Quote TTL 必须
在既有 v171/v223 合法范围 `30..=3600` 秒内；exact TTL、curve/source identity 和价格载荷仍未选择。
Offer `price_terms.curve_id=profile.price.curve_id`、`curve_version=Some(profile.price.curve_revision)`、
`instrument_id=profile.price.instrument_id=null`。Curve pair只属于Offer price terms；snapshot另投影
`price_source.{source_id,source_version,source_digest}`，两组产品事实不得省略、互换或强制相等。

Workload V1 要求非空、排序唯一的 `allowed_task_kinds` 与 `allowed_output_media_types`，并固定
`input_artifacts_allowed=false`、`model_allowed=false`、`result_artifact_required=false`、`checkpoint_mode=disabled`。
实际 task kind、runtime、output contract、retry/poll/cleanup 数值都是初始 profile 载荷的一部分，当前不得猜测。
`offer_policy`数组必须排序唯一，`sku.task_kind`必须属于`workload_policy.allowed_task_kinds`，Job data class与output
media type必须分别属于profile允许集合；public/account/project/data-class与policy revision都是首个inventory item必须
审批的产品载荷。
Enabled profile的current constructor还必须重证typed Provider capabilities包含所选task kind、accelerator kind、region和
全部allowed data classes。`offer_policy.policy_revision`为正safe integer；`public=true`时account/project arrays必须空，
`public=false`时两者至少一项非空，不能生成既不公开也无人可用的Offer。

`review_source.approved_by_user_id`必须是对exact profile ID/revision/review-material完成产品、经济与安全审批的真实authenticated user，
并由compiled catalog的typed review source audit；final profile digest通过填入的source digest传递绑定该evidence。该user不得等于
market service actor，也不得从Provider owner或caller猜出。
`review_source.approved_at<=profile.valid_from<=current constructor checked_at`。该字段进入profile digest，并由projection ABI逐字投影到legacy
publication `approved_by_user_id`；publication时间仍是本次checked-at，不是历史approved-at。

## 6. Transport、时间与 lease issuer

Transport policy 必须逐字投影已有 production task protocol 常量：

```text
ledger_json_max_bytes=524288
outer_request_max_bytes=262144
upstream_request_max_bytes=65536
response_max_bytes=262144
observation_max_bytes=262144
events_per_batch_max=256
exchange_ordinal_max=64
exchange_timeout_ms=15000
```

不得把 outer request 与 upstream request 合并为一个字段；除 `max_output_bytes` 外，transport byte limit 不进入
resource ceiling。

Profile 时间必须满足：

```text
valid_from <= checked_at < new_plan_accept_until
new_plan_accept_until <= expires_at < inflight_execution_valid_until
snapshot.quoted_at=checked_at
snapshot.observation_window_start=checked_at-1s
snapshot.observation_window_end=checked_at
snapshot.expires_at=min(checked_at+quote_ttl,new_plan_accept_until,expires_at)
offer.created_at=offer.valid_from=delivery_window.starts_at=checked_at
delivery_window.ends_at=offer.valid_until=inflight_execution_valid_until
price_terms.valid_until=offer.valid_until=inflight_execution_valid_until
plan.start.hard_deadline_at<=inflight_execution_valid_until
```

上述式子冻结关系，不选择首个 profile 的绝对时间或 deadline/retry 数值。既有Offer validator要求price terms覆盖
完整Offer有效窗；新Plan截止由snapshot/admission currentness单独限制。Tx-A 后的 sealed Plan 可在自身 hard deadline内继续
Tx-B；profile/snapshot new-plan expiry 不回溯撤销已seal Plan。

`deadline_policy`全部整数必须为正safe integer，且`retry_initial_backoff_ms<=retry_max_backoff_ms`。Job retry次数与
ELTP exchange ordinal是不同计数器，不得互相映射。Plan必须满足
`not_after < lease_expires_at < hard_deadline_at <= inflight_execution_valid_until`且
`lease_expires_at-not_after>=60s`。实际retry/poll/session/cleanup数值仍须产品与安全审批，network阶段的
effective deadline继续取本profile、claim/session/Secret、Plan/command/outbox、Reservation/Lease、fresh route/credential与
historical cleanup适用窗的最小值，不能只用profile延长既有V273/V278截止。

Lease issuer V1 固定 `issuer_policy_id=external_pool_service_managed_lease_issuer_policy_v1`、`issuer_revision=1`、
`authority_kind=external_pool_adapter_task_lease`、
`delivery_mode=eltp_commit`、`audience_kind=executor_id`、`required_scopes=[compute_attempt.execute]`、
`ref_prefix=external_pool_task_lease_ref_v1_` 与 `hint_prefix=external_pool_task_lease_hint_v1_`。
`issuer_policy_digest` 使用 `LEASE_ISSUER_POLICY_DOMAIN`，计算时自身字段置空。

每个 Attempt 的 ref/hint 分别使用 `LEASE_REF_DOMAIN`/`LEASE_HINT_DOMAIN` 对同一 exact key set 计算：

```text
issuer_policy_id
issuer_policy_digest
provider_id
provider_policy_revision
provider_digest
job_id
job_revision
job_digest
reservation_id
reservation_revision
reservation_digest
claim_id
claim_revision
claim_digest
attempt_lease_id
attempt_no
fencing_generation
plan_id
plan_digest
plan_seal_id
plan_seal_digest
route_authorization_id
route_authorization_revision
route_authorization_digest
route_seal_id
route_seal_digest
executor_id
stable_executor_binding_digest
fence_digest
issued_at
expires_at
```

`issued_at=Tx-B command.issued_at`，`expires_at=plan.start.hard_deadline_at`。结果是相应prefix加64-hex；ref必须
`<=512` bytes、hint必须`<=160` bytes（当前固定shape分别96/97 bytes）。调用方不得提交或轮换 raw ref/hint，route
credential也不得充当lease credential。Material中的`fence_digest`只消费未来Gateway/session/validator typed value；本页不定义
production fence派生，也禁止升格conformance fixture domain。

## 7. Allocation 与 catalog selection

Capacity allocation exact material：

```text
provider_id
provider_binding_id
provider_binding_digest
profile_id
profile_revision
profile_digest
capacity_scope
capacity_unit
allocation_total_units
```

`allocation_digest` 是 `CAPACITY_ALLOCATION_DOMAIN` 对该 material 的 domain-separated digest；`allocation_id` 固定为
`external_pool_capacity_allocation_v1_` 加同一 digest。Current profile authority 必须同时绑定 typed active external_pool
Provider 与 exact V249 provider-binding pair，不能只接 raw ID/digest。

Catalog owner 固定为计划中的
`compute_federation/external_pool_service_managed_admission/policy.rs`。未来必须提供 private-field、non-Clone、non-Serde
`CurrentExternalPoolServiceManagedMarketProfileAuthority`，只从编译进 server 的 immutable inventory、typed Provider/V249
binding 与 server `checked_at` 构造。选择规则是 exact 一项 enabled、未 revoked，且
`valid_from<=checked_at<new_plan_accept_until<=expires_at`；constructor还必须重算review-material并逐字审计typed approval
evidence的source identity/digest/approver/time。0或多项或approval漂移均失败关闭。

计划 inventory 在本阶段逻辑上为空且尚无源码，revocation set亦为空，因此 current authority 正向不可构造。添加第一项
enabled profile 必须：

1. 由产品/经济/安全 owner 对exact review-material digest审批全部未选择载荷，并提交typed approval evidence；
2. 填入该evidence source digest后提交byte-exact RFC8785-JCS profile JSON与final digest；
3. 更新本权威和 acceptance 的 payload 指纹；
4. 保留历史 item，且 V1 不允许 successor 或运行时可变配置；
5. 逐字满足 [admission receipt ABI](external-pool-service-managed-admission-receipt-abi-authority.md)，并与仍待冻结的
   Gateway/session/validator ABI、完整 V280 纵切源码同批集成，不能单独打开 fence。

## 8. 计划中的 Rust 边界

计划 symbols：

```text
ExternalPoolServiceManagedMarketProfileV1
ExternalPoolServiceManagedLeaseIssuerPolicyV1
CurrentExternalPoolServiceManagedMarketProfileAuthority
validate_external_pool_service_managed_market_profile
external_pool_service_managed_market_profile_from_json
external_pool_service_managed_market_profile_json_is_canonical
canonical_external_pool_service_managed_market_profile_json_and_digest
canonical_external_pool_service_managed_market_profile_review_material_digest
current_external_pool_service_managed_market_profile_authority
```

DTO 可做 deny-unknown parse/readback，但 authority 不得 Clone/Serde；owner-local constructor只能接 typed Provider/V249
binding 与 checked-at，不接 raw profile/capacity/price/ceiling/bool。Profile 纯 validator不得读 DB、时间、网络或环境变量。

本批不写这些 symbols，不注册任何物理migration/UDF/table且不预占280，不修改 V254 fence、worker、Store、API 或 runtime。静态审计不计入
实现 `passed/failed`。

## 9. 明确待产品选择的 payload

进入首个 enabled catalog 前仍须逐字批准：

- `max_concurrent_attempts` 与十项 resource ceiling 的九个整数和 network egress；
- profile 四个绝对时间、deadline/retry/poll/session/cleanup 数值；
- SKU/task kind/runtime family/version/precision、accelerator kind、region/SLA/verification/output media与Offer authorization；
- consumer/provider unit price与max amount、quote TTL、curve/source identity；
- owner/review source（含真实approver user）、workload allowlist 与正式 lease issuer policy 审批来源。

这些是产品、经济和安全决定，不是 canonical validator 的默认值。
