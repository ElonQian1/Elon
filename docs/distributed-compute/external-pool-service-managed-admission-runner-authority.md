---
title: 外部矿池 service-managed admission 与 Runner bridge 权威
status: current
reviewed_at: 2026-08-21
owners: backend, security, ai-economy
design_status: design_frozen
design_scope: end_to_end_vertical_slice_architecture_only
implementation_status: implementation_unwired
verification_status: design_review_only
---

# 外部矿池 service-managed admission 与 Runner bridge 权威

## 1. 唯一结论与当前事实

V280 冻结 external-pool 从受管市场准入到单个 artifactless Attempt 正向执行的最小完整纵切架构。它不是一批已写
源码，也不是 migration 280 已注册：当前全局 migration 最高仍为 V279，仓库没有 V280 table、UDF、trigger、
Domain、Store、production validator 或 Runner 实现。

V254 #13-#18 继续 absolute deny；V278 worker 继续 default-off，Provider=`registering`、
`eligible_rows=0`、`delivery_attempted=false`。现有 V273/V274/V277/V278 私有内核、fixture、历史 migration
证据或 source review 都不能证明正常 Offer、Job、ACK、Lease 或 Runner 可达。本页状态必须分轴读取：

```text
vertical_slice_architecture=design_frozen
market_profile_schema_abi=design_frozen
initial_profile_inventory=unselected
admission_receipt_semantic_contract=design_frozen
admission_receipt_canonical_abi=design_frozen
admission_receipt_physical_schema_abi=design_frozen
market_projection_identity_abi=design_frozen
admission_receipt_table/migration/source=absent
implementation=unwired/uncompiled/unrun
passed=0
failed=0
```

V280 只有在下列边界同批闭合后才允许进入 source-written 阶段：

1. server-owned market profile 提供可信容量、价格和数值执行上限；
2. service-managed Pool、供给账本、Offer 与 immutable admission receipt 在一个原子事务中成立；
3. Execution Plan 与 Start Dispatch 两段 Gateway 都能从 sealed authority 构造并恢复；
4. 独立 task session、封闭 production semantic validator、post-commit ELTP exchange 与 authenticated ingress 接通；
5. timeout、reconcile、cancel/no-start、event poll 与 restart recovery 沿 V278 账本闭合。

禁止把 admission-only、validator-only、构造器-only 或把 worker 的零候选报告改成非零当作 V280 实现。

## 2. 为什么不存在更小的正向切片

只开放 V254 market fence 会产生可售 Pool/Offer，却仍没有可构造的
`ValidatedComputeAttemptExecutionPlanInputs`、`ValidatedComputeAttemptStartDispatch`、production validator、
task session 或 Runner，形成“市场可售但永远不可派发”的错误状态。只补 validator 则没有 caller、session、
claim、outbound 或 receipt ingress；只补 Gateway 构造器则没有可信 capacity/price/numeric ceiling source。

user_node Ready 也不能替代本阶段：其真实前置仍是 Windows 动态证据、production VFS/process owner、真实 opened
projector 与独立 v15；现有 v14 是 blocked-only。计量与结算又必须消费真实执行事实。因此 V280 不伪造 Ready，
不跳到经济效果，也不为文档预占一个空 migration。

## 3. 版本与交付规则

“V280”是当前架构阶段名。未来开始实现时必须重新读取全局 migration 最大值；只有 280 仍为空且完整纵切可以
同批落盘时，才注册物理 migration 280。若编号已被占用，物理 migration 必须顺延并同步所有代码合同，不能覆盖、
重排或借文档保留旧编号。

第一批实现不得只注册空表或先打开 fence。migration、pending-plan UDF、替换 trigger、Store writer、Gateway、
validator、Runner 与 source-contract 必须在同一 feature branch 内可静态审计；编译和动态验收恢复后再决定是否合并。

## 4. Server-owned market profile

Profile 的 exact schema、JCS/SHA-256、字段集合、结构性约束、allocation/ref/hint 派生和 empty-inventory 失败关闭规则
由 [market profile authority](external-pool-service-managed-market-profile-authority.md) 与
[acceptance](external-pool-service-managed-market-profile-acceptance.md)唯一维护。父纵切只消费 sealed typed profile
authority，不重复定义 canonical keys/domain。

