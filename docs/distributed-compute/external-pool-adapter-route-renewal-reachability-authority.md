---
title: 外部矿池 Adapter route renewal 与 production reachability 权威
status: current
reviewed_at: 2026-08-20
owners: backend, security, ai-economy
design_status: design_frozen
implementation_status: implementation_uncompiled
verification_status: source_review_only
---

# 外部矿池 Adapter route renewal 与 production reachability 权威

## 1. 唯一结论与当前现实

V278 冻结 external-pool active Provider 的 route credential/authorization/seal 续签、V274 active refresh、
default-off V273 worker/ingress 接线与恢复边界。它不重新激活 Provider，不把历史 V277 receipt 解释成
current route，也不开放 market fence。

本页合同已有对应 Domain、DDL、Store、migration、default-off worker、typed ingress与source-contract源码落盘，
当前严格为`design_frozen / source_written / source_review_only / implementation_uncompiled /
implementation_unrun`、`passed=0 / failed=0`。本批没有编译、测试、执行migration、SQLite、runtime或network；
sealed semantic ingress也没有production validator/Runner caller。V254 #13-#18仍 absolute deny，因此正常
Offer→Job→v211/v213 source不可达；Provider继续`registering`，V273 `eligible_rows=0`。源码存在不等于route、
V274 refresh、send、ACK、Lease或Runner已运行。

## 2. 权威顺序与不得形成的环

正向顺序只能是：

1. 从 durable V277 activation receipt、exact V274 sequence-1 genesis历史见证和 genesis route closure重建
   historical-route recovery authority；
2. 以 V277 historical root与live projected-active Provider恢复fresh V253 credential evidence；
3. 不依赖current V274/V268/V272，原子续签route并取得renewed-route current authority；
4. 只有current route存在后，才为一个candidate取得fresh V253、V268、V272；
5. 事务外执行active broker、Secret delivery、managed child与authenticated no-work；
6. final writer使用独立V274 refresh ordered plan，append/重读/commit并same-connection promote fresh V274；
7. 在一个显式`checked_at`同时重证current renewed route与promoted current V274，形成runtime carrier；
8. 该carrier才可进入V273 scoped claim、send、direct reconcile、poll与authenticated ingress。

禁止以下环：route renewal先要求current V274；active V253 recovery先要求current V274；V274 refresh先要求旧
V274 current；V274 receipt保存route receipt；historical route recovery直接进入dispatch；V273 receipt反向铸造
route或executor。

P0 circularity是`renew route→current V274→active preparation→current route`与`active V253→current V274→refresh→
active V253`，会使genesis/restart永久不可达；必须用historical V277+sequence-1 witness起步。P1 coupling是把current
V274 refresh identity写进V278 receipt或反向写route identity进V274，虽可排出写序，却会把两个独立head/TTL绑成
restart deadlock；两张receipt都只保留单向stable/historical join。

## 3. Store-private 类型与 ownership

以下名称属于V278 Store-private ABI，均不得进入HTTP/MCP/WebSocket/PC DTO：

- `ExternalPoolAdapterRouteRenewalReceipt`：owned canonical immutable receipt；
- `HistoricalExternalPoolAdapterRouteRecoveryAuthority<'tx, 'conn>`：只允许audit、predecessor recovery、
  cancel/reconcile/event cleanup；类型上绝不可dispatch；
- `PendingExternalPoolAdapterRouteRenewalCommit`：持有same-connection ordered plan与待postcommit readback identity；
- `CommittedExternalPoolAdapterRouteRenewal`：owned receipt identity与
  `ExternalPoolAdapterRouteRenewalDisposition::{Inserted, ExactReplay}`，不保存mutable status或current boolean；
- `CurrentExternalPoolAdapterRenewedRouteAuthority<'tx, 'conn>`：唯一可进入fresh v211/v213 producer的route型；
- `ReprovedExternalPoolAdapterRouteAndActiveSuccessorAuthority<'authority, 'tx, 'conn, 'runtime>`：current route、
  fresh V253/V268/V272与promoted current V274在同一`checked_at`的runtime carrier，供V273消费。
- `HistoricalExternalPoolAdapterTaskExchangeCleanupAuthority<'tx, 'conn>`：exact已durablesend/exchange/outbox identity、
  historical route recovery与cleanup horizon的组合，只允许cancel/direct reconcile/poll/event/terminal ingress。

