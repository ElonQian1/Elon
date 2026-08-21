---
title: 外部矿池受管 Gateway、task session 与 production validator 内部 ABI 权威
status: current
reviewed_at: 2026-08-21
owners: backend, security, ai-economy
design_status: design_frozen
design_scope: gateway_session_validator_internal_and_attempt_fence_abi_v1
implementation_status: implementation_unwired
verification_status: design_review_only
---

# 外部矿池受管 Gateway、task session 与 production validator 内部 ABI 权威

## 1. 范围、状态与唯一结论

本页是 [V280 service-managed admission + Runner 纵切](external-pool-service-managed-admission-runner-authority.md)
的第四个子 ABI。它只冻结：

1. Tx-A sealed Execution Plan 与 Tx-B Start Dispatch 的最终 owner-local builder 边界；
2. production Attempt/Lease `fence_digest` 的唯一无环派生与重放；
3. task-specific 八根 session、owned child/TLS custody 与单一 effective deadline；
4. 五类 operation 的 broker-owned concrete validator/context/output 与不可拆 exchange custody；
5. 唯一 Store/worker caller、source-contract 和 source-written 前置门。

当前轴状态逐字为：

```text
vertical_slice_architecture=design_frozen
market_profile_schema_abi=design_frozen
admission_receipt_canonical/physical_schema_abi=design_frozen
market_projection_identity_abi=design_frozen
gateway_builder_internal_abi=design_frozen
attempt_production_fence_abi=design_frozen
task_session_custody_abi=design_frozen
production_validator_internal_abi=design_frozen
external_adapter_semantic_wire_profile=unselected
initial_market_profile_inventory=unselected
implementation=unwired/uncompiled/unrun
passed=0 failed=0
```

`design_frozen`只修饰本页列出的内部 ABI，不表示外部 Adapter semantic wire schema、首个经济 profile 或完整
V280 纵切已经冻结。当前 migration 最大值仍为 V279，V280 table/UDF/trigger/Domain/Store/Gateway builder、五个
production validator impl、八根 production child、worker caller 与 Runner 均不存在；V254 #13-#18 打开数仍为 0。

本页不改写 V273 的 ELTP v1 framing、五个 operation code、八根 transcript/KDF、六表或 recovery 状态机；不改
[profile ABI](external-pool-service-managed-market-profile-authority.md)、
[admission ABI](external-pool-service-managed-admission-receipt-abi-authority.md)或
[market projection ABI](external-pool-service-managed-market-projection-identity-abi-authority.md)。也不新增 migration、
API、Ready、usage、settlement、Secret/payload/session 表或 public ingress。

## 2. Attempt production fence canonical ABI

### 2.1 常量与 exact material

常量逐字固定：

```text
schema = compute_federation.attempt_production_fence.v1
canonicalization = rfc8785_jcs
digest_algorithm = sha256
domain = ELON-COMPUTE-ATTEMPT-PRODUCTION-FENCE-V1
```

material 顶层 exact 7 keys，未知、缺失或 extra key 一律拒绝：

```json
{
  "schema": "compute_federation.attempt_production_fence.v1",
  "attempt": {
    "job_id": "...",
    "reservation_id": "...",
    "attempt_lease_id": "...",
    "attempt_no": 1,
    "shard_id": null,
    "fencing_generation": 1
  },
  "provider_id": "...",
  "executor_id": "...",
  "executor_binding_digest": "...",
  "route_binding_digest": "...",
  "execution_plan": {
    "plan_id": "...",
    "plan_digest": "...",
    "seal_id": "...",
    "seal_digest": "..."
  }
}
```

`attempt` exact 6 keys，`execution_plan` exact 4 keys。`shard_id`保留 Plan 的显式 `null|string`；首批 V280
只接受 `attempt_no=1`、`fencing_generation=1`，未来多 Attempt/代际分配必须另立单调 allocator authority，不能由 retry
counter、worker 或 Adapter 自增。

所有值只来自已提交且完整审计的 Tx-A Plan+seal：Provider、Attempt、route binding、Plan identity逐字读取 Plan；
`executor_id=plan.start.executor_id`；`executor_binding_digest=plan.start.selected_runtime.runner_digest`，并先证明其同时
等于 capability runtime runner digest、Offer runtime runner digest与historical V277 stable executor binding digest。
`route_binding_digest`是Plan内稳定 Adapter/config binding，不是 rolling route authorization、credential或seal。

digest 唯一公式为：

```text
fence_digest = lowercase_hex(
  SHA256(domain UTF-8 || 0x00 || RFC8785-JCS(material) UTF-8)
)
```

