---
title: 外部矿池 Adapter production semantic wire profile registry ABI 权威
reviewed_at: 2026-08-21
status: current
owners: backend, security, ai-economy
design_status: design_frozen
design_scope: external_adapter_production_semantic_wire_profile_registry_abi_v1
implementation_status: implementation_unwired
verification_status: design_review_only
---

# 外部矿池 Adapter production semantic wire profile registry ABI 权威

## 1. 唯一结论与状态边界

本页只冻结 external-pool Adapter production semantic wire profile 的 registry、purpose-specific approval evidence、
current/historical selection 与 application write 前 request authorization 元 ABI。它不选择任何 vendor request/response/observation
schema、字段、媒体类型、encoder、parser、authorizer、validator 或真实 profile/evidence bytes，也不创建 Rust、表、migration、API、
child、TLS exchange、Runner 或经济效果。

当前状态逐字为：

```text
vertical_slice_architecture=design_frozen
external_adapter_semantic_wire_profile_registry_abi=design_frozen
external_adapter_semantic_wire_profile_approval_evidence_abi=design_frozen
initial_external_adapter_semantic_wire_profile_approval_evidence_set=unselected
initial_external_adapter_semantic_wire_profile_inventory=unselected
external_adapter_semantic_wire_profile=unselected
current_external_adapter_semantic_wire_profile_authority=unconstructible
implementation=unwired/uncompiled/unrun
passed=0 failed=0
```

本页是 [Gateway/session internal ABI](external-pool-service-managed-gateway-session-validator-abi-authority.md) 的外部语义选择前置，
纵切顺序仍由 [V280 parent authority](external-pool-service-managed-admission-runner-authority.md) 约束。本页验收见
[registry ABI acceptance](external-pool-adapter-production-semantic-wire-profile-registry-abi-acceptance.md)。

## 2. 真源与不可替代边界

V249 只证明 exact registry release、implementation、capability 与文件根；V268 只证明 runtime compatibility；V272 concrete JSON
是 synthetic non-production fixture；V273 只保存 ELTP exchange/receipt 与 byte length/SHA；V258 只证明 TLS target。它们都不能替代
本页 semantic profile 或 approval evidence。Market-profile product/economy/security evidence 绑定另一 subject/domain，也不得复用。

在不修改 V273 carrier 的前提下，唯一 durable selector 固定为：

```text
registry_release_id
registry_release_digest
registry_release_material_digest
implementation_digest
operation_kind
```

同一 exact V249 release+implementation+operation 永久最多映射一个 immutable profile/evidence pair。任何 semantic 变化必须发布
新的 V249 release；禁止同 release profile revision 轮换、latest fallback、原地替换、删除 historical item，或把 semantic profile
digest 塞入 `adapter_registry_digest`、`task_protocol_profile_digest`、第九 session root、V273 六表或 Tx-A/Tx-B 新行。

当前 compiled profile/evidence catalog 与 current index 都 exact empty。Profile schema可实施不等于 profile已选择，canonical bytes也不
等于真实 authenticated approval。五类 positive operation 都必须有独立 pair 后才可形成完整 current set。

## 3. 常量、canonical 与 JSON 类型

常量逐字固定：

```text
PROFILE_SCHEMA=compute_federation.external_pool_adapter_production_semantic_wire_profile.v1
PROFILE_ID_PREFIX=external_pool_adapter_production_semantic_wire_profile_v1_
PROFILE_ID_DOMAIN=ELON-EXTERNAL-POOL-ADAPTER-PRODUCTION-SEMANTIC-WIRE-PROFILE-ID-V1
PROFILE_REVIEW_MATERIAL_DOMAIN=ELON-EXTERNAL-POOL-ADAPTER-PRODUCTION-SEMANTIC-WIRE-PROFILE-REVIEW-MATERIAL-V1
PROFILE_DIGEST_DOMAIN=ELON-EXTERNAL-POOL-ADAPTER-PRODUCTION-SEMANTIC-WIRE-PROFILE-V1
PROFILE_REVISION=1
APPROVAL_SCHEMA=compute_federation.external_pool_adapter_production_semantic_wire_profile_approval_evidence.v1
APPROVAL_ID_PREFIX=external_pool_adapter_production_semantic_wire_profile_approval_v1_
APPROVAL_ID_DOMAIN=ELON-EXTERNAL-POOL-ADAPTER-PRODUCTION-SEMANTIC-WIRE-PROFILE-APPROVAL-ID-V1
APPROVAL_EVIDENCE_DOMAIN=ELON-EXTERNAL-POOL-ADAPTER-PRODUCTION-SEMANTIC-WIRE-PROFILE-APPROVAL-EVIDENCE-V1
APPROVAL_SOURCE_KIND=external_pool_adapter_production_semantic_wire_profile_approval
APPROVAL_REVISION=1
APPROVAL_REVIEW_SCOPE=adapter_protocol_security_compatibility_v1
APPROVAL_DECISION=approved
APPROVAL_CONFIRMATION=confirm_external_pool_adapter_production_semantic_wire_profile_approval
CANONICALIZATION=rfc8785_jcs
DIGEST_ALGORITHM=sha256
MAX_JSON_BYTES=1048576
```

