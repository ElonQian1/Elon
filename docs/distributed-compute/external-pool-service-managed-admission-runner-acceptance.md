---
title: 外部矿池 service-managed admission 与 Runner bridge 验收
status: current
reviewed_at: 2026-08-21
owners: backend, security, ai-economy
design_status: design_frozen
design_scope: end_to_end_vertical_slice_architecture_only
implementation_status: implementation_unwired
verification_status: design_review_only
---

# 外部矿池 service-managed admission 与 Runner bridge 验收

## 1. 当前证据强度

V280 当前只有权威与验收设计，正式计数为 `passed=0 / failed=0`。仓库不得出现以下当前事实之外的声明：

- 全局 migration 最高为 V279，V280 尚未注册；
- V254 #13-#18 全部维持原 deny 行为，opened=`0`；
- 没有 V280 table、UDF、trigger、Domain、Store、production validator或Runner源码；
- V278 worker仍default-off，Provider=`registering`、`eligible_rows=0`、`delivery_attempted=false`；
- 没有正常 external-pool Offer→Job→Plan→Start→ACK→Lease→Runner 动态证据。

本批只允许记录纵切架构`design_frozen/design_review_only`，并逐轴说明
`market_profile_schema_abi=design_frozen`、`market_profile_inventory_approval_evidence_abi=design_frozen`、
`initial_profile_approval_evidence/initial_profile_inventory=unselected`、
`admission_receipt_canonical/physical_schema_abi=design_frozen`、`market_projection_identity_abi=design_frozen`、
`gateway_builder/fence/task_session/validator_internal_abi=design_frozen`、
`external_adapter_semantic_wire_profile_registry_abi=design_frozen`、
`external_adapter_semantic_wire_profile_approval_evidence_abi=design_frozen`、
`initial_external_adapter_semantic_wire_profile_approval_evidence_set=unselected`、
`initial_external_adapter_semantic_wire_profile_inventory=unselected`、
`external_adapter_semantic_wire_profile=unselected`与实现
`unwired/uncompiled/unrun`。文档链接、source-size或静态零改动扫描
不是实现证据，任何 V273/V274/V277/V278 历史 passed 也不能计入 V280。

## 2. 本次文档冻结验收

| Case | 必须结果 |
|---|---|
| authority | authority与acceptance互链，并从分布式算力README、当前状态、AI入口可达。 |
| version truth | `MIGRATIONS` max/last仍为279；无`migration_v280`、V280 schema或UDF注册。 |
| fence truth | V254六个market fence源码零修改；#13/#15继续绝对deny。 |
| worker truth | 不改worker report，不制造`eligible_rows>0`或production validator caller。 |
| worktree | 除文档外零Rust/SQL/schema/API改动；`git diff --check`、文档模块化和本地链接通过。 |
| status | 所有入口都明确0/0、unwired、uncompiled、unrun，不使用implemented/reachable/ready。 |

只有这些静态检查通过时，本批可称“V280 end-to-end vertical-slice architecture frozen”；不得称“V280 fully frozen”或
“V280 implemented”。

## 3. Market profile gate

[Market profile authority](external-pool-service-managed-market-profile-authority.md) 与
[acceptance](external-pool-service-managed-market-profile-acceptance.md)已冻结schema/canonical ABI、结构约束与empty-inventory
失败关闭；初始产品载荷仍`unselected`，所以current profile authority不可构造。
[Projection identity authority](external-pool-service-managed-market-projection-identity-abi-authority.md) 与其
[acceptance](external-pool-service-managed-market-projection-identity-abi-acceptance.md)只冻结legacy market identity/clock映射，
不使profile或writer可构造。

进入source-written前必须提交byte-exact profile JSON/digest与
[purpose-specific四眼approval evidence](external-pool-service-managed-market-profile-approval-evidence-abi-acceptance.md)，并通过子验收的capacity、price、ceiling、
workload、transport、deadline、issuer与负向source矩阵。缺任一项时migration、fence replacement、Gateway和Runner都必须停止，
不能以fixture、默认零值、最小值或无限上限继续。

## 4. Admission schema 与 migration matrix