它明确排除 command/outbox/ACK/application/Lease/ref/hint、V273 exchange/delivery/session、route authorization/
credential、current head 与任何时间，避免 `fence -> ref/hint -> command -> fence` 或 route renewal 改写 fence。
它也不是 V254 trigger inventory、admission digest、random value或V268/V272 conformance fixture fence。

### 2.2 Typed custody 与 durable carrier

Domain owner 新增 private-field、non-Clone/non-Debug/non-Serde
`VerifiedComputeAttemptProductionFence`。唯一 crate-visible deep-audit seam建议固定为：

```text
verified_compute_attempt_production_fence_from_audited_sources(
  plan, seal, capability, historical_admission
)
  -> Result<VerifiedComputeAttemptProductionFence>

Store-private:
recompute_external_pool_service_managed_production_fence_on(connection, plan_id)
  -> Result<VerifiedComputeAttemptProductionFence>
```

Store函数必须加载`StoredPlan { plan, seal, capability }`、historical Offer、唯一V280 admission及其V277 stable root，先做
owner-specific full audit；Domain seam再重算Plan/seal/capability/admission canonical digest、Plan↔seal、Plan start↔capability
runtime↔admission stable executor与上述全部cross-field后才返回。只传Plan+seal无法证明capability runtime/V277 lineage，明确
禁止；也不能接caller-provided `fence_digest`或一组字符串。Store V280 orchestrator是唯一caller，source-contract必须锁住。

不为既有 v1 command、start outbox 或 LeaseAuthorityBinding 增加 `fence_digest`列。它们已保存 Plan pair，seal 对 Plan
唯一且可完整回读，因此 fresh/replay/reopen 均从同一 Plan+seal重算；Tx-B ref/hint material消费 typed fence，accepted
closure与Runner handoff再沿 Plan pair重算。V273 exchange attempt/receipt、reconcile poll与event poll是唯一直接持久化
`fence_digest`的既有 carrier；batch/event只沿FK lineage继承。全部路径必须证明等于重算值。若未来为性能增加冗余列，
仍只能是同一派生值，不能成为第二真源。

## 3. Tx-A final sealed builder ABI

Tx-A继续使用父权威冻结的独立 `BEGIN IMMEDIATE`、artifactless `3+2A` 行写入和0写 replay。本页冻结完整可达类型边界：

```text
execution_plan owner:
validated_external_pool_service_managed_execution_plan_inputs(
  material: &ExternalPoolServiceManagedExecutionPlanMaterial
)
  -> Result<ValidatedComputeAttemptExecutionPlanInputs>

Store owner:
produce_compute_attempt_execution_plan_on(
  connection, validated_inputs, recorded_at
)
  -> Result<ComputeAttemptExecutionPlanReceipt>
```

第一个函数必须是 `pub(crate)` deep-audit builder，直接构造 private `VerifiedComputeExecutionCapability`并返回最终
`ValidatedComputeAttemptExecutionPlanInputs { plan, capability, artifact_accesses=[] }`。它只消费 Store orchestrator 从
current admission、historical profile、fresh V274/V278 composite和exact Job/Reservation/Claim/Broker/Budget获得的sealed
sources；不得接 raw capability、numeric ceiling、route bool、caller time或未经审计的 Plan DTO。其 exact capability/runtime/
provenance/lease requirement/六项 route capability mapping继续以父权威 §7.1 为唯一真源。

第二个函数只在 Store 模块内可见，不自行开事务、不 commit、不采时钟、不复制现有 SQL。外层必须先按
`attempt_lease_id`与`(job_id,attempt_no)`查询：existing 只审 immutable Plan/capability/seal并0写返回；两把 identity
冲突拒绝；none 才读取 current sources、分配 IDs/times、调用 builder并写入。当前 public-ish facade不能先构造 token再
发现 existing，也不能嵌入 V278 callback中另开事务。

## 4. Tx-B final sealed builder 与 replay ABI

Tx-B 在新的 `BEGIN IMMEDIATE` 内首先只用 sealed Plan identity同时查询：

1. `(provider_id, activation_idempotency_key)`；
2. `(job_id, attempt_no)`。

两把 key 均0才允许读取 current business/fresh V274/V278、生成command/outbox/actor IDs与`issued_at`。任一命中必须
0/1且收敛同一row；existing分支只审 stored command、dispatch actor、prepare outbox、immutable Plan、historical
admission/profile与本页重算 fence，不能要求 current route、重采时钟或重铸ref/hint。split或同identity不同material拒绝。

owner-local builder链逐字固定：