本页新 ID/digest 全部为：

```text
SHA256(domain UTF-8 || 0x00 || RFC8785-JCS(value) UTF-8)
```

输入是 UTF-8 RFC8785/I-JSON exact bytes；拒绝 duplicate/unknown/missing key、float、非 safe integer、非canonical string/number、
trailing bytes、额外whitespace与parse后JCS byte不等。Digest为64 lowercase hex；identifier/reference ID为去首尾空白、无control、
1..200 UTF-8 bytes；时间为canonical UTC nanos；revision/code/size/timeout均为`1..=9007199254740991` JSON safe integer，固定0值
字段仅允许本页逐字列出的`frame_flags=0`。

JSON type matrix固定：两种envelope的`profile_revision|approval_revision`为integer、`profile|approval`为object，其余top-level为
string；release binding与review source全为string；base protocol的revision/count/ordinal/四ceiling与operation的code/四ceiling/timeout为
integer、refs为object、其余为string；compatibility policy中两个`*_allowed`与`semantic_change_requires_new_registry_release`为bool，
其余为string；nested approval只有`semantic_wire_profile_revision`为integer，其余14项为string。Null/array/数字字符串均拒绝。

## 4. Profile envelope、ID 与 self digest

Profile envelope exact 7 keys：

```text
schema
profile_id
profile_revision
profile_digest
canonicalization
digest_algorithm
profile
```

Nested `profile` exact 5 keys：

```text
release_binding
base_protocol
operation
compatibility_policy
review_source
```

Profile ID material exact 7 keys：

```text
registry_release_id
registry_release_digest
registry_release_material_digest
implementation_digest
operation_kind
operation_code
profile_revision
```

`profile_revision=1`；`profile_id=PROFILE_ID_PREFIX || domain_digest(PROFILE_ID_DOMAIN,id_material)`。ID不含review source或
profile digest，以避免approval环；operation name/code必须是§6固定pair。

`profile_digest=domain_digest(PROFILE_DIGEST_DOMAIN,envelope)`，计算时保留`profile_digest` key并置空串。Profile review-material
同样保留完整7/5层shape，只把top-level `profile_digest`与nested `review_source.source_digest`同时置空，再使用
`PROFILE_REVIEW_MATERIAL_DOMAIN`。禁止删除blank key、用final digest代替review-material、普通serde SHA或无domain SHA。

## 5. V249 release 与 ELTP base protocol 投影

`release_binding` exact 9 keys：

```text
registry_release_id
registry_release_digest
registry_release_material_digest
adapter_id
release_version
implementation_digest
capability_set_digest
route_kind
supported_provider_kind
```

它们必须来自同一份deep-audited V249 release；`route_kind=server_adapter`、`supported_provider_kind=external_pool`，
`implementation_digest=declared_implementation_sha256=archive_sha256`；manifest digest与entry inventory digest是另两份独立root，
不得与implementation digest混同。Profile不得只凭五个scalar自行铸造V249 authority。

`base_protocol` exact 12 keys：

```text
frame_magic
frame_version
frame_flags
task_protocol_profile_id
task_protocol_profile_revision
task_protocol_profile_digest
operation_count
exchange_ordinal_max
semantic_body_max_bytes
upstream_request_max_bytes
upstream_response_max_bytes
observation_max_bytes
```