冻结的Store入口为：

```text
historical_external_pool_adapter_route_recovery_authority_on(...)
external_pool_adapter_route_renewal_head_identity_on(...)
external_pool_adapter_route_renewal_decision_on(...)
renew_external_pool_adapter_route_on(...)
finalize_external_pool_adapter_route_renewal_after_commit_on(...)
require_current_external_pool_adapter_renewed_route_on(...)
Store::with_reproved_external_pool_adapter_route_and_active_successor(...)
Store::with_historical_external_pool_adapter_task_exchange_cleanup(...)
```

Currentness由Store在每次使用时重证；worker不得从receipt、诊断view、状态字符串或caller boolean构造上述类型。

Historical constructor与head/decision ABI exact冻结为：

```text
historical_external_pool_adapter_route_recovery_authority_on<'tx, 'conn>(
    transaction: &'tx Transaction<'conn>,
    activation_receipt_id: &str,
    expected_activation_receipt_digest: &str,
    activation_genesis_successor_receipt_id: &str,
    expected_activation_genesis_successor_receipt_digest: &str,
    checked_at: &str,
) -> Result<Option<HistoricalExternalPoolAdapterRouteRecoveryAuthority<'tx, 'conn>>>

external_pool_adapter_route_renewal_head_identity_on(
    transaction: &Transaction<'_>, provider_binding_id: &str,
    expected_activation_receipt_id: &str, expected_activation_receipt_digest: &str,
) -> Result<Option<(String, String)>>

external_pool_adapter_route_renewal_decision_on(
    transaction: &Transaction<'_>, provider_binding_id: &str,
    expected_activation_receipt_id: &str, expected_activation_receipt_digest: &str,
    checked_at: &str,
) -> Result<ExternalPoolAdapterRouteRenewalDecision>
```

head pair是owned `(route_renewal_receipt_id, route_renewal_receipt_digest)`：零行返回`None`，exact一个lineage head返回
`Some`，fork/multi-head/gap/root漂移报错。decision只有`Current { route_renewal_receipt_id,
route_renewal_receipt_digest }`与`RenewalRequired { predecessor_route_renewal_receipt_id,
predecessor_route_renewal_receipt_digest }`；两个predecessor必须同NULL或同非NULL，首次续签为NULL pair。renewable leaf
TTL不足/到期/被后继或route credential/authorization/capability/seal撤销可要求续签；stable activation root、executor、
service actor/delegation漂移或撤销直接报错。`Current`仍须调用`require_current...`；不得按error text决定续签。

builder完整signature为：

```text
build_external_pool_adapter_route_renewal_receipt<'tx, 'conn>(
    transaction: &'tx Transaction<'conn>,
    historical: &HistoricalExternalPoolAdapterRouteRecoveryAuthority<'tx, 'conn>,
    credential: &CurrentExternalPoolAdapterCredentialReattestationAuthority,
    decision: &ExternalPoolAdapterRouteRenewalDecision,
    evidence_checked_at: &str,
) -> Result<ExternalPoolAdapterRouteRenewalReceipt>
```

它只接受`RenewalRequired`，receipt ID/sequence/predecessor/idempotency与全部route leaf ID均由S1事务内生；worker/caller
identity完全排除。S2/S3/worker既不传也不构造receipt。pending result在commit前借用transaction/connection；
`CommittedExternalPoolAdapterRouteRenewal`及其identity/disposition在commit后owned，runtime carrier仍借用fresh authority。

`CurrentExternalPoolAdapterRenewedRouteAuthority`只暴露borrowed `receipt()`、`checked_at()`、
`effective_expires_at()`与provider binding、V277 receipt、sequence-1 genesis、activation root、executor/stable binding、
projection/v211 binding的`&str` getters，不能clone/mint。`effective_expires_at`为credential/auth/seal fresh-use canonical
minimum；active observation/runtime expiry还必须取`min(checked_at+15s,V250,V252,V253,V268,V272,route expiry)`。
Identity getter exact命名为`provider_binding_id()`、`provider_binding_digest()`、`activation_receipt_id()`、
`activation_receipt_digest()`、`activation_genesis_successor_receipt_id()`、
`activation_genesis_successor_receipt_digest()`、`activation_root_digest()`、`executor_id()`、
`stable_executor_binding_digest()`、`route_adapter_projection_id()`、`projected_v211_adapter_binding_digest()`。