当前 `initial_profile_inventory=unselected`：仓库没有可合法升格的价格、容量、SKU/runtime、ceiling 或时限载荷，
因此 current profile authority 正向不可构造。只有 byte-exact profile JSON/digest、产品/经济/安全审批和完整纵切源码
同批落盘后才能加入第一项 enabled inventory；Profile 本身不直接铸造任何市场、执行或经济对象。

## 5. Immutable admission receipt

计划中的唯一 durable V280 对象是 `compute_external_pool_service_managed_admissions`：一张77列 immutable
`WITHOUT ROWID`表，0 mutable head/view/revocation/queue/session/Secret/payload表。Exact schema/domain、6-key envelope、
7组/72个direct group keys及单元素25-key bucket、ID/request/integrity/receipt digest、列序、parent keys、FK/UNIQUE/CHECK、
bucket inventory、
replay与readback全部由 [admission receipt ABI authority](external-pool-service-managed-admission-receipt-abi-authority.md)
及其 [acceptance](external-pool-service-managed-admission-receipt-abi-acceptance.md)唯一维护。

当前只完成设计冻结：table、migration、UDF、trigger、Domain与Store source均不存在；任何物理registration仍为0，阶段名不预占280。
Pool/ledger/Offer/publication/snapshot 的确定性身份、legacy owner mapping与single checked-at由
[market projection identity ABI](external-pool-service-managed-market-projection-identity-abi-authority.md)及其
[acceptance](external-pool-service-managed-market-projection-identity-abi-acceptance.md)维护；design freeze仍不能单独打开writer/fence。

Currentness 只能只读派生：Provider只有唯一genesis receipt且未过期，server catalog中同一 market profile revision/digest仍
current、未撤销且在有效窗内，Provider exact active且仍绑定同一V277 stable activation root/provider binding/executor/adapter
lineage，Pool active，Offer active且其revision/digest/window current，绑定的既有v171 price snapshot exact且未过期。
不存在可 UPDATE 的“current admission”行。
Exact replay必须在读取 current source 前按request/receipt identity零写回读；fresh只允许无既有admission的Provider，
随后重证全部live roots。相同Provider的不同第二请求拒绝，不得新建sequence 2。不同Provider可以各自消费同一policy
revision明确授权的一份per-provider allocation；它们的allocation ID/digest必须不同，且总供给按各自receipt逐份审计。
V280没有global remaining-capacity head，任何实现都不得把该scope解释成共享余额。

Admission内的V274/V278 pair只证明准入时使用了当时current的runtime/route，不参与后续head equality。V274 refresh或V278
renewal后，receipt保持historical audit；每次Plan/Start必须独立取得fresh V274+V278 composite，并只把其stable V277 root、
provider binding、executor和adapter lineage与admission逐字对齐。Receipt、historical pair或admission currentness都不能铸造
route/runtime authority。这一分型禁止把短TTL head耦合进长期market admission。

首批明确不实现admission successor。Profile更新、撤销或过期会使genesis失去currentness，但不会自动创建第二Pool/Offer或
重复供给。未来若需要sequence>1，必须另立authority，在同一事务先退役旧Offer/Pool、冲销或迁移供给账本与capacity，
再创建successor；不得只追加第二份active admission。

唯一production caller是既有external-pool task worker cycle内的Store-private bounded stage。新gate固定为
`ELON_EXTERNAL_POOL_SERVICE_MANAGED_MARKET_ENABLED`；unset/false时profile read、market/Gateway candidate scan、plan install、
Tx-A、Tx-B与market mutation都必须为0。true时也不得跳过下列固定顺序：

1. 先完成V278 active preparation；
2. replay或稳定选择最多一个`provider_kind=external_pool`、V249 binding exact、current V277/V274/V278 roots成立且没有V280 genesis admission的active Provider，并原子提交market admission；
3. 分别计算Plan-recovery与fresh-Job候选集，再由worker-owned、non-authorizing lane turn在两类间有界轮转，每cycle最多选择一个Gateway work item；
4. 选中recovery时要求Plan自身provider kind为external_pool、可反查唯一historical V280 admission、未过`plan.not_after`、已有sealed Plan但没有exact dispatch command，fresh reproof后只执行Tx-B；选中fresh时要求current external-pool admission下的normal reserved Job+active Reservation+held Claim+budget且没有durable Plan，提交Tx-A后在新事务fresh reproof并执行Tx-B；
5. 最后才运行既有V278 task delivery source scan、session/claim/exchange及其cleanup/recovery；delivery本身仍受独立default-off gate控制。