固定source truth为`frame_magic=ELTP`、`frame_version=1`、`frame_flags=0`、`operation_count=5`、
`exchange_ordinal_max=64`，四个global ceiling依次为`262144/65536/262144/262144`。Task protocol triple必须来自同一
retained V272 server-owned conformance/base profile；它只投影ELTP framing/bounds，synthetic semantic fixture不进入本页。

## 6. Operation record 与未选择 payload refs

每个 profile 只绑定一个operation。五个exact pair为：

```text
prepare=1
idempotent_commit=2
cancel_no_start=3
reconcile=4
authenticated_events=5
```

`operation` exact 16 keys：

```text
operation_kind
operation_code
semantic_request_mode
semantic_request_contract_ref
upstream_request_encoder_ref
pre_send_request_authorizer_ref
upstream_response_mode
upstream_response_contract_ref
child_observation_mode
child_observation_contract_ref
response_observation_validator_ref
semantic_body_max_bytes
upstream_request_max_bytes
upstream_response_max_bytes
observation_max_bytes
operation_timeout_milliseconds
```

每个`*_ref` exact 3 keys=`id,revision,digest`；它只引用future owner批准的immutable contract/implementation，不把ref字符串
当可执行authority。三个`*_mode`的actual值当前都unselected；未来每项只允许`canonical_json|opaque_bytes`。值为
`canonical_json`时必须deny duplicate/unknown/omitted/trailing并parse→JCS byte-equal；值为`opaque_bytes`时只允许Zeroizing
bytes+length/SHA进入对应approved contract/validator，raw bytes不得落Store。V272 fixture的JSON选择不能决定任何mode。

每个ref必须由其purpose-specific owner解析：fresh current按`id+revision+digest`命中exact 1 approved/current immutable contract或
implementation；durable historical按同一三元组命中exact 1 retained item。0/multi、digest/bytes drift、owner/source substitution或
current item替换historical ref都失败。所有ref在profile authority形成前必须解析为private-field non-Clone authority；悬空ref、仅shape
合法字符串或profile自身approval不能授权encoder/authorizer/parser/validator执行。

四个operation ceiling必须正且分别不大于§5 global ceiling；`operation_timeout_milliseconds<=15000`。Profile payload必须给出
全部refs、mode与bound的actual exact值，但当前五类payload全部`unselected`；fixture selector、placeholder、zero digest或待填ref拒绝。

`operation_timeout_milliseconds`必须进入Gateway单一absolute deadline：Tx-preflight以同一paired UTC/monotonic clock一次派生
`operation_deadline=min(既有全部适用cutoff,preflight_monotonic+selected timeout)`，把selected值与absolute cutoff写入owned plan。
Final reproof必须核同一profile值/cutoff，只能因elapsed或更早existing cutoff收紧remaining；ELTP BEGIN、gate2、validator与receipt共用该
absolute deadline，禁止child/TLS后重开relative timeout。Historical cleanup按sealed target operation的retained pair同样派生一次。

`compatibility_policy` exact 10 keys并固定为：

```text
selector_cardinality=exactly_one
fresh_selection=current_approved
durable_recovery_selection=retained_historical_exact
semantic_change_requires_new_registry_release=true
same_release_profile_rotation_allowed=false
latest_fallback_allowed=false
application_request_authorization=one_shot_before_socket_write
post_durable_authorization_failure=durable_unknown_reconcile
v273_carrier_change=none
task_session_root_change=none
```

`review_source` exact 6 keys=`source_kind,source_id,source_revision,source_digest,approved_by_user_id,approved_at`，只接受§7
purpose-specific evidence投影。

## 7. Purpose-specific approval evidence 与无环构造

Approval envelope exact 7 keys=`schema,approval_id,approval_revision,approval_digest,canonicalization,digest_algorithm,approval`。
Nested `approval` exact 15 keys：

```text
semantic_wire_profile_id
semantic_wire_profile_revision
semantic_wire_profile_review_material_digest
registry_release_id
registry_release_digest
registry_release_material_digest
implementation_digest
operation_kind
review_scope
decision
submitted_by_admin_user_id
submitted_at
approved_by_user_id
approved_at
confirmation
```