## 4. 唯一 durable business object

V278 exact只新增：

```text
compute_external_pool_adapter_route_renewal_receipts
```

它是一张immutable历史表，`0 view / 0 revocation table`。不得新增mutable head、generic queue、session、Secret、
currentness projection或第七张V273表。schema与digest固定为：

```text
schema = compute_federation.external_pool_adapter_route_renewal.v1
receipt domain = ELON-EXTERNAL-POOL-ADAPTER-ROUTE-RENEWAL-RECEIPT-V1
id domain = ELON-EXTERNAL-POOL-ADAPTER-ROUTE-RENEWAL-ID-V1
policy domain = ELON-EXTERNAL-POOL-ADAPTER-ROUTE-RENEWAL-POLICY-V1
idempotency domain = ELON-EXTERNAL-POOL-ADAPTER-ROUTE-RENEWAL-IDEMPOTENCY-V1
canonicalization = rfc8785_jcs
digest_algorithm = sha256
```

精确77列如下；列名、NULL语义与顺序属于migration/source-contract ABI：

```text
route_renewal_receipt_id
route_renewal_receipt_schema
route_renewal_receipt_digest
route_renewal_receipt_json
canonicalization
digest_algorithm
provider_binding_id
provider_binding_digest
activation_root_digest
renewal_sequence
predecessor_route_renewal_receipt_id
predecessor_route_renewal_receipt_digest
activation_receipt_id
activation_receipt_digest
active_provider_id
active_provider_policy_revision
active_provider_digest
executor_id
stable_executor_binding_digest
projected_v211_adapter_binding_digest
route_adapter_projection_id
route_adapter_revision
route_adapter_digest
predecessor_service_actor_authorization_id
predecessor_service_actor_authorization_digest
predecessor_route_credential_id
predecessor_route_credential_revision
predecessor_route_credential_digest
predecessor_route_authorization_id
predecessor_route_authorization_revision
predecessor_route_authorization_digest
predecessor_route_seal_id
predecessor_route_seal_digest
activation_genesis_successor_receipt_id
activation_genesis_successor_receipt_digest
credential_reattestation_receipt_id
credential_reattestation_receipt_digest
service_actor_id
service_actor_authorization_id
service_actor_authorization_revision
service_actor_authorization_digest
route_credential_id
route_credential_revision
route_credential_digest
route_authorization_id
route_authorization_revision
route_authorization_digest
route_capabilities_json
route_capability_count
route_capability_set_digest
route_capability_0_id
route_capability_0_revision
route_capability_1_id
route_capability_1_revision
route_capability_2_id
route_capability_2_revision
route_capability_3_id
route_capability_3_revision
route_capability_4_id
route_capability_4_revision
route_capability_5_id
route_capability_5_revision
route_seal_id
route_seal_digest
authenticated_at
authorized_at
expires_at
cleanup_expires_at
evidence_checked_at
created_at
delegation_id
delegation_digest
renewal_policy_digest
renewed_by_actor_kind
renewed_by_service_actor_id
idempotency_material_json
idempotency_digest
```

每一行必须引用同activation root的exact V274 `successor_sequence=1`历史见证，通过
`activation_genesis_successor_receipt_id/digest`与V277 witness/root/provider exact join证明。该pair不加
`UNIQUE`，多次route renewal可复用同一genesis历史见证；它不要求该V274 row current、未撤销、同process或仍有
process seal。普通V274 refresh identity不得写入V278 receipt。

## 5. Lineage、stable binding 与 currentness

`renewal_sequence=1`时predecessor V278 pair必须NULL，predecessor route closure必须精确等于V277 genesis route；
sequence大于1时必须引用前一V278 receipt与其new closure。每个predecessor只允许一个successor，head count必须
精确为1；不得用`ORDER BY ... LIMIT 1`掩盖fork。

同一activation root永久固定：Provider binding/root、original V221 onboarding source、V249/V254 lineage、owner、
`executor_id`、`stable_executor_binding_digest`、projected v211 binding、projection Adapter revision/digest、稳定
`service_actor_id`与六能力ID/revision/set digest。V278只轮换actor authorization、credential version/current root、
route authorization与seal。logical Adapter digest仍只做lineage，不得替代projected v211 digest。