```text
route_authority owner:
validated_compute_route_dispatch_sources_from_canonical_envelopes(
  adapter, credential, actor, authorization, seal
)
  -> Result<AuthorizedComputeRouteDispatchSources>

start_outbox owner:
validated_external_pool_service_managed_start_outbox_operation(
  sources: AuthorizedComputeRouteDispatchSources,
  material: &ExternalPoolServiceManagedPrepareMaterial
)
  -> Result<ValidatedComputeStartOutboxOperation>

attempt_gateway owner:
validated_external_pool_service_managed_start_dispatch(
  prepare_outbox: ValidatedComputeStartOutboxOperation,
  material: &ExternalPoolServiceManagedStartDispatchMaterial
)
  -> Result<ValidatedComputeAttemptStartDispatch>

Store owner:
prepare_compute_attempt_start_dispatch_on(
  connection, validated_dispatch, created_at
)
  -> Result<ComputeAttemptDispatchCommandReceipt>
```

现有 `validated_compute_route_authorization_from_canonical_envelopes` 保持通用既有 callers。新增route-owner wrapper先对
同一组fresh canonical envelopes完成一次现有deep audit；仅在成功后才把同一actor envelope封成owned
`AuthorizedComputeRouteAuthorization`与owned `AuthorizedComputeServiceActor`，装入field-private、non-Clone/non-Serde的
`AuthorizedComputeRouteDispatchSources`。该wrapper只能被本页start-outbox builder整体消费；不得公开通用`into_parts`、
raw getter或让caller分别选择route与actor，source-contract只允许同一Store orchestrator调用V280 wrapper。

attempt-gateway builder消费owned prepare operation、sealed Plan receipt、本页typed fence与historical issuer policy，完整构造
`ComputeAttemptStartActivationPlan`和最终dispatch。它必须锁：Provider-owner activation actor与V278 service actor分离；
server-derived activation idempotency；`issued_at>=seal.sealed_at`；exact command/adapter/Plan/lease times；profile-derived
non-bearer ref/hint；ref/hint material中的fence逐字等于typed fence。Store `_on`不自行开事务/commit/now，写序仍是
route replay/currentness→dispatch actor→prepare outbox→command→full readback，fresh固定3行、replay 0行。

三种`ExternalPoolServiceManaged*Material`均由所属Domain owner定义为field-private、non-Clone/non-Serde的`pub(crate)`
carrier；它们只承载已审计sources，不提供一串raw `String`、bool、time或digest setter。两个Store `_on` 的精确可见性是
`pub(in crate::store)`，接收现有transaction connection与外层唯一checked-at，不开transaction、不commit、不调用
`Utc::now()`。V280路径不得调用仍自行开事务/采时钟的既有facade。

## 5. Task-specific session 与 effective deadline

### 5.1 八根复用，不定义第九根

V280逐字复用 V273 exact八根顺序：policy、runtime launch profile、task protocol profile、upstream target、companion、
launch image、ephemeral task Secret delivery root、current conformance receipt。八个 argv prefix、raw32顺序、roots transcript
与KDF salt域均以 [V273 production authority §4](external-pool-adapter-task-protocol-production-authority.md)为唯一真源。

```text
roots transcript = SHA256(
  "elon.external_pool_adapter.task_protocol.production.session.roots.v1\0"
  || root1_raw32 || ... || root8_raw32
)

KDF salt = SHA256(
  "elon.external_pool_adapter.task_protocol.production.session.kdf_salt.v1\0"
  || root1_raw32 || ... || root8_raw32 || host_nonce32 || child_nonce32
)
```

这里没有额外separator、length、排序或hex文本。command/route/executor/fence是per-exchange binding，不进入八根；V280不得
复用 V278 no-work六根child、Secret delivery authority或已消费TLS channel。

### 5.2 Owned custody

短事务只能产 `ExternalPoolServiceManagedTaskSessionPreflight`：owned、non-authorizing、non-Clone/non-Serde，保存精确
command/outbox/source identity、八根选择与cutoff，不保存DB连接、transaction、Prepared installation、raw Secret或borrowed
current authority。随后独立async supervisor线性拥有 managed child、AuthenticatedSession、fresh TLS channel、effective
deadline及一个dedicated blocking session worker的join/cancel custody；`ExternalPoolAdapterTaskProtocolHost<'_>`与HostExchange
只能是该worker内对owned session的lexical borrow，不能成为跨await/self-referential字段。阻塞的host begin/complete、child
lifecycle/poll/wait不能直接占用Tokio executor。上述owned custody必须跨受控await，不能detach、Clone、写库或暴露generic
stream，并在所有terminal路径shutdown/kill/reap/join/zeroize。