Worker lifecycle必须分别严格解析新market gate与既有`ELON_EXTERNAL_POOL_ADAPTER_ATTEMPT_DELIVERY_ENABLED`，任一为true才
initialize/spawn同一个bounded worker并取得所需runtime custody；不能继续只用旧gate决定线程是否存在。四种组合固定为：两者
false不spawn且两类call均为0；market=true/delivery=false只运行active preparation、admission与Gateway，durable command/outbox保持
pending且task claim/task-delivery ELTP request为0；market=false/delivery=true只运行既有V278 preparation/source/session/cleanup，market/Gateway call为0；
两者true才运行完整纵切。单gate关闭不能让另一gate的合法恢复路径失去production caller。

Active preparation失败不能以`?`提前终止整个cycle：它只把本轮admission、Tx-A/Tx-B与fresh send lane标成结构化deferred，
delivery gate为true时仍必须运行不要求current route/V274的historical cleanup selector、reconcile/cancel/event/terminal ingress；
只有cleanup lane自身失败才报告该项失败。Selected Provider的admission/profile/market transaction失败也只释放本轮admission slot，
不得阻断既有admission的Plan recovery/fresh Gateway或historical cleanup。

Tx-A提交与Tx-B开始之间的crash由下一cycle的第3步恢复，不能回到fresh Tx-A或生成第二Plan。Gateway candidate失败只能形成
结构化deferred/error并释放本轮slot，不得回滚已提交admission，也不得阻断historical cleanup、poll recovery或后续Provider/Job。
生成的market admission actor、confirmation、idempotency key与Gateway identity都由server派生，不能来自env、HTTP、Provider owner或Job caller。

`external_pool_adapter_task_worker/report.rs`必须随lifecycle/cycle一起归集成owner。报告至少区分active preparation、admission、
Gateway A、Gateway B、delivery与historical cleanup各stage的`disabled/no_candidate/selected/committed/deferred/error`，并保留真实
eligible count与delivery attempted；`eligible_rows=0`不能让deferred/error静默不记录。报告不得携带Provider/Job/Plan ID、Secret或raw payload，
也不能用改常量伪造正向证据。

Admission、Plan-recovery与fresh-Job三类selector都不能使用不可变最老行永久`LIMIT 1`。每类在同一短事务以完全相同
predicate计算candidate count并稳定排序，再用server UTC minute slot modulo count选择offset；caller不能传Provider、Job、Plan或slot。
Gateway的process-local lane turn在recovery/fresh两类都非空时每次eligible arbitration后必切换，即使选中项deferred/error也切换；
启动时只用server UTC minute parity初始化，不能由env/caller控制，且不授予任何DB写权限。Plan crash recovery与fresh Job都至多等待
一个对方lane机会；不得以“recovery优先”形成永久队头。
Exact-ID replay绕过slot。Selection只提供hint，Tx-A/Tx-B仍在各自IMMEDIATE transaction重证全部sources并依靠现有unique identity
收敛并发；loser只能exact readback或defer，不能把候选行本身当authority。
三类predicate都必须在首层排除`user_node`；V279 binding表不是fallback source，不得被selector读取、计数、写入或产生effect。

## 6. 原子 market transaction 与 fence inventory

唯一 service-managed market writer 使用一个 `BEGIN IMMEDIATE`，按下列顺序完成并 exact readback 后才 commit：

1. INSERT registering CapacityPool projection；
2. INSERT pool version 1；
3. UPDATE Pool projection并保持`registering`，闭合version projection；
4. INSERT N 个 capacity buckets；
5. INSERT supply ledger transaction；
6. INSERT 2N 条守恒 ledger legs；
7. CAS N 个 bucket projections；
8. INSERT pool lifecycle activation event；
9. CAS Pool `registering→active`；
10. INSERT draft Offer projection；
11. INSERT draft Offer version 1；
12. INSERT active Offer version 2；
13. CAS Offer `draft→active`；
14. INSERT publication receipt；
15. 由server profile确定性派生并INSERT既有v171 `compute_price_snapshots`；
16. INSERT V280 admission receipt；
17. exact readback 全部对象与 currentness，随后 commit。