Approval ID material exact 3 keys=`semantic_wire_profile_id,semantic_wire_profile_revision,approval_revision`；revision固定1。
`approval_id=APPROVAL_ID_PREFIX || domain_digest(APPROVAL_ID_DOMAIN,id_material)`；`approval_digest`保留self key并置空后用
`APPROVAL_EVIDENCE_DOMAIN`计算。Evidence禁止携带final profile digest。

唯一DAG为：V249 roots+operation→profile ID→approval ID→profile双blank review-material→blank-self approval digest→
`review_source.source_digest`→final profile digest。Pair必须逐字满足：

```text
paired_evidence.approval.semantic_wire_profile_id = paired_profile.profile_id
paired_evidence.approval.semantic_wire_profile_revision = paired_profile.profile_revision
paired_evidence.approval.semantic_wire_profile_review_material_digest = recompute_review_material(paired_profile)
paired_evidence.approval.registry_release_id = paired_profile.profile.release_binding.registry_release_id
paired_evidence.approval.registry_release_digest = paired_profile.profile.release_binding.registry_release_digest
paired_evidence.approval.registry_release_material_digest = paired_profile.profile.release_binding.registry_release_material_digest
paired_evidence.approval.implementation_digest = paired_profile.profile.release_binding.implementation_digest
paired_evidence.approval.operation_kind = paired_profile.profile.operation.operation_kind
paired_evidence.approval_id = derive_id(paired_profile.profile_id,paired_profile.profile_revision,paired_evidence.approval_revision)
paired_profile.profile.review_source.source_kind = APPROVAL_SOURCE_KIND
paired_profile.profile.review_source.source_id = paired_evidence.approval_id
paired_profile.profile.review_source.source_revision = paired_evidence.approval_revision
paired_profile.profile.review_source.source_digest = paired_evidence.approval_digest
paired_profile.profile.review_source.approved_by_user_id = paired_evidence.approval.approved_by_user_id
paired_profile.profile.review_source.approved_at = paired_evidence.approval.approved_at
paired_evidence.approval.review_scope = APPROVAL_REVIEW_SCOPE
paired_evidence.approval.decision = APPROVAL_DECISION
paired_evidence.approval.confirmation = APPROVAL_CONFIRMATION
```

Purpose-specific issuance必须消费两个不同authenticated `admin|owner` session；submitter/approver user ID和各自时间只来自两次
sealed session与server clock，body/CLI/env/profile/compiled item不得提供。固定`submitted_at<=approved_at`，两user不同，且都不得
是service actor、Provider owner替身、fixture `local-owner`或caller。Current catalog只接收approved evidence；普通canonical JSON不证明
历史登录。V249 release的`registered_at`及所有ref owner的authenticated approval/existence time必须`<=submitted_at`；未来ref owner
缺少可审计时间就不得进入profile。当前没有issuance workflow或evidence instance。

## 8. Compiled catalog、current set 与 historical replay

计划catalog分为append-only retained profile/evidence pairs与build-scoped current index；均server-compiled，不查询deployment DB或
“latest”。Profile/evidence bytes、ID/digest与V249/operation binding不得删除、改写或重绑；current membership只控制本build的新send，
retained bytes永久保留。本页不宣称跨binary rollback的monotonic revocation；若以后需要anti-rollback，必须另冻epoch/tombstone authority。

Retained item的typed logical fields固定8项=`profile_id,profile_revision,profile_digest,profile_json,approval_id,approval_revision,
approval_digest,approval_json`。Current index entry固定11项=§2 selector五项+`profile_id,profile_revision,profile_digest,approval_id,
approval_revision,approval_digest`；它只能引用exact retained pair。两者不是新的wire JSON、DB row或digest preimage。

V249 release ID/digest含runtime registration identity/time，不能在源码中预猜。首项bootstrap顺序固定为受控V249注册→导出并审计
exact canonical release receipt→purpose-specific profile issuance/approval→checked-in retained catalog+current index→部署后按exact receipt重放；
不得从deployment DB选择latest后补写compiled item，也不得用placeholder ID先占位。