final reproof后同一outbound transaction执行claim CAS，再写v213 send-attempt、V273 exchange-attempt和outbox
claimed→unknown CAS；因此总计2 INSERT+2 CAS，其中outbound leaf为2 INSERT+1 CAS。commit后才允许ELTP socket write。
DB/borrowed authority/Prepared/raw Secret不跨await；owned child/session/TLS custody可以且必须跨await。

### 5.3 单一 absolute deadline

新增non-Clone
`EffectiveExternalPoolAdapterTaskDeadline { absolute_monotonic: std::time::Instant, cutoff_utc_nanos: String }`。paired clock只在
preflight采一次；它只可收紧，不能Clone后延长或在Host begin重新开始15秒。session-core必须新增
`begin_before(request, delivery_attempt_digest, absolute_monotonic)`；现有内部`Instant::now()+Duration`的relative `begin`只保留
旧caller兼容，V280路径禁止调用。cutoff取适用窗口最小值：ELTP 15秒、claim、task-session/Secret、Plan/command/outbox、
Reservation/Lease/hard、prepare/commit适用的historical profile inflight、fresh credential/route/actor/seal及cleanup operation
适用的独立cleanup horizon；已durable reconcile/cancel/events不得因profile或fresh-route窗口结束而丢失cleanup。Offer/
snapshot new-plan expiry只约束Tx-A，不得回溯加入已seal Plan的send cutoff。

preflight、child/TLS建立、final reproof、outbound commit、BEGIN/application socket write、validator进入/返回和receipt timestamps
都用同一deadline的`remaining()`与UTC cutoff双检。若cutoff在DNS/TCP/TLS或child准备期间越过，已有channel/child必须关闭且
ELTP/upstream application request为0；DNS、TCP connect与TLS handshake每阶段timeout都取
`min(owner_stage_timeout, effective.remaining())`，不能用target自带timeout越过absolute。已经durable的unknown send只走
reconcile，不能回滚或盲重发。

## 6. 五类 concrete validator 内部 ABI

通用sealed trait不能让一个output同时声称五类语义。broker TLS owner必须在
`compute_federation/external_pool_adapter_broker_tls/production_validator.rs`定义五组concrete context/output，全部
private-field、sealed、Send、non-Clone/non-Debug/non-Serde：

```text
ExternalPoolAdapterPrepareValidator
  owns ExternalPoolAdapterPrepareValidationContext
  -> VerifiedExternalPoolAdapterPrepareSemantic
  -> VerifiedExternalPoolAdapterPrepareExchange
ExternalPoolAdapterIdempotentCommitValidator
  owns ExternalPoolAdapterIdempotentCommitValidationContext
  -> VerifiedExternalPoolAdapterCommitSemantic
  -> VerifiedExternalPoolAdapterCommitExchange
ExternalPoolAdapterCancelNoStartValidator
  owns ExternalPoolAdapterCancelNoStartValidationContext
  -> VerifiedExternalPoolAdapterCancelSemantic
  -> VerifiedExternalPoolAdapterCancelExchange
ExternalPoolAdapterReconcileValidator
  owns ExternalPoolAdapterReconcileValidationContext
  -> VerifiedExternalPoolAdapterReconcileSemantic
  -> VerifiedExternalPoolAdapterReconcileExchange
ExternalPoolAdapterAuthenticatedEventsValidator
  owns ExternalPoolAdapterAuthenticatedEventsValidationContext
  -> VerifiedExternalPoolAdapterEventsSemantic
  -> VerifiedExternalPoolAdapterEventsExchange
```

| operation | exact context | verified output唯一可表达的后继 |
|---|---|---|
| `prepare` | exchange attempt + send attempt + adapter binding | semantic view只能验证Store-built accepted ACK+remote observation；当前不接受final rejected |
| `idempotent_commit` | exchange attempt + send attempt + authenticated remote subject | semantic view只能验证Store-built first event poll |
| `cancel_no_start` | exchange attempt + send attempt + authenticated remote subject | semantic view只能验证Store-built first reconcile poll；cancel ACK绝不直接释放/no-start |
| `reconcile` | exchange attempt + exact reconcile poll | sealed semantic四分支分别只验证Store-built successor reconcile、accepted prepare、terminal no-start或first event poll |
| `authenticated_events` | exchange attempt + exact event poll | semantic view只验证Store-built batch、0..256 ordered events与optional successor poll |