正常路径只需要把现有 V254 四个 trigger 的五次命中追加为 ordered pending-plan 分支：

- `v254_external_pool_capacity_pool_update_active_fence` 一次；
- `v254_external_pool_offer_insert_market_fence` 一次；
- `v254_external_pool_offer_version_market_fence` 两次；
- `v254_external_pool_offer_update_market_fence` 一次。

`v254_external_pool_capacity_pool_insert_active_fence` 与
`v254_external_pool_capacity_pool_version_active_fence` 不属于正常写序，必须继续 absolute deny。不得为了宣称
“6/6 opened”扩大入口。

计划 UDF 名为
`elon_v280_external_pool_service_managed_admission_pending_plan_matches`：variable arity、非 deterministic、
`SQLITE_UTF8 | SQLITE_INNOCUOUS`、connection-local、ordered、one-shot、type+bytes exact fingerprint、RAII discard。
它只能由唯一 Store-private orchestrator 安装。

除四个 fence 的原名替换外，还要为 registering pool root INSERT、pool version INSERT、保持registering的projection
UPDATE、bucket INSERT、supply transaction/legs、bucket CAS、lifecycle event、publication、v171 price snapshot与
admission receipt保留或追加exact ordered guard。第3步projection UPDATE使用独立plan kind，只允许status不变且version/
timestamp projection exact；它不能复用只在active CAS触发的V254 #14 permit。任何失败、遗漏、wrong-order、额外写入、
跨connection、未完全消费或rollback都不得留下permit。

Price snapshot 必须复用既有v171 Registry及其Store-private `register_compute_price_snapshot_on`，不得建平行表。snapshot/
quote identity、components、max amounts、source、TTL与时间公式只取
[profile子权威](external-pool-service-managed-market-profile-authority.md)和本次admission/Offer sealed material。Publication真实
approver、snapshot/quote legacy IDs/times、shared checked-at及owner helper已由projection identity ABI冻结设计；source
observation window固定为`[checked_at-1s,checked_at]`。Tx-B fence仍归未冻结Gateway/session/validator ABI。实现不得让各既有
facade分别调用`now()`、随机ID或猜值。
既有`price_snapshot_effect=none`保持不变；snapshot是其后的独立受管写入。所有既有snapshot writer，包括public
Offer-owner facade与v223四眼admin curve application，都必须拒绝external_pool；只有持有本次ordered plan的V280
Store-private `_on` writer可调用底层registry kernel，任何caller/admin都不能提交raw price。
首批每份genesis admission只生成这一份initial snapshot，不实现snapshot refresh；其过期会停止新Job/Plan admission。
后续持续spot报价必须另立profile/snapshot successor authority，不能覆盖v171历史行或借同一quote ID换价。
Admission table必须对price snapshot ID/digest建立唯一exact binding，使任何sealed Plan都能从自身snapshot pair反查唯一
historical admission/profile；0或multi match失败关闭。

现有 owner-facing bucket create、supply add/withdraw、Offer create/revise/revoke seam 必须显式拒绝 external_pool。
用户、Provider owner、HTTP、MCP、fixture、seed 与 direct SQL 都不能调用 service-managed writer。
现有 pool、bucket、ledger、lifecycle、Offer、publication与price-snapshot facade分别拥有事务；实现时必须在各自owner内提供
Store-private `_on` kernel给唯一外层事务编排，不能嵌套调用facade或把上述写序拆成多个commit。

## 7. Gateway 的两段事务

首批只接受 normal reserved Job、active Reservation、held Capacity Claim、有效 budget、artifactless workload、
`result_artifact_required=false` 和 checkpoint disabled。

### 7.1 Transaction A：sealed Execution Plan

在 fresh V278 route+V274 runtime carrier 与 current V280 admission 的同一 checked-at 下，并要求双方V277 stable root/
provider binding/executor/adapter lineage一致：

Tx-A先按`attempt_lease_id`与`(job_id, attempt_no)`查询durable Plan。存在时只audit/readback同一plan/capability/seal并返回，
不得重新生成`plan_id`、`planned_at`或`sealed_at`；冲突则拒绝。只有不存在时才由server生成一次IDs/times并执行fresh写入，
所以commit-ambiguous retry仍是0写exact replay。

