---
title: 外部矿池 Adapter stable executor 与原子激活验收
status: current
reviewed_at: 2026-08-16
owners: backend, security, ai-economy
design_status: design_frozen
implementation_status: implementation_uncompiled
verification_status: source_review_only
---

# 外部矿池 Adapter stable executor 与原子激活验收

## 1. 当前证据与唯一结论

V275 当前只完成 authority/acceptance 静态冻结。未编译 Rust、未执行 migration/SQLite、未运行 HTTP/startup/
filesystem/Linux child/network/restart/direct-SQL/fault injection，正式计数为 `passed=0 / failed=0`，状态为
`design_frozen / source_review_only / implementation_uncompiled / implementation_unrun`。

唯一语义来源是
[`external-pool-adapter-stable-executor-atomic-activation-authority.md`](external-pool-adapter-stable-executor-atomic-activation-authority.md)。
文档出现不等于表、trigger、UDF、active Provider、route或executor已经存在；当前Provider=`registering`、V273
`eligible_rows=0`。

## 2. Namespace、canonical 与无环关系矩阵

| Case | 必须结果 |
|---|---|
| exact namespace | 仅一张 `compute_external_pool_adapter_atomic_activation_receipts`；0 view、0 revocation、0 mutable head/queue/pending table。 |
| permanence | row 完全 immutable且永不 current；UPDATE/DELETE/REPLACE、canonical drift和非exact replay全部拒绝。 |
| canonical direction | V274 receipt保存V275 witness pair；V275 canonical内不存在V274 receipt ID/digest或其nested/indirect copy。 |
| one-way FK | V275自身receipt/root三元组为UNIQUE parent；V274 witness/root三元组以immediate FK引用它。V275先写、V274后写，反向FK禁止。 |
| successor reuse | 同一activation root的V274 genesis/successor可多row复用exact同一V275 witness triple；V274 child triple不得UNIQUE。 |
| negative closure | 缺V275 parent、冲突的第二个V275 witness、wrong digest/root、same sequence/sibling predecessor或任意反向跨版本edge全部失败且零残留。 |
| migration | 先建V275 table/parent UNIQUE，再zero-row rebuild V274双时间+immediate child FK；fresh/repeat/reopen保持V275 exact 1表/0view/0revocation，且不得seed业务row/process registry。 |
| integrity UDF | deterministic one-arg `elon_v275_external_pool_adapter_atomic_activation_receipt_is_exact(json)`只重算canonical/scalar projection；flags exact为`SQLITE_UTF8 | SQLITE_DETERMINISTIC | SQLITE_INNOCUOUS`。 |

receipt canonical domain必须exact为
`ELON-EXTERNAL-POOL-ADAPTER-ATOMIC-ACTIVATION-RECEIPT-V1`，全部 scalar与JSON投影逐字段相等，六 capability按
v213固定顺序展开。receipt含历史 V253/V270-equivalent/V272 evidence snapshot但不延长TTL；含V274 identity、
pending token、raw-result、process HMAC/Secret/session、market或V276字段均为P0。

## 3. 双时间与 Provider bytes 矩阵

| Case | 必须结果 |
|---|---|
| preflight | I/O前服务端生成`activation_target_updated_at`，从current registering typed Provider冻结adjacent projected-active bytes/root。 |
| target bytes | 只用`serde_json::to_string(&ComputeProvider)`；`policy_revision=source+1`、status=`active`、adapter=projection、`updated_at=activation_target_updated_at`，其余字段exact。 |
| final time | 外部证据完成后final transaction生成独立`evidence_checked_at`；不得回写target时间。 |
| ordering | `source.updated_at <= activation_target_updated_at <= started <= completed <= evidence_checked_at < expires_at`。 |
| final reproof | 同connection从live source与structural roots重新生成同一target/root，并在`evidence_checked_at`重验所有renewable evidence。 |
| custody | Transaction/Connection/Prepared authority/pending seal不得跨filesystem/network/child/await；typed current authority不能由raw result wrapper替代。 |

caller timestamp/JSON、两时间合并、commit时间替换target时间、source/target/root/evidence漂移、过期或跳revision均失败。