Historical recovery authority即使canonical、未过cleanup horizon，也绝不可用于fresh prepare/commit。Current renewed
route必须在一个显式`checked_at`重验live projected-active Provider、exact Adapter root、credential current root、
actor/delegation、TTL/revocation与unique receipt head。V274/V268/V272均不属于route currentness。

TTL不接受caller/env数值：builder从server-fixed renewal policy与fresh V253/actor/delegation上限计算canonical
`expires_at`，exact为全部fresh-use上限的minimum；`cleanup_expires_at`为全部cleanup上限的minimum，并强制
`evidence_checked_at < expires_at < cleanup_expires_at`。Renew-before阈值和两项maximum均承诺进
`renewal_policy_digest`；policy不exact即拒绝，不得由worker延长。Fresh prepare/commit只用`expires_at`，历史
cancel/reconcile/event/terminal ingress只可在`cleanup_expires_at`前。

## 6. Route renewal transaction 与 V254 18 fences

Route renewal固定在一个`BEGIN IMMEDIATE`内执行`11 INSERT + 1 credential-root CAS UPDATE`：

1. INSERT fresh service-actor authorization；
2. INSERT same credential ID的next version；
3. CAS UPDATE credential root到next revision/digest且保持`active`；
4. INSERT fresh route authorization；
5. 依ordinal `0..5` INSERT六个capability；
6. INSERT fresh route seal；
7. INSERT V278 receipt。

即actor 1 + credential version 1 + route authorization 1 + capability 6 + seal 1 + receipt 1 = 11 INSERT，另有
credential root CAS 1。事务不写Provider、Adapter、V274、V253、outbox、Pool、Offer、usage或settlement。任一写、
readback、FK、commit失败整体rollback；SQLite transaction不跨任何I/O。

V254矩阵只能如下：

| Fence | V277 freeze | V278 effective |
|---|---|---|
| #1 Provider adjacent-active UPDATE | V277 plan only | V277 plan only；V278 deny |
| #5 active Provider version | V277 plan only | V277 plan only；V278 deny |
| #6-#7 projection Adapter root/version | V277 plan only | V277 plan only；V278 deny |
| #8 actor authorization | V277 plan only | V277或V278 exact plan |
| #9 credential version | V277 plan only | V277或V278 exact plan |
| #10 route authorization | V277 plan only | V277或V278 exact plan |
| #11 six capabilities | V277 plan only | V277或V278 exact plan |
| #12 route seal | V277 plan only | V277或V278 exact plan |
| #2-#4 Provider insert/identity/kind | absolute deny | absolute deny |
| #13-#15 CapacityPool active | absolute deny | absolute deny |
| #16-#18 Offer draft/active | absolute deny | absolute deny |

因此V278不开放Provider、Pool或Offer。Credential-root CAS使用独立V278 trigger，不改变18项inventory。V271 active
branch只证明original V221 source经V277 registering→active bridge仍exact；它不得消费ordered plan或重新解释历史source。

## 7. Exact 四个 UDF 与 connection-local custody

V278 exact只有四个purpose：

1. deterministic receipt canonical validator：
   `elon_v278_external_pool_adapter_route_renewal_receipt_is_exact(json)`；
2. route renewal 12-step ordered plan：
   `elon_v278_external_pool_adapter_route_renewal_pending_plan_matches(...)`；
3. V273 reachability/ingress ordered plan：
   `elon_v278_external_pool_adapter_task_reachability_pending_plan_matches(...)`；
4. V274 refresh ordered plan：
   `elon_v278_external_pool_adapter_provider_active_successor_refresh_pending_plan_matches(...)`。

Canonical validator使用`SQLITE_UTF8 | SQLITE_DETERMINISTIC | SQLITE_INNOCUOUS`；三项plan UDF只能使用
`SQLITE_UTF8 | SQLITE_INNOCUOUS`，不得标deterministic。三份registry按purpose隔离，connection-local、ordered、
one-shot、type-preserving、RAII；wrong connection/order/arity/value、partial consumption、NULL/0/error、restart、
direct SQL或交叉代用全部失败关闭。

注册arity exact为：canonical=`1`、route renewal=`-1`、V273 reachability/ingress=`-1`、V274 refresh=`17`。
V274 registration symbol exact为
`register_external_pool_adapter_provider_active_successor_refresh_pending_plan_udf(connection: &rusqlite::Connection)
-> anyhow::Result<()>`。它在既有`v274_provider_active_successor_receipt_pending_seal` BEFORE INSERT中追加且不替代
canonical/process-seal条件；调用exact为：