1. 由所属Domain owner的sealed builder构造verified execution capability：固定
   `capability_kind=adapter_execution/provider_kind=external_pool/route_kind=server_adapter`，route/adapter/executor取fresh V278，
   provenance与runtime取同一V274/V277 composite，十项resource ceiling逐项取historical profile，首批
   `capability.model=Offer.model=None`、`node_ready=None`；`observed_at/expires_at`由同一checked-at与最小硬截止派生，caller不能传raw numeric、route或time；
2. artifact access 数量固定为 0；
3. 重证 Provider/Offer/Job/Reservation/Claim/Broker/Budget；
4. 重证Job/Reservation/Plan引用的exact current v171 price snapshot来自同一admission，并由同Provider allocation账本证明
   Reservation与held Claim exact占1 attempt slot、总held/active不超过`max_concurrent_attempts`；
5. INSERT capability receipt；
6. INSERT 0 条 artifact access；
7. INSERT Execution Plan、ordinal accesses与plan seal；
8. exact readback并 commit。

Runtime/provenance逐字段映射固定为：`runner_id=V277 executor_id`、`Offer.runtime.runner_digest=capability.runtime.runner_digest=V277 stable_executor_binding_digest`；
`runtime_family/runtime_version/precision`逐字等于profile派生的Offer与Job runtime；首批`plugin_release=None`且Offer runtime
`plugin_id/plugin_version/plugin_digest`全为None，Job runtime逐字段等于Offer；`runtime_digest`
使用`ELON-EXTERNAL-POOL-SERVICE-MANAGED-RUNTIME-BINDING-V1`对上述字段与V274 runtime observation ID/digest做NUL+JCS+SHA-256。
Provenance的`source_schema/source_id/source_digest`取唯一V280 admission receipt，`verification_kind`固定
`external_pool_adapter_route_and_active_successor.v1`，`verifier_id/verification_digest`是V277 activation+current V274 successor+
current V278 renewal的canonical composite ID/digest，`authenticated_at=Tx-A checked_at`；`observed_at`同checked-at，`expires_at`
固定等于`plan.start.hard_deadline_at`，且`plan.planned_at=Tx-A checked_at`、profile inflight/permanent horizons必须覆盖该hard
deadline。V274最多15秒的observation与V278 rolling route expiry只约束Tx-A commit前的source currentness，不能缩短sealed
capability lifetime；Tx-B与每次fresh send另行重证current V274/V278。Capability ID只可在Tx-A durable Plan lookup确认none后由server
分配一次，digest继续使用既有capability canonical domain；caller不能选ID、field、time或digest。

`execution_plan` Domain owner必须提供唯一`pub(crate)` sealed builder
`validated_external_pool_service_managed_execution_plan_inputs`，内部构造`VerifiedComputeExecutionCapability`并返回最终
`ValidatedComputeAttemptExecutionPlanInputs`。它必须deep-audit完整Plan envelope与sources、derived resource grant、lease requirement、
V278 exact六项required route capability、artifact access固定空集、IDs/times/canonical digests；source-contract只允许Store
service-managed orchestrator调用。只返回capability或让Store用raw plan DTO拼私有三字段都不算可达。

首批lease requirement逐字固定`authority_kind=external_pool_adapter_task_lease`、`delivery_mode=eltp_commit`、
`audience=V277 executor_id`、`required_scopes=[compute_attempt.execute]`且`valid_until=plan.start.hard_deadline_at`；该hard horizon
不得越过profile inflight、Claim、budget、capability或其它permanent authority cutoff，scope必须排序且不得extra。
Required route capabilities逐字为current V278 route中的`authenticated_ack/authenticated_events/cancel_no_start/
idempotent_commit/prepare/reconcile`六项及各自exact revision，排序后0 missing/0 extra。Attempt identity、start、Offer/runtime、
workload、lease/hard times与derived resource grant全部从sealed Job/Reservation/Claim/Offer/profile sources生成，caller不能提交。

Tx-A 必须在snapshot与`new_plan_accept_until`前完成。Plan自身`not_after`、lease expiry与hard deadline由既有永久business
horizon和profile的`inflight_execution_valid_until`共同收紧；snapshot/admission的new-plan expiry可以早于Plan hard deadline，
因为reservation已经锁定价格，但不得晚于任何capability、budget、claim或lease authority硬截止。