Current selector按§2五元组0/1收敛：0项返回`current...authority=unconstructible`；1项必须full-audit V249 current release、
retained V272 server-owned conformance/base profile、profile/evidence DAG、current index与operation；multi/split/same key different
bytes失败。完整positive task set要求五份release_binding及base_protocol triple/shape逐字相同、operation各exact 1；
缺任一项不能形成`CurrentExternalPoolAdapterProductionSemanticWireProfileSetAuthority`。

Fresh first-send分两阶段：短Tx-preflight在child/TLS前full-audit current V249/V272+five-set+target operation，只产owned non-authorizing
`PlannedSemanticWireSelection`；0/multi/missing时零child、零network、零outbound。该plan可跨await，但transaction-bound authority/DB
connection不可跨。建立child/TLS custody后、进入final outbound事务前，approved server builder必须从该plan与operation-specific durable material
唯一构造non-authorizing `PreparedExternalPoolAdapterSemanticRequest`；它线性拥有同一Zeroizing body、encoder/ref commitment及由
`prepare_external_pool_adapter_task_request`计算的`PreparedExternalPoolAdapterTaskRequest`/`request_digest`，尚不授ELTP BEGIN或写网。

Final outbound事务必须重新full-audit current V249/V272与exact same profile/evidence/refs/bounds及prepared commitment；漂移则rollback、
close/reap、zeroize prepared token、零ELTP/application write与零outbound。该事务必须把exact同一digest分别写入V213
send-attempt的`request_digest`与V273 exchange identity的`request_digest`并逐字相等；只有commit成功后，
`CommittedSemanticWireExchangeAuthority::StartSend(CommittedExternalPoolAdapterTaskOutbound)`、`CommittedRetainedSemanticWireSelection`与原同一prepared token才可线性
提升为第一道one-shot authorized body。不得在commit后另造第二份body或仅凭stored digest伪造token；gate2、validator与receipt ingress从此
不得重新查询current index。StartOutbox路径在paired V213 send-attempt+V273 commit前没有stored request digest；此时token丢失只等价于
rollback/drop，fresh必须重新current selection+prepare，historical cancel必须从retained target重新prepare。Paired commit一旦成功，token丢失
只可做canonical audit并保持durable unknown，禁止promotion、重进原BEGIN或盲重发，后续必须由state machine创建新的cleanup target intent。

Existing receipt replay从V273 exchange的operation与其exact historical route lineage重建V249 triple+implementation：有renewal时只接受
V278-renewed branch，无renewal时只接受V277+sequence-1 V274 genesis witness，两个branch必须恰有一个成立；并逐字
核`session.roots.task_protocol_profile_digest`及conformance run receipt ID/digest可追到paired base_protocol triple；同一五元组必须命中
exact 1 retained pair。0/multi/bytes drift或protocol mismatch都是integrity failure。

Historical cleanup的cancel/reconcile/authenticated-events可能是新的target operation。它必须从original durable lineage+sealed cleanup
intent的target operation选择对应retained pair，并核本次task-session base protocol；该pair与exact V278 cleanup authority/state machine
组合后才可通过两道application-write gate。Historical pair alone不授fresh Job/Plan/Start或任意operation；也不要求current index、
current V249/V272或未撤销。禁止按adapter ID、release version或latest profile回退，也禁止用新release解释旧lineage。

每个historical target operation同样先由exact cleanup authority+retained pair产non-authorizing prepared body/digest；caller不得提供digest。
Cancel在同一事务把digest原子写入V213 send-attempt+V273 identity。Reconcile/authenticated-events先写既有poll intent，再在claim/final
reproof时要求V273 identity等于poll stored digest；只有这两类poll intent可在尚无V273 attempt时由retained pair重构body并匹配stored digest
后继续。对应commit后才可用正确committed carrier+原prepared token promotion；attempt commit后丢token只由state machine写既有V273 cleanup
poll intent/row，不新增semantic-profile专属carrier、schema或row kind。

Post-commit retained pair缺失、漂移或protocol mismatch时必须零application write并保持durable unknown；只允许恢复同一canonical item后
继续cleanup。Cleanup horizon到期则隔离并进入人工处置，不得伪造no-start、释放Lease/Claim或换profile重试。