```text
NEW.successor_sequence > 1 AND
elon_v278_external_pool_adapter_provider_active_successor_refresh_pending_plan_matches(
  'provider_active_successor_refresh', NEW.active_successor_receipt_id, NEW.receipt_digest,
  NEW.receipt_json, NEW.provider_binding_id, NEW.activation_root_digest, NEW.successor_sequence,
  NEW.predecessor_active_successor_receipt_id, NEW.predecessor_active_successor_receipt_digest,
  NEW.activation_target_updated_at, NEW.evidence_checked_at, NEW.created_at,
  NEW.observation_expires_at, NEW.process_custody_epoch_digest, NEW.process_custody_nonce_digest,
  NEW.process_custody_seal_digest, NEW.receipt_integrity_digest
) IS NOT 1
```

每个值保留SQLite type、byte length与bytes；trigger exact WHEN为上述`NEW.successor_sequence > 1 AND ... IS NOT 1`。
Sequence 1完全不调用/消费refresh registry，仍只受V277 activation plan与既有V274 canonical/process-seal门；UDF自身也
拒绝sequence<=1或NULL predecessor pair。
Route renewal新表trigger exact只有`v278_route_renewal_receipt_integrity`、`v278_route_renewal_receipt_lineage`、
`v278_route_renewal_receipt_no_replace`、`v278_route_renewal_receipt_no_update`、
`v278_route_renewal_receipt_no_delete`；credential-root CAS guard exact为`v278_route_credential_root_cas`。既有V254
trigger中只替换`v254_external_pool_candidate_service_actor_fence`、
`v254_external_pool_route_credential_fence`、`v254_external_pool_route_authorization_fence`、
`v254_external_pool_route_capability_fence`、`v254_external_pool_route_seal_fence`为显式V277-then-V278选择；其余13个
trigger不调用V278 plan。V273 outbound/ingress只在既有send-attempt、exchange-attempt、outbox transition与terminal
receipt/state-machine guards追加第3项plan，不删除任何既有root/currentness/immutability条件。

同名`v273_task_exchange_attempt_exact_authority`保留原fresh V253/target/companion/V272分支，并只为historical cleanup
追加第二分支：`reconcile|authenticated_events`必须逐字段继承原durable send的adapter/route/credential/verifier/
executor/fence/runtime/session roots；`cancel_no_start`必须由新cancel send/outbox经`subject_outbox_id`精确回溯原prepare
exchange。两类都要求V278 renewal或V277+sequence-1 genesis cleanup witness的COUNT总和exact为1且
`started_at < cleanup_expires_at`；live Provider当前版本仍须保持V277的stable adapter/config pair，但不要求旧route仍是
current adapter head，也不以delegation后续撤销抹掉已发送的历史事实。该authority trigger不调用plan UDF；同一INSERT
仍只由既有no-replace trigger消费一次`ExchangeAttempt` plan。

historical Accepted使用第3项registry的独立四写purpose，顺序exact为dispatch Actor→LeaseAuthority→Commit→
Application。V278只以同名替换追加receipt-backed historical分支到
`trg_compute_attempt_dispatch_actor_exact_source`、`trg_compute_attempt_lease_authority_projection`、
`trg_compute_attempt_commit_live_authority_v215`、`trg_compute_attempt_application_live_authority_v215`与
`trg_compute_attempt_application_commit_closure_v213`；legacy fresh分支逐字保留。historical分支必须精确闭合原prepare
send、direct/reconcile authenticated receipt、observation、accepted ACK、activation与唯一cleanup witness，并在严格cleanup
horizon内消费四项plan；它不能成为new send或market authority。

V254 #8-#12必须用显式`CASE`先尝试V277 plan，未命中才尝试V278 plan，不能依赖SQL布尔求值顺序。V271 source
trigger不调用plan UDF。V274 refresh guard只接受第4项；route renewal与V273不得消费或promote V274 seal。

## 8. Default-off worker、outbound 与六表边界

V278只接现有唯一环境开关：

```text
ELON_EXTERNAL_POOL_ADAPTER_ATTEMPT_DELIVERY_ENABLED
```