Transaction A 后崩溃时，sealed Plan-without-command 是合法可恢复状态；它不发送任务、不占用 Runner，也不产生 Lease。

### 7.2 Transaction B：Start Dispatch

新 `BEGIN IMMEDIATE` 首先只读sealed Plan identity并执行下述双unique查询；existing分支只审计stored command/actor/outbox与
immutable Plan、historical admission/profile roots，必须在任何current business/route读取前0写返回。只有两把key都不存在时才
重读current business sources、fresh V278/V274 composite，并从Plan的price-snapshot pair反查唯一historical V280 admission/profile；
它不要求admission、profile或snapshot仍current/未过new-plan expiry。随后由所属模块内sealed builder构造private dispatch actor
receipt和prepare operation。写序必须复用现有Gateway约束：route/actor/prepare
outbox 先于 dispatch command，`command.issued_at >= plan_seal.sealed_at`，随后 exact readback并 commit。不能从 receipt、
raw envelope、Ready bool或 caller-selected route ID 构造 authority。Transaction B 必须重新取得 fresh V278 composite；
Transaction A 的 transaction-bound authority不得跨 commit。

Tx-B不能只在`attempt_gateway.rs`补最外层constructor。`ValidatedComputeStartOutboxOperation`拥有
`AuthorizedComputeRouteAuthorization`和dispatch actor receipt，后者再拥有`AuthorizedComputeServiceActor`；而fresh V278
composite只提供同一transaction内的borrowed route/actor view。Route owner复用现有pub(crate)
`validated_compute_route_authorization_from_canonical_envelopes`做完整deep audit（不改变其既有callers）；`start_outbox/validated.rs`
新增pub(crate) `validated_external_pool_service_managed_start_outbox_operation`，只接fresh canonical route envelopes、同源actor与sealed
dispatch/outbox material并一次性返回owned operation。该V280 wrapper再交给attempt gateway，source-contract锁唯一Store orchestrator caller。只有Store侧selector/orchestrator与
transaction-aware `_on` kernel使用`pub(in crate::store)`。禁止为此给authority增加`Clone`、`Serde`、public/raw/unchecked
constructor，或让caller分别选择route与actor。

`attempt_gateway` Domain owner还必须提供唯一`pub(crate)` sealed builder
`validated_external_pool_service_managed_start_dispatch`，消费上述owned start-outbox operation与server-derived activation material，
deep-audit command、adapter、activation idempotency、Provider-owner activation actor、dispatch service actor、lease ref/hint与outbox
全部roots后返回最终`ValidatedComputeAttemptStartDispatch`。Source-contract只允许同一Store orchestrator调用；只构造inner route、
actor、activation plan或outbox而不能返回final token仍属于零caller形状。

Tx-B的`activation_idempotency_key`由server按固定domain+JCS/SHA-256从Provider、Job/Reservation/Claim exact revisions、
Attempt lease/fencing generation与Plan ID/digest确定性派生，caller不能提供。Transaction一开始、任何current source或fresh构造前先按
`(provider_id, activation_idempotency_key)`与`(job_id, attempt_no)`两把unique key查询durable command：两者均无才fresh；任一存在时
必须0/1收敛到同一row并逐字回读同一command、dispatch actor receipt与prepare outbox，包括原
`command_id/issued_at/outbox_id/actor_receipt_id/ref/hint`；两把key指向不同row或material冲突立即拒绝，不得先生成IDs/times。
并发或commit-ambiguous loser必须经两把unique key exact readback，不能重铸第二command或把constraint冲突当fresh。

现有 `produce_compute_attempt_execution_plan` 与 `prepare_compute_attempt_start_dispatch` 各自开启事务，未来需要新增
各自的 Store-private transaction-aware `_on` kernel，由唯一 orchestrator 严格拥有两个先后事务；不得把 A+B 暗并成
一个外层事务、嵌套调用 facade或复制其 SQL。若 artifact access 数量为 A，两段 fresh 净新增固定为`6+2A`行；首批
artifactless A=0，因此是6行。Exact replay为0新增。