Replay不采时钟、不重铸ID/digest、不更换evidence/ref/mode/bounds。Current或historical authority、selected operation与five-set token均
private-field、non-Clone、non-Debug、non-Serde；canonical DTO本身不是authority。

## 9. 两道发送前授权与 response/observation validation

Owner-sealed `CommittedSemanticWireExchangeAuthority`固定为两variant：`StartSend(CommittedExternalPoolAdapterTaskOutbound)`只对应
prepare/idempotent_commit/cancel_no_start的exact V213+V273 paired commit；`PollExchange(CommittedExternalPoolAdapterTaskPollExchange)`只对应
reconcile/authenticated_events的claimed poll+V273 attempt commit。它private-field、non-Clone、non-Serde；operation与variant不匹配必须拒绝。

第一道分为prepare与commit后promotion：server-owned builder在final事务前从planned selection+command/outbox/route/executor/fence及
operation-specific durable material产上述non-authorizing prepared token；raw/caller/fixture body不得进入
`prepare_external_pool_adapter_task_request`。Final事务只消费其sealed commitment，commit成功后再以同一prepared token+committed retained
operation+对应`CommittedSemanticWireExchangeAuthority`产one-shot `AuthorizedExternalPoolAdapterSemanticRequestBody`。新的owner `begin_authorized` seam
必须消费该token；ELTP BEGIN完成后只返回一个non-Clone continuation，线性取回同一Zeroizing body/commitment/refs并与HostExchange共同
持有，不能复制、borrow后复用或公开拆分。Rollback/drop必须zeroize prepared token，commit前任何path都不能BEGIN或application write。

第二道位于child返回actual upstream request之后、broker TLS application socket write之前：purpose-specific authorizer消费上述唯一
continuation、profile的exact encoder authority、profile ID/revision/digest、selector五元组、operation、source kind/ID/digest、exchange-attempt
ID/digest、command/outbox/send-attempt/delivery-attempt、route/executor/fence/semantic-request digest、session/task-profile root与同一absolute
deadline，并由approved encoder/commitment重算expected upstream bytes或其length/SHA；actual request必须逐字或按approved
commitment等于expected，再绑定expected-response bytes与response-policy ref，产不可拆
`AuthorizedExternalPoolAdapterUpstreamRequest`。它线性拥有Zeroizing request bytes及expected policy，只能被TLS writer消费一次；
Host receipt的actual request length/SHA必须与token相等。

后验validator必须同时消费actual upstream response与child observation及outbound commit锁定的同一retained operation profile；只看
observation、只比length/SHA或只在response后选profile都不能替代两道pre-send authority。Raw request/response/observation只存在于
Zeroizing custody，不进入Store DTO。若durable outbound send-attempt已commit后任一道失败，只能保持unknown→reconcile；未另冻
write-not-started proof时不得记`local_never_sent`或释放Lease/Claim。

## 10. Ownership、唯一caller 与晋级门

建议独占owner：

```text
server/src/compute_federation/external_pool_adapter_production_semantic_wire_profile/
  types.rs canonical.rs validation.rs approval.rs catalog.rs selected.rs
server/src/store/compute_external_pool_adapter_production_semantic_wire_profile/
  current.rs historical.rs types.rs
server/src/compute_federation/external_pool_adapter_broker_tls/
  production_request_authorizer.rs production_validator.rs
```

Domain seams最宽`pub(crate)`且source-contract锁唯一worker/broker caller；Store historical/current seams只`pub(in crate::store)`。
不得提供raw constructor、generic `into_parts`、Clone/Serde authority、HTTP/env selector或第二Store sibling caller。Module aggregators、
V249/V272 historical audit、V273/V278 cleanup与Gateway worker属于共享integration owner。

进入source-written前必须同批提供：首个exact五operation profile set及五份真实evidence、全部ref背后的approved owner contracts与实现、
purpose-specific issuance provenance、current/historical catalog、两道pre-send builder/authorizer、response+observation validators、
production child loop、Gateway/worker/V273 ingress/recovery与source-contract。缺任一项时profile/current authority/validator/positive ELTP仍为0，
不得改migration、V273 carrier/session roots、打开V254 #13-#18或宣称Runner可达。编译、migration、SQLite、child/network与end-to-end证据
必须在架构阶段解除后分别执行；本页0/0静态审计不能替代。