| Case | 必须结果 |
|---|---|
| durable shape | exact 1张immutable admission receipt表；0 mutable head、0 view、0 revocation、0 session/Secret/payload表。 |
| semantic projection | identity、V277 stable root、admission-time V274/V278 historical witnesses、profile+per-provider allocation、Pool/ledger、Offer/publication、v171 price snapshot、time、canonical receipt全部逐列投影。 |
| idempotency | market admission actor/confirmation/scope/key/request digest逐字投影；replay在current-source read前，caller不能选择这些字段。 |
| ABI freeze | admission子权威已冻结77列/canonical/parent keys；projection子权威已冻结legacy ID/digest/clock mapping；table/migration/source仍absent。 |
| genesis | sequence固定1、predecessor固定NULL；同Provider只有一份genesis，不接受第二个不同请求。 |
| capacity scope | 同Provider重复或改量拒绝；两个Provider的allocation ID/digest必须不同，各自只铸一次profile授权量；不得出现global余额或跨Provider重复消费的隐式解释。 |
| concurrency | Tx-A同事务重证同Provider ledger与exact Reservation/held Claim各占1 slot，held+active总量不得超过profile；跨cycle或并发writer也不能超售。 |
| fresh | admission INSERT前要求当时current的V274/V278+exact profile/Provider/V277/Pool/Offer/v171 snapshot同一checked-at重证，单INSERT/readback。 |
| rolling heads | receipt中的V274/V278只作historical witness；head轮换不使admission stale，Plan/Start另取fresh composite并对齐stable V277/provider/executor/adapter lineage。 |
| replay | 在current-source read前按stable request/receipt identity零写回读；任何内容冲突拒绝。 |
| no successor | profile失效只使currentness失败；不得追加第二active Pool/Offer。sequence>1须后续authority先退役/冲销旧market state。 |
| immutability | UPDATE/DELETE/REPLACE、backfill、seed、fixture与直接SQL全部拒绝。 |
| upgrade | fresh/repeat/reopen、旧库升级与中途失败都得到唯一相同inventory；不得重解释历史V277/V278行。 |

物理版本开始前重新核对全局max；若280已占用必须顺延，绝不能覆盖已有migration。

## 5. Market transaction 与 pending-plan matrix

正向动态验收必须在一个 IMMEDIATE transaction 中观察以下严格顺序：Pool registering root→pool v1→Pool
projection UPDATE(仍registering)→N buckets→supply transaction→2N legs→N bucket CAS→lifecycle event→Pool active
CAS→Offer draft root→draft v1→active v2→Offer active CAS→publication→v171 price snapshot→admission receipt→full
readback→commit。

| Case | 必须结果 |
|---|---|
| V254 #14 | 只允许exact Pool registering→active CAS，消费一次plan。 |
| V254 #16 | 只允许exact draft Offer root INSERT，消费一次plan。 |
| V254 #18 | draft v1与active v2各消费一次，顺序或内容漂移拒绝。 |
| V254 #17 | 只允许同一Offer draft→active CAS，消费一次plan。 |
| V254 #13/#15 | 仍absolute deny，UDF名不得出现在条件中。 |
| registering projection | pool root INSERT→version INSERT→status仍registering的projection UPDATE各有独立ordered kind；第3步不得借#14 active CAS permit。 |
| auxiliary guards | bucket/ledger/legs/bucket-CAS/event/publication/v171 snapshot/admission各写点都受同一ordered plan或exact source guard保护。 |
| price snapshot | 复用既有v171表/private kernel；ID确定性，source=fallback_curve/sample_count=0，所有金额/组件/TTL取profile；owner facade与v223 admin apply都拒绝external_pool，唯V280 ordered-plan `_on` writer可写。 |
| snapshot lifetime | genesis只建一份initial snapshot；过期后停止新Job/Tx-A，不撤销未过plan.not_after的sealed Plan；refresh/successor不得覆盖历史行。 |
| historical lookup | snapshot ID/digest唯一绑定一份admission；Tx-B可由Plan snapshot反查historical profile，0/multi拒绝。 |
| custody | wrong-order、extra/missing write、跨connection、未fully-consumed、rollback、guard drop均失败关闭且无残留permit。 |
| owner APIs | external_pool bucket/supply/Offer经owner-facing API全部拒绝；只有Store-private orchestrator可写。 |
| transaction owner | pool/bucket/ledger/lifecycle/Offer/publication只暴露各owner的private `_on` kernel；不得嵌套facade或中途commit。 |

UDF 必须是 variable arity、非 deterministic、UTF8+INNOCUOUS、connection-local、one-shot、type+bytes exact，
并覆盖 SQLite handle ABA 与 commit/rollback custody。

Production reachability还必须冻结独立default-off gate
`ELON_EXTERNAL_POOL_SERVICE_MANAGED_MARKET_ENABLED`：false时profile read、market/Gateway candidate scan、plan install、Tx-A、
Tx-B与SQL mutation均为0。true时bounded worker cycle严格为active preparation→最多一个admission fresh/replay→最多一个
Gateway work item→V278 source scan/session/cleanup；admission与Gateway commit都必须先于消费新outbox。Env、HTTP、Provider owner
或Job caller不得注入market admission actor、confirmation、idempotency或candidate identity。