`ComputeAttemptStartActivationPlan` 还必须消费 server-issued、non-bearer、per-Attempt 的 sealed lease delivery material。
该 material 从Tx-A锁定的historical market profile内lease issuer policy派生，精确绑定 Provider、Job、Reservation、Claim、Attempt、
Plan seal、route、executor、fence、`lease_credential_ref`、`lease_credential_hint`、issued-at与expiry；ref/hint不是Secret或
bearer token。它不能来自V278 route credential、caller字段、Ready、旧v185 request或随机字符串。Tx-B exact replay必须
回读同一ref/hint，不能重铸新值；issuer root已由Plan snapshot→唯一admission→historical profile链durable保存，ref/hint是
该root与Attempt/Plan/route/executor/fence的domain-separated确定性派生。Accepted closure把同一ref/hint写入
LeaseAuthorityBinding，并通过上述历史链重算验证；现有binding无需伪造一个并不存在的issuer-root字段。

Tx-B 的两种actor必须分离：`activation_plan.activated_by_user_id`固定为fresh Provider authority中的exact
`provider_owner_account_id`，并与route source `approved_by_user_id`一致；dispatch actor receipt则使用V278 authorized service
actor，且必须与Provider owner不同。Market admission actor、Provider owner与dispatch service actor都由各自typed authority
内生，caller不能传值或互相替代。

## 8. Task session、network 与 authenticated ingress

task session 不得复用 V278 no-work child、六根 Secret binding或已消费 TLS channel。唯一顺序是：

1. active preparation 完成 current V278 route、V268/V272 carrier与current V274；
2. 短事务选择 exact command/outbox并产生 owned、non-authorizing session preflight；
3. 无 SQLite transaction 地启动独立 managed child，绑定 task-specific 八根 session，交付 ephemeral Secret并连接 fresh TLS；
4. 新事务 final reproof后执行`outbox pending→claimed CAS`，再按
   `v213 send-attempt INSERT→V273 exchange-attempt INSERT→outbox claimed→in_flight_unknown CAS`写outbound子计划；
   整个事务合计2 INSERT+2 CAS，其中outbound leaf固定2 INSERT+1 CAS，随后commit；
5. 只有 commit 后才执行最多 15 秒的 ELTP/TLS await；
6. 新 historical-cleanup 事务插入 authenticated receipt并在 commit 前解决全部 ingress obligation。

每次exchange的effective deadline取以下适用窗口最小值：ELTP 15秒、claim expiry、task-session/Secret custody、
`plan.not_after`、command/outbox operation deadline、reservation/lease/hard deadline、profile
`inflight_execution_valid_until`、fresh V253 credential、V278 route authorization/actor/seal，以及historical poll/cancel的cleanup
horizon。Offer/snapshot的quote/new-plan expiry只约束Tx-A，sealed Plan之后不重新加入exchange cutoff。Final reproof、ELTP request write
前的本地deadline check、semantic validator与receipt timestamps都必须落在同一effective deadline内；任一已过期则零network或
把durable send留作reconcile，不能延长窗口。

SQLite transaction、rusqlite authority、Prepared installation、Secret与borrowed current carrier都不得跨 await。Owned child
handle必须先移交独立async supervisor custody，再由该supervisor跨ELTP await持有并负责shutdown/reap/cleanup；它不能与DB
transaction或borrowed authority同栈跨await，也不能丢弃后仅凭PID重建authority。

Production validator 只能在 broker TLS owner 内实现 sealed traits。它必须为`prepare`、`idempotent_commit`、
`cancel_no_start`、`reconcile`、`authenticated_events`五种 exact operation token使用 deny-unknown canonical DTO，绑定 command/outbox/
route/executor/fence/request/session roots与大小上限。Raw response bytes只在 Zeroizing buffer内存在；Store 只接收不可拆分的
verified host receipt+typed semantic wrapper，不能接收独立 receipt与caller-supplied semantic。

## 9. Terminal、Runner 与 recovery

Prepare accepted 的同一 ingress transaction 必须依次形成 V273 authenticated receipt、v213 remote observation INSERT、
start outbox `in_flight_unknown→delivery_observed` CAS、v211 ACK、v185 activation、v215 application actor、
LeaseAuthorityBinding、commit outbox与application，并 exact readback后commit。Runner 只有在远端
取得 exact LeaseAuthority 后才允许运行；Start、ACK 或 activation 都不能单独代表 Runner 已启动。

Commit、event、cancel与reconcile继续复用 V213/V273/V278 state machine：