三摘要还必须逐字验证：projected transition proof使用
`ELON-EXTERNAL-POOL-ADAPTER-CREDENTIAL-PROJECTED-ACTIVE-TRANSITION-PROOF-V1`并覆盖schema、binding/root、source/
target Provider pair、registering V253 pair、logical/projection IDs与`evidence_checked_at`；idempotency使用
`ELON-EXTERNAL-POOL-ADAPTER-ATOMIC-ACTIVATION-IDEMPOTENCY-V1`，actor固定`provider_owner`，actor user ID逐字取
`activation_root.activation_root.provider_owner_account_id`（列可名`activated_by_actor_user_id`，不得造alias），
scope固定`external_pool_adapter_atomic_activation`、key固定root digest；confirmation literal固定
`I_CONFIRM_EXTERNAL_POOL_ADAPTER_ATOMIC_ACTIVATION`并用
`ELON-EXTERNAL-POOL-ADAPTER-ATOMIC-ACTIVATION-CONFIRMATION-V1`绑定actor、idempotency digest与binding/root。
三者均按`SHA256(domain || 0x00 || RFC8785_JCS(material))`；caller-selectable actor/key/confirmation或任一缺字段均拒绝。

## 4. Stable executor 与 projected binding 矩阵

| Case | 必须结果 |
|---|---|
| ID domain | exact `ELON-EXTERNAL-POOL-STABLE-EXECUTOR-ID-V1`，结果为`external_pool_executor_<64 lowercase hex>`。 |
| binding domain | exact `ELON-EXTERNAL-POOL-STABLE-EXECUTOR-BINDING-V1`，对authority列出的RFC8785 material计算。 |
| stable identity | Provider binding/root/projection/service actor/carrier policy任一漂移改变ID；process/session/receipt/expiry不进入ID。 |
| projected v211 digest | exact复用`ELON-COMPUTE-ATTEMPT-ADAPTER-BINDING-V1`与既有8字段shape；仅`adapter_id=route_adapter_projection_id`，Provider kind/route kind固定`external_pool/server_adapter`，不存在Provider revision/digest/status或executor slot。 |
| lineage separation | V271/V254 `logical_adapter_binding_digest`只做lineage；不得等于projected route/adapter binding digest。 |
| anti-substitution | projection/logical Adapter、service actor、lane subject、OS process、session、worker均不能冒充stable executor。 |

V275 route、seal与future v211 command必须逐字使用同一projected digest；planned active Provider pair与stable executor
由V275 activation-route binding/receipt另行共同绑定。wrong domain、JCS、字段集合、大小写、ID前缀、向v211 shape
虚构Provider revision/digest/status或executor字段，或把V274/V275 receipt identity混入stable material均拒绝。

private active carrier必须使用
`ELON-EXTERNAL-POOL-ADAPTER-TASK-PROTOCOL-ACTIVE-CARRIER-V1`与schema
`compute_federation.external_pool_adapter_task_protocol_active_carrier.v1`；material exact只有binding pair、activation root、
target active Provider ID/revision/digest、projection ID与V272 conformance receipt pair，并按domain+NUL+JCS计算。
kind/time、V274 identity、process seal/session/Secret任一混入均拒绝。genesis与active-refresh必须走不同typed
constructors/current reproof；互换typed carrier即使material可重算也失败。

## 5. Pending plan、direct SQL 与 18-fence 矩阵

完整 current schema必须仍有18个具名trigger。exact permitted set只能是`#1,#5-#12`九个：Provider adjacent-active
UPDATE、active version、projection Adapter/version、service actor、route credential、route authorization、六 capability、
route seal。它们仅在同connection、non-deterministic variable-arity UDF
`elon_v275_external_pool_adapter_atomic_activation_pending_plan_matches(...) = 1`且row bytes与一次性plan逐字相等时通过；
该UDF flags exact为`SQLITE_UTF8 | SQLITE_INNOCUOUS`，绝无`SQLITE_DETERMINISTIC`。

`#2-#4,#13-#18`九个继续 absolute deny且不得查询UDF：active Provider INSERT、Provider identity/kind UPDATE、
CapacityPool active insert/update/version、Offer draft|active insert/update/version。验收必须逐个使用trigger exact name并证明
`9 pending-plan permits / 9 absolute denies`，不能只比较数量。

必须覆盖：pending UDF未注册、被错误注册为deterministic、返回错误/0/NULL、空registry、restart、wrong connection/token/root/row/order、partial plan、replay、
manual UDF/direct SQL、migration seed均失败关闭；exact Store kernel正向可行且plan仅消费一次。任何HTTP/service-only check、
caller token、临时DROP trigger或#2 active INSERT放行都是P0。