Lifecycle/source-contract必须覆盖双gate四组合：market=false+delivery=false不spawn；market=true+delivery=false仍spawn并只运行
preparation/admission/Gateway，task claim/task-delivery ELTP request为0且outbox可durable pending；market=false+delivery=true仍spawn并只运行既有V278
preparation/source/session/cleanup；两者true才跑完整顺序。任一true都必须初始化同一worker与所需runtime custody，不能因旧delivery gate=false
让market production caller消失，也不能因market gate=false停止既有durable delivery cleanup。

注入active-preparation failure时，admission/Tx-A/Tx-B/fresh send call必须为0或deferred，但delivery gate=true时historical cleanup
selector与其reconcile/cancel/event/terminal ingress call必须继续且只可使用historical authority；不得由cycle级`await?`提前返回。
注入单个admission/profile/market transaction failure时，本轮admission slot释放，既有admission的Gateway与historical cleanup仍继续。

Admission selector必须只计`provider_kind=external_pool`+exact V249 binding+current V277/V274/V278 roots+no V280 genesis；
Gateway分别计算“external_pool Plan+唯一historical V280 admission+sealed Plan存在且exact command不存在”的recovery集与
“current external-pool admission+reserved Job+active Reservation+held Claim+budget且无durable Plan”的fresh集。两类都非空时，
worker-owned non-authorizing lane turn每次eligible arbitration后必切换，启动值只取server UTC-minute parity；每cycle最多一个work item，
recovery只执行Tx-B，fresh执行Tx-A并在commit后以新事务fresh reproof再执行Tx-B。
三类candidate（admission、Plan recovery、fresh Job）的COUNT与SELECT必须各自使用同一predicate、稳定排序和server UTC-minute
modulo offset；caller slot/ID、永久最老`LIMIT 1`与失败队头饿死后续Provider/Job都应拒绝。Selection只是hint，事务内必须
重新证明并以unique identity使并发loser exact readback或defer。单个Gateway失败不得阻断已提交admission、historical cleanup或poll recovery。
V279 user_node Provider即使active也必须count=0/never selected；整个V280正向与负向矩阵对V279表保持零读、零写、零effect。
Worker report必须对preparation/admission/Gateway A/Gateway B/delivery/historical cleanup输出结构化stage outcome，即使
`eligible_rows=0`也记录deferred/error；报告不得带业务ID/Secret/raw payload，动态正向仍以durable readback而非report常量计数。

## 6. Gateway matrix

| Case | Transaction A | Transaction B |
|---|---|---|
| source | current admission+fresh V274/V278 composite+同一v171 snapshot、reserved Job、active Reservation、held Claim、budget | sealed Plan identity先双key lookup；existing只按stored+historical roots回读，none才读current business+historical admission/profile并fresh V274/V278 |
| identity/replay | 先按attempt_lease_id与(job_id,attempt_no)查Plan；existing回读原IDs/times，none才fresh | server domain派生activation key；同时查(provider,key)与(job,attempt)，0/1必须收敛同一row，existing回读command/actor/outbox原IDs/times，分叉拒绝 |
| capability | 固定adapter_execution/external_pool/server_adapter；fresh V278 route+V274/V277 provenance/runtime+profile十项ceiling，model/node_ready=None | 只消费sealed Plan capability，不重铸或换root |
| workload | artifactless、0 access、no result artifact、checkpoint disabled | exact同一workload与Plan seal |
| writes | capability→0 access→Plan→ordinal access→seal→readback | route/actor→prepare outbox→dispatch command→readback |
| row count | `3+2A`，artifactless A=0为3 | route authority exact replay新增0，actor+outbox+command固定3；两段总`6+2A`，首批6 |
| recovery | commit后崩溃保留Plan-without-command | profile/snapshot expiry或Tx-A后revocation不回溯；plan.not_after前用locked issuer重试B，绝不轮换root |
| lease delivery | 不产生lease credential或ref/hint | server issuer按profile policy铸sealed non-bearer material，exact绑定Attempt/Plan/route/executor/fence |
| actor split | 不产生activation/dispatch actor | activated_by固定Provider owner且等于route approved_by；dispatch receipt固定V278 service actor且不等于owner |
| owned authority | 不跨commit携带V278 authority | Domain owner的pub(crate) sealed builder从同一fresh canonical envelopes deep-audit并封装owned route+同源actor；source-contract限唯一Store caller，无Clone/Serde/raw ctor |
| negative | historical V274/V278 pair当current、raw ceiling/artifact/route/Ready/bool、过期profile、非reserved Job拒绝 | caller key/IDs/times/envelope、route credential充当lease credential、raw ref/hint、receipt-as-authority、跨head未对齐stable lineage、nested facade transaction拒绝 |