- durable send 无 receipt 一律为 unknown，进入 reconcile，不得盲目重发；
- sealed Plan 无 command 只重试 Transaction B；pending command/outbox只有在原route/V274 final reproof与locked issuer material integrity仍成立时才可claim；
- historical cleanup 不要求 route/V274 current，但新 send 必须重新取得 fresh V278+V274 carrier；
- cancel ACK 只证明请求收到，terminal-no-start 仍需 reconcile receipt+tombstone+proof；
- running event batch 同事务产生 successor poll，terminal batch才关闭 transport；
- transport terminal 不等于 usage、Attempt finalization、metering或settlement完成。

若Tx-B commit后、首次send前发生route/V274 rotation或locked issuer material/lineage integrity失败，禁止把既存command/outbox重绑到
新route，也不得启动network或无限claim。Task-session supervisor先shutdown/reap/cleanup child；outbox保持durable pending
（过期claim先按既有恢复回pending），直到immutable prepare `not_after`关闭后由既有窄CAS进入`abandoned_no_send`，再派生
`local_never_sent` proof并通过broker finish gate释放对应Job/Reservation/Claim/budget。任何send-attempt已存在时都不能走
local-never-sent，必须按unknown→reconcile处理。

普通profile/admission/snapshot expiry或Tx-A后的catalog revocation不得回溯阻断未过`plan.not_after`的Tx-B恢复；Tx-A已把
当时未撤销的exact issuer root锁入historical admission/profile链。Revocation只阻断新的Tx-A，既存Plan继续使用原root且
绝不能换issuer。若产品需要在Start前强制终止sealed Plan，必须后续新增plan-before-start cancellation与Held Claim/
Reservation/Job/budget释放authority；现有Attempt abort要求已激活Lease，不能冒充该路径。

Ingress obligation必须在同一transaction内被ACK/no-start/event/successor closure消费；任何crash或drop都会使receipt及closure
整体rollback。因此不存在可提交的“receipt已写但未闭包”恢复状态：restart只能由durable exchange-without-receipt派生并claim
reconcile，再用fresh task session接收reconcile receipt；不能恢复旧socket、旧response或旧process custody。已经完整commit的
receipt+closure只能做零写exact replay。

## 10. Ownership 与实施切片

新增独占目录建议为：

- Domain：`compute_federation/external_pool_service_managed_admission/`；
- migration：`store_migrations/compute_external_pool_service_managed_admission/`；
- Store：`store/compute_external_pool_service_managed_admission/`；
- worker：`compute_federation/external_pool_adapter_task_worker/{runner,session,recovery}.rs`；
- transport：`compute_federation/external_pool_adapter_broker_tls/production_validator.rs`。

共享 seam 包括 Store/migration aggregators、`server/src/compute_federation/mod.rs`、V254 fences、capacity registry/lifecycle/ledger、
Offer registry/publication、v171/v223 snapshot writers、execution-plan validated owner、route-authority owner、`start_outbox/validated.rs`与attempt gateway，必须由一个集成
owner串行收口；worker ownership同时包含`external_pool_adapter_task_worker.rs`及其`lifecycle.rs`/`cycle.rs`/`report.rs`，transport ownership
包含`external_pool_adapter_broker_tls.rs`。每个 leaf ≤430 行；入口只做
声明、re-export或编排。

推荐实现顺序是profile子权威首项payload审批→按已冻结profile/admission/projection ABI实现policy/admission Domain/DDL/Store→
service-managed market transaction→Gateway A/B→task session+
validator→Runner/ingress/recovery→source-contract→恢复后的 compile/migration/runtime acceptance。前一步未闭合时不得打开下一步
生产 fence。

## 11. 明确非目标

V280 不实现 user_node Ready/v15、result artifact、checkpoint、usage metering、settlement、提现、容量期货、多 Provider
并行/权重公平调度（本阶段只冻结有界selector防饿死）、pre-start sealed Plan cancellation、Windows Runner、公开 API、动态 plugin schema或生产发布。它也不修改 V279 binding receipt、V277 activation、
V278 renewal历史语义。完成本页文档不改变任何运行态能力。

验收合同见 [`external-pool-service-managed-admission-runner-acceptance.md`](external-pool-service-managed-admission-runner-acceptance.md)。