Context必须按值私有持有上述exact envelopes，构造后不能替换或复用；validator的`validate(self, response, observation)`消费
self，不再接可借用/可换配的`&ExactOpContext`。Verified output只是不可伪造的semantic view，不生成Store-owned ID/time/digest；
V273/V211/V213 envelope仍由同一transaction的Store factory派生并交给view校验。每个operation返回独立
`Verified...Exchange`，私有持有HostReceipt+对应view，只能被唯一Store ingress以同步`consume_once`借用一次；删除/收窄
当前pub(crate)通用`into_parts`，禁止跨transaction拆分、重组或保存raw observation。

relay必须把实际TLS upstream response与child observation同时交给concrete validator：

```text
validate(self, upstream_response: &[u8], child_observation: &[u8])
```

当前只调用`validator.validate(observation)`的形状不足以证明response↔observation一致。opaque upstream response可保留
vendor bytes，但其exact length/SHA必须与observation、HostReceipt和V273 receipt一致；raw response/observation只存在于
Zeroizing buffer，Store永远收不到bytes。

size ceiling沿V273/V272不变：ELTP request material 262144 bytes、实际upstream request 65536、upstream response
262144、semantic observation 262144；exchange ordinal 1..64，单次总exchange不超过15秒。operation code逐字仍是
`prepare=1,idempotent_commit=2,cancel_no_start=3,reconcile=4,authenticated_events=5`。

## 7. 尚未冻结的 external semantic wire profile

仓库当前没有production Adapter的request body、actual upstream response与child semantic observation的获批schema/domain/key
set；唯一具体JSON来自V272 synthetic fixture，禁止升格。故本页不发明统一4-key envelope，也不声称五类外部DTO已冻结。

未来registry-owned `ExternalPoolAdapterProductionSemanticWireProfile`至少必须绑定 Adapter registry ID/revision、implementation
digest、operation、request encoder、upstream response parser或opaque policy、child observation schema/canonical domain、大小/
时间界限与版本兼容。若使用JSON，必须UTF-8 I-JSON、deny duplicate/unknown/omitted/trailing、显式null、safe integer、
parse→RFC8785-JCS后byte-equal；若vendor response非JSON，只允许opaque bytes+length/SHA并由canonical child observation证明
其解析结果。五类exact key set与approved payload只能由该profile提供。

当前prepare rejected还缺V278同事务receipt→observation→delivery-observed→rejected ACK→prepare-rejected no-start closure，因此本
ABI只接受accepted prepare；rejected bytes必须失败关闭/进入既有unknown→reconcile，不能直接activation、Lease、commit或释放。
未来若开放rejected分支，必须先另冻并实现该原子closure。

在这个semantic wire profile与首个market profile inventory都未选择前：五个validator impl、production child protocol loop、
eight-root task session authority、Gateway/worker caller与positive ELTP均不可实现；这不是用fixture、source-contract或手写JSON可绕过的门。

## 8. Ownership、唯一caller与晋级门

建议独占owner：

- fence Domain：`compute_federation/attempt_production_fence/{types,canonical,validated}.rs`；
- Gateway final builders：`execution_plan/validated.rs`、`route_authority/validated.rs`、`start_outbox/validated.rs`、
  `attempt_gateway.rs`；
- worker custody：`compute_federation/external_pool_adapter_task_worker/{runner,session,recovery}.rs`；
- broker validator：`compute_federation/external_pool_adapter_broker_tls/production_validator.rs`；
- task production child/session：既有`external_pool_adapter_task_protocol_production` owner与Linux supervisor owner。

共享integration owner还包括相应module aggregators、Store plan/dispatch `_on` kernels、V273 task-delivery ingress/outbound、worker
`lifecycle.rs/cycle.rs/report.rs`。Domain builder只能`pub(crate)`且每次full audit；Store `_on`只能`pub(in crate::store)`；
source-contract必须证明唯一production path是worker selector→Tx-A→Tx-B→V278 source scan→task preflight→outbound→exchange→
同transaction ingress。任何raw/unchecked constructor、第二caller、test fixture、HTTP/env payload或generic callback均拒绝。

本批仍只写文档。进入`source_written/source_review_only`前必须同时满足：首个approved market profile inventory、external semantic
wire profile、完整Domain/DDL/Store/owner kernels、Gateway final builders、production fence、8-root child/session、5 validator impl、
worker caller、V273 ingress/Runner/recovery与全量source-contract同一feature branch落盘；届时才读取实际migration最大值选择next-free
编号，并仍保持runtime gate default-off。编译、migration、SQLite、child/network与positive Runner动态证据分别验收，不能由本页0/0
设计审计推导。

验收矩阵见 [对应 acceptance](external-pool-service-managed-gateway-session-validator-abi-acceptance.md)。