两段都必须使用所属模块内 sealed builder 和 transaction-aware Store kernel。现有 facade 的独立 IMMEDIATE transaction
不得被嵌套；不得复制SQL或公开private validated type构造器。Exact replay两段都为0新增，任何额外Gateway行拒绝。
并发fresh/commit-ambiguous retry必须恰好一个writer，其他调用回读同一Plan或command；ID/time漂移、相同key不同material、
相同Attempt第二Plan或相同Provider/key第二command均拒绝。

Tx-A source-contract必须锁`execution_plan` owner的pub(crate) final builder直接返回
`ValidatedComputeAttemptExecutionPlanInputs`，并证明完整Plan/resource grant/lease requirement/六route capabilities/空access集合；
Tx-B必须锁route/start-outbox owner tokens最终由`attempt_gateway` owner的pub(crate) builder返回
`ValidatedComputeAttemptStartDispatch`。两者都只能有唯一Store orchestrator caller，raw DTO、partial inner token或public ctor为负例。

Capability runtime逐字段必须锁V277 executor ID、Offer/capability runner digest=V277 stable binding digest、profile→Offer/Job family/version/precision、Offer model=None、Offer plugin三字段=None、capability plugin_release=None与
V274 observation-rooted runtime digest；provenance逐字段锁V280 admission、V277/V274/V278 composite与Tx-A checked-at。Capability
observed/authenticated/planned time均锁Tx-A checked-at，expires_at固定Plan hard deadline；V274 15秒observation与V278 rolling
route expiry只作Tx-A source-currentness gate，绝不能当capability lifetime。ID只在none-after-durable-lookup后由server分配，
重复/并发必须读回原ID/digest；任何字段改源或caller输入都拒绝。
Plan lease requirement还必须exact为`external_pool_adapter_task_lease/eltp_commit`、audience=executor ID、单scope
`compute_attempt.execute`、valid_until=plan.start.hard_deadline_at且不越过任何permanent authority cutoff；route capability集合只准current V278六项exact revisions，missing/extra/
乱序都拒绝。Attempt/start/time/resource grant必须由sealed sources与既有canonical derivation生成，不能由candidate DTO夹带。

Source-contract必须锁住从task worker cycle到service-managed selector/orchestrator、Tx-A `_on` kernel、Tx-B `_on` kernel再到
V278 source scan的唯一production call path；只有builder/Store定义而没有该caller不得晋级`source_written`。动态验收还必须覆盖
gate=false时上述call count全0、Plan-without-command重启可恢复、fresh Job产生Plan+command、并发candidate collision、deferred
候选不造成队头阻塞、persistent recovery failure下fresh在至多一个对方lane机会后仍推进（反向亦然），以及Gateway失败后cleanup/recovery仍继续。

## 7. Session、validator 与 network matrix

| Case | 必须结果 |
|---|---|
| session isolation | task-specific八根、fresh child/Secret/TLS；不复用V278 no-work六根、child或channel。 |
| no authority across await | DB tx、borrowed current authority、Prepared installation与raw Secret在await前释放；只携owned plan、child/session/TLS/deadline/join custody。 |
| child custody | Host/Exchange只在dedicated blocking worker lexical借用owned session；async supervisor持join/cancel并负责shutdown/reap/cleanup。 |
| outbound | final reproof后同tx先pending→claimed CAS，再执行2 INSERT+claimed→unknown CAS；总计2 INSERT+2 CAS，outbound leaf为2+1；commit前ELTP request计数=0。 |
| validator | 五operation concrete semantic view、root/size/time exact；external wire profile未选，未来JSON才强制deny-unknown/JCS。 |
| wrapper | Store只收不可拆verified receipt+semantic wrapper；独立receipt+semantic或generic raw callback在类型边界不可达。 |
| timeout | effective deadline=min(15s、claim、session/Secret、plan/command/outbox、reservation/lease/hard、profile inflight、fresh credential/route/actor/seal、historical cleanup适用窗)；失败不回滚durable send。 |