默认缺失或`false`。不得增加第二个worker、path、poll interval、ingress address、Secret或bypass env。该worker的
Store-private source stage可完整接线V277 private orchestrator、V253 recovery、route renewal、active preparation、
V274 refresh、scoped claim/send/poll/ingress，但绝不新增public API或caller-provided authority。

每次start-outbox-sourced `prepare|idempotent_commit|cancel_no_start` outbound固定同一transaction三项mutation：

1. INSERT既有`compute_attempt_start_send_attempts`首个send-attempt；
2. INSERT V273 `compute_external_pool_adapter_task_exchange_attempts`；
3. exact CAS既有outbox从`claimed`到`in_flight_unknown`，`state_revision+1`、`attempt_count+1`。

即`2 INSERT + 1 outbox CAS`；任一失败全部rollback，commit后才允许ELTP。V273 durable shape继续exact六表，
V278不新增第七表。首次direct reconcile source只能在send已durable且物理结果unknown后创建，固定
`reconcile_ordinal=1`；不得在send前预造reconcile row，也不得以第二次prepare/commit send替代reconcile。

Route rotation绝不修改旧command/outbox的route pair。Fresh work只能由current renewed route生成fresh v211/v213
command/outbox；已发送旧route继续按historical cleanup authority做reconcile/event，未发送旧route不得借新route续命。
`Store::with_historical_external_pool_adapter_task_exchange_cleanup`只接受`exchange_attempt_id + checked_at`，从durable
exchange派生全部receipt/route identity，caller不传V277/V274/V278 IDs。它可INSERT ordinal-1 reconcile poll、CAS claim
poll、以原send identity INSERT `reconcile|authenticated_events` V273 exchange/receipt/poll/event，或claim既有
`cancel_no_start` outbox并执行cleanup的2 INSERT+1 CAS；不得创建prepare/commit、command、outbox intent、remote execution
或route rebind。Direct reconcile不新建v213 send-attempt，也不CAS原outbox。

## 9. Authenticated terminal ingress、restart 与 replay

只有production session验证MAC、八roots、route/executor/fence、nonce/ordinal与exchange digest后才能形成terminal
ingress。Accepted terminal ingress必须在同一transaction append V273 authenticated receipt，并接入既有v211 ACK、
v185 activation与v215 accepted closure；任一子步骤失败全部rollback。V278不另建ACK、Lease、application、commit、
Runner或accepted表，也不开放public callback/listener。

Exact replay只readback同一receipt/attempt/ACK closure；同ID不同digest、第二terminal receipt、cursor fork或remote
sequence fork失败关闭。Commit前crash回滚；outbound commit后无receipt一律`remote outcome unknown`并从ordinal 1
direct reconcile。Restart后所有plan/session/V274 process custody失效；historical rows不消失，但新send必须fresh
current route、V253/V268/V272、active preparation与promoted V274。Cleanup/reconcile不得要求route仍为current head。

## 10. Admission blocker 与零经济效果

V254 #13-#18仍absolute deny，所以normal service-managed CapacityPool/Offer→Job→Reservation/Lease→v211/v213
producer在本批不可达。V278 source stage可以完整写入默认关闭接线，但以下动态证据明确后移到独立的
service-managed admission + Runner bridge批次：

- `eligible_rows>0`；
- 真实outbound/ELTP/ACK；
- accepted Lease/Runner；
- Pool/Offer/Job/Reservation/Attempt正向生产链。

Fixture、seed、临时删trigger、直接SQL、mock receipt或手工插row都不算正向验收。V278不写Pool、Offer、usage、
settlement、余额、冻结、付款或链上状态。未来accepted ingress会复用既有Claim/Job/Reservation/Lease状态机，但本批
不动态宣称其已运行。

## 11. 正式状态与后继门

当前正式结论仅为：V278 route renewal/reachability合同与对应源码已落盘，状态必须记录为
`design_frozen / source_written / source_review_only / implementation_uncompiled / implementation_unrun`、
`passed=0 / failed=0`、Provider=`registering`、`eligible_rows=0`。静态格式、文件规模、模块路径与合同文本审计
不等于compiled、migrated、runtime verified或production ready。

后继批次必须先编译并验证本页source，再由独立service-managed admission + Runner bridge打开#13-#18的最窄
业务门、实现sealed production validator并产生normal source。任何先制造eligible fixture、先出网、先接ACK或先
开放market fence的实现均为P0。