授权分工必须单独验收：全部mutation虽在同一Store kernel，V275 receipt+九类业务写只接受V275 one-shot plan；V274
append只接受独立V274 pending purpose-seal UDF。二者绑定同root/witness/dual time但互不替代；交叉代用、只注册一份
authority或任一direct-SQL路径均失败且零写入。

## 6. 原子事务与 fault matrix

时序必须逐步证明：external observation完成时只有non-authorizing evidence；final`BEGIN IMMEDIATE`取得writer lock后
才生成`evidence_checked_at`并完成fresh typed reproof/target-root-route-receipt重建；随后才注册one-shot V275 plan与
mint/remember V274 pending seal；17 mutations与same-tx readback后commit；same connection postcommit exact readback
成功后才promote seal并discard plan。writer lock/`evidence_checked_at`之前出现plan或seal，或raw-result wrapping代替
typed reproof，均为P0。

正向验收必须证明同一`BEGIN IMMEDIATE`内exact闭合：existing Provider adjacent-active UPDATE/version、projection
Adapter/version、service actor authorization、route credential、V221-source route authorization、六 capability、seal、
V275 receipt与V274 genesis。V275先写、V274后写，V274→V275 immediate FK在一次commit通过；只有既有Adapter/
credential内部cycle可保留deferred FK。commit后才promote purpose seal并
consume plan。

正向mutation计数必须逐行证明`16 INSERT + 1 CAS UPDATE = 17`：actor auth 1、Adapter root/version 2、credential
root/version 2、route authorization 1、capability 6、seal 1、Provider version 1、V275 receipt 1、V274 genesis 1，
以及existing registering→adjacent projected-active Provider CAS UPDATE 1。V253新receipt/outbox/Attempt/Pool/Offer/
usage/settlement全部为0；stable executor不另建row/table。

每个写点前后、canonical readback、foreign key、commit、promote gap都要注入故障。precommit或明确commit失败时，
所有业务对象与两份receipt row count必须恢复原值；不得出现active Provider无route、route无receipt、已commit V275
receipt无V274 witness或冲突的第二份V275 replay。commit不确定/postcommit readback失败不得谎报rollback或promote：
durable rows可作为historical，pending seal永不授权，plan必须discard/失效，恢复只能分类exact committed closure。
事务内网络/child/filesystem I/O固定失败审计。Provider/route虽原子提交，V273仍`eligible_rows=0`且不得产生
exchange-attempt、send-attempt、network、ACK/event或outbox claim。

## 7. V253、V274、restart 与撤销语义

| Case | 必须结果 |
|---|---|
| V253 genesis | current registering receipt + planned adjacent projection形成non-authorizing proof；不要求预存/current V274。 |
| V253 active | commit后只由durable V275 witness + historical root + live projected-active Provider门控；logical-active历史证据仍superseded。 |
| V274 freshness | 每次current消费均需fresh V270-equivalent active observation、fresh V272 private carrier、live root/Provider reproof与process seal。 |
| restart | 旧V270/V272/V274 custody立即失效；从V275 witness/root/live active Provider重新取证并append fresh V274 successor，不依赖旧V274 current。 |
| route expiry | V275只可复用仍current的genesis route；已撤销/过期route失败并等待V276 renewal。 |
| stop/revoke | Provider/route/capability/V274失效阻止future work，但V275永久receipt不撤销、不删除、不改写。 |

必须加入 V253↔V274 recursion negative test：任何 active V253 先要求 current V274，或 fresh V274 又先要求 active V253
current V274 wrapper，都应在源码/运行矩阵失败。V271 logical digest只可作为lineage。cleanup只能清除进程内pending
plan/seal；不得删除activation历史或既有authenticated attempt事实。

## 8. V276 与绝对零效果

V276才验收route renewal、worker reachability、per-attempt current reproof、V273正向ledger、ELTP send/ACK/event与
`eligible_rows>0`。V275矩阵不得包含Pool/Offer/market fence permit、Job/Reservation/Attempt/Lease、usage、settlement、
公网、生产Secret、部署、MCP/PC或production readiness。

在Rust/DDL/migration/Store实现、fresh/repeat/reopen、direct-SQL、restart、并发/crash、逐写点rollback与生产运行全部
完成前，正式结论保持：`design_frozen / source_review_only / implementation_uncompiled / implementation_unrun`，
`passed=0 / failed=0`、Provider=`registering`、`eligible_rows=0`。这是FORMAL DOCS FREEZE，不是activation完成声明。