Offer/snapshot的quote/new-plan expiry只约束Tx-A，不得在sealed Plan恢复时重新加入timeout。Final reproof、socket write前
local check、validator与receipt times都必须在同一effective deadline内；准备期间过期后application request计数为0，durable unknown send只走
reconcile。

至少覆盖 framing/HMAC错误、operation substitution、unknown field、digest/root/session漂移、oversize、timeout、child退出、
cleanup失败与crash between commit/network/receipt。

## 8. Ingress、Runner 与 recovery matrix

Prepare accepted 必须在同一事务写 V273 receipt→v213 remote observation→start outbox delivery-observed CAS→v211 ACK→
v185 activation→v215 actor→LeaseAuthorityBinding→commit outbox→application→readback。任何 pending ingress 被drop、
未消费或commit前obligation未解决都应rollback。

| Case | 必须结果 |
|---|---|
| Runner gate | 远端只有拿到exact LeaseAuthority后才启动；Start/ACK/activation均不足。 |
| lease binding | binding保存Tx-B exact ref/hint；issuer root由Plan snapshot→唯一admission→historical profile重算，replay不得轮换或伪造不存在的root列。 |
| direct accepted | authenticated semantic exact，ACK/activation/application原子；replay零写readback。 |
| send timeout | durable send保持unknown，只建reconcile，不重复prepare/commit。 |
| stale before first send | route/V274 final reproof或locked issuer integrity失败则network=0、child cleanup、绝不rebind；到prepare not_after后abandoned_no_send→local_never_sent→broker release。 |
| reconcile | accepted/no-start/event/successor disposition唯一，fresh与replay不fork。 |
| cancel | cancel ACK不释放资源；terminal-no-start须receipt+tombstone+proof。 |
| events | running batch产生cursor successor；terminal batch只关闭transport，不冒充usage/finalization/settlement。 |
| historical cleanup | old send可在cleanup horizon ingress，即使route/V274已换代；任何新send仍要求fresh V278/V274。 |
| restart | sealed Plan、pending outbox、claimed poll、exchange无receipt逐态恢复且幂等；receipt+closure同tx rollback或完整commit，不存在durable未闭包receipt。 |

## 9. 必须通过的最小正向纵切

在允许宣称 V280 reachable 前，至少用单一 external_pool Provider、单一 Pool/Offer、单一 Job/Reservation/Claim、
单一 artifactless Attempt 完成：

1. service-managed admission原子提交；
2. `eligible_rows` 曾经大于0；
3. Plan与Start两段提交；
4. prepare accepted、commit、event terminal-after-run；
5. worker队列回到0且没有悬挂delivery/outbox/poll claim或ingress obligation；Capacity Claim与Attempt业务终态仍由后续v193/v194流程推进；
6. durable send后timeout→reconcile成功；
7. prepare未commit→cancel→terminal-no-start；
8. 在关键commit边界重启后exact recovery。

动态矩阵还必须覆盖fresh/repeat/reopen migration、wrong-order plan、并发claim、rollback、stale profile/route、超时、
semantic substitution与所有V254 fence负例。仅source-contract、fixture或手工INSERT不能替代该纵切。

## 10. 状态晋级规则

- 父authority/acceptance完成：只证明`vertical_slice_architecture=design_frozen`；
- profile子权威完成：只证明`market_profile_schema_abi=design_frozen`，初始inventory仍可保持unselected；
- approval evidence子权威完成：只证明evidence ABI可实施；真实evidence与首个profile仍可保持unselected；
- [semantic wire registry子权威](external-pool-adapter-production-semantic-wire-profile-registry-abi-authority.md)完成：只证明registry、
  purpose-specific evidence、current/historical selection与两道pre-send元ABI可实施；actual五operation set仍可保持unselected；
- 首个market profile approval/payload或external Adapter semantic wire profile set未选择时不得进入source-written；production fence与Gateway/session/
  validator内部ABI虽已冻结，但没有五类wire profile与实现仍不可达；
- Domain/DDL/Store/Gateway/validator/Runner全部落盘且静态合同通过：可记`source_written/source_review_only`；
- compile+fresh/repeat/reopen migration通过：才可记`implementation_compiled`或对应局部验证；
- 正向纵切、timeout/reconcile/cancel/restart动态矩阵通过：才可记production reachability已验证；
- usage、settlement、public API、user_node Ready、Windows Runner仍另立阶段，不能由V280推导。
- pre-start sealed Plan强制取消与Held Claim/Reservation/Job/budget释放另立authority；不得误用要求已激活Lease的Attempt abort。

对应权威见 [`external-pool-service-managed-admission-runner-authority.md`](external-pool-service-managed-admission-runner-authority.md)。
