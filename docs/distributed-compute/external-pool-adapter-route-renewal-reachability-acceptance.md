---
title: 外部矿池 Adapter route renewal 与 production reachability 验收
status: current
reviewed_at: 2026-08-20
owners: backend, security, ai-economy
design_status: design_frozen
implementation_status: implementation_uncompiled
verification_status: source_review_only
---

# 外部矿池 Adapter route renewal 与 production reachability 验收

## 1. 当前证据强度

V278 Domain、DDL、Store、migration、default-off worker、typed ingress与source-contract源码已经落盘，但当前只有
source review；正式计数仍为`passed=0 / failed=0`，状态为`design_frozen / source_written /
source_review_only / implementation_uncompiled / implementation_unrun`。本批没有编译、测试、执行migration、
SQLite、runtime或network；Provider=`registering`、V273 `eligible_rows=0`、运行态V254 fence opened=`0`。

Docs、fixture、seed、直接SQL、临时删trigger或历史V271/V273/V274/V277 passed均不能计入V278。当前可记
`source_written/source_review_only`；在compile/migration/runtime矩阵完成前仍不可记implemented或reachable。

## 2. Namespace、schema 与 canonical matrix

| 验收面 | 必须结果 |
|---|---|
| durable shape | exact一张`compute_external_pool_adapter_route_renewal_receipts`；0 view、0 revocation、无mutable head/queue/session/Secret表。 |
| 77 columns | authority列出的77列逐名逐序存在；`active_successor_*`不得出现，必须是`activation_genesis_successor_*`。 |
| canonical | schema/domain/JCS/SHA-256 exact；canonical UDF接受exact row，未知字段、NULL漂移、digest不重算、大小写或时间格式漂移拒绝。 |
| genesis witness | 每行引用同root exact V274 sequence-1 historical pair；允许多次renewal复用，pair无UNIQUE且不要求current/process V274。 |
| immutability | receipt no-update/no-delete/no-replace；exact replay只readback。 |
| forbidden bytes | raw Secret、locator/token、MAC/session/nonce/address/body、Attempt/Lease或经济数据均不落表。 |

Fresh/repeat/reopen migration必须得到同一schema、trigger和UDF inventory；升级不得backfill、reinterpret或修改V277/V274
历史row。Existing active Provider若没有同事务V277 receipt不得事后伪造activation base。

## 3. Authority type 与 circularity matrix

| Case | 必须结果 |
|---|---|
| historical recovery | `HistoricalExternalPoolAdapterRouteRecoveryAuthority`只可audit/recovery/cleanup，任何fresh dispatch调用在类型或Store边界失败。 |
| renewed current | 只有`CurrentExternalPoolAdapterRenewedRouteAuthority`可进入fresh v211/v213 producer；receipt或bool不能构造。 |
| runtime composition | fresh prepare/commit claim/send只接受`ReprovedExternalPoolAdapterRouteAndActiveSuccessorAuthority`。 |
| historical cleanup | 旧send cleanup型可建ordinal-1 poll、claim poll、建reconcile/event exchange/receipt，或send既有cancel outbox；不得建prepare/commit/command/outbox intent或rebind。 |
| renewal roots | route renewal消费V277 historical root、live Provider、fresh V253 recovery；不查询current V274/V268/V272。 |
| active preparation | current route后才取得fresh V253/V268/V272并执行broker/Secret/no-work。 |
| V274 refresh | refresh从historical witness/root/live Provider开始，使用独立ordered plan；不得先要求旧V274 current。 |
| same-time join | V274 commit/readback/promote后，在单一checked-at同时重证route与V274；不得把route identity写进V274 receipt。 |
| selector/decision | unique historical head selector只返回owned pair；typed decision仅Current/RenewalRequired，fork/gap/root/delegation漂移报错，不能按error text分支。 |

必须有V253↔V274、route↔V274、V273↔route三组negative source contract；任一反向依赖或historical-to-current提升
均为P0。
必须另测P1双head耦合：V278 receipt写current refresh pair或V274 receipt写route pair均拒绝，两个head可独立轮换/重启。

## 4. Renewal transaction、lineage 与 fence matrix

正向transaction必须逐项证明`11 INSERT + 1 credential-root CAS UPDATE`：actor authorization 1、credential version 1、
route authorization 1、capability 6、seal 1、V278 receipt 1、credential root CAS 1。Provider、Adapter、V274、V253、
outbox、Pool、Offer、usage、settlement写入均为0。

| Fence set | 必须结果 |
|---|---|
| #1/#5-#7 | 仍只接受V277 plan；V278 plan和direct SQL均拒绝。 |
| #8-#12 | exact同一trigger只允许V277或V278 ordered plan；wrong purpose/order/row/connection拒绝。 |
| #2-#4 | absolute deny，不能调用V278 UDF。 |
| #13-#18 | CapacityPool/Offer继续absolute deny，不能调用V278 UDF。 |
| credential CAS | 独立V278 guard只允许old exact active root到next exact active revision，不能改变18项inventory。 |

Sequence 1必须以V277 genesis closure为predecessor；后续sequence必须精确引用前一V278 receipt/new closure。双写竞争
只能一个winner；sibling、gap、fork、旧head、wrong activation root、executor/projection/capability漂移全部拒绝。
TTL必须由server policy与fresh evidence取minimum，caller/env override拒绝；fresh-use止于`expires_at`，cleanup-only止于
`cleanup_expires_at`，renew-before/maximum任一不匹配`renewal_policy_digest`即拒绝。

每个写点、readback、FK、commit前后均须fault injection：precommit失败零row；commit不确定不得发送；postcommit
readback失败不得返回current authority。Restart后可full-row audit exact committed history，但必须fresh重证才可使用。

## 5. 四UDF、direct SQL 与 restart matrix

| UDF | 必须证明 |
|---|---|
| receipt canonical | 唯一deterministic UDF，exact arity 1；畸形JSON/摘要/投影拒绝。 |
| renewal plan | exact 12 ordered calls；capability按0..5；少、多、乱序、跨connection、replay拒绝。 |
| V273 reachability/ingress | outbound与terminal ingress使用同namespace的purpose-separated ordered plan；不能替代renewal/V274。 |
| V274 refresh | refresh append/readback/promote专用；不能签route、send或ingress。 |

三份plan registry必须connection-local、one-shot、RAII、restart-empty且非deterministic。V254 #8-#12显式`CASE`
选择V277或V278；V271 source trigger不得双消费plan。缺UDF、NULL/0/error、wrong purpose或直接SQL全部失败关闭。
Arity必须exact为`1/-1/-1/17`；V274 trigger exact WHEN必须是`NEW.successor_sequence > 1 AND udf(17 args) IS NOT 1`，
逐值匹配purpose与16个NEW字段，只追加到既有pending-seal trigger，不替代canonical/process-seal guard。Sequence 1
不得调用/消费refresh registry；UDF也拒绝sequence<=1或NULL predecessor。

Trigger inventory必须证明：V254只改#8-#12且显式V277-then-V278，另外13项不调用V278；V278 receipt只新增
canonical/immutability/no-replace/lineage guards和credential-root CAS guard；V273只向既有outbound/terminal guards
追加reachability plan，旧root/currentness/immutability条件逐字保留。

`v273_task_exchange_attempt_exact_authority`必须保留legacy fresh分支，并追加不消费第二次plan的historical cleanup
分支：poll逐字段绑定原durable send，cancel经`subject_outbox_id`绑定原prepare；两者共享V277/V278 exact cleanup witness
COUNT总和`=1`与严格`started_at < cleanup_expires_at`，并重证live Provider stable adapter/config pair；不得要求旧route
仍是current adapter head或delegation仍未撤销。no-replace trigger仍是本次ExchangeAttempt唯一plan消费者。

Historical Accepted四写必须按Actor→LeaseAuthority→Commit→Application消费，同名五trigger保留fresh分支并只在
exact authenticated receipt/observation/ACK/activation与唯一cleanup witness闭合时接受historical分支；NULL/0/error、
错receipt、错connection、乱序、route/root漂移或cleanup边界值全部rollback。Application live guard只做准入，closure
guard才消费Application，不能双消费。

## 6. Worker source、outbound 与 recovery matrix

唯一startup env必须仍为`ELON_EXTERNAL_POOL_ADAPTER_ATTEMPT_DELIVERY_ENABLED`且default-off；不得新增第二worker、
public API、callback/listener、path、Secret或bypass配置。

| Case | 必须结果 |
|---|---|
| source wiring | private连接V277 orchestrator、V253 recovery、renewal、active preparation、V274 refresh、claim/send/poll/ingress；caller不能提供authority。 |
| outbound count | start-outbox source exact 2 INSERT：v213 send-attempt+V273 exchange-attempt；1 CAS：outbox claimed→unknown、revision/attempt+1。 |
| atomicity | pair/CAS同一BEGIN IMMEDIATE和commit；任一失败三者rollback；commit后才允许ELTP。 |
| six tables | V273仍exact六表，V278无第七表。 |
| route rotation | old command/outbox不update/rebind；fresh work生成fresh command/outbox，old sent route只走historical cleanup。 |
| direct reconcile | 只能在send已durable且结果unknown后创建，第一份固定ordinal 1；send前预造或盲重发拒绝。 |
| restart | session/plan/V274 custody失效；fresh route runtime carrier后才可继续new send，historical cleanup不要求route current。 |

Builder验收必须证明worker/caller ID完全不在signature/receipt/idempotency；只接受typed `RenewalRequired`并由S1事务内生
全部ID。Current route getter均borrowed，runtime TTL严格截断到route leaf与V250/V252/V253/V268/V272/15秒minimum。

物理exchange不是exactly-once。Outbound commit后无authenticated receipt一律unknown；prepare/commit不得盲重发，
只能direct reconcile。Exact receipt/event replay去重；digest、cursor、remote sequence fork terminal。

## 7. Terminal ingress 与现有状态机

Accepted terminal ingress必须在同一transaction持久化authenticated V273 receipt并接入既有v211 ACK、v185 activation、
v215 accepted closure；任一子步骤失败全部rollback。不得新增ACK、Lease、application、commit、Runner或accepted表，
不得让HTTP status、EOF、日志、本地timeout或caller JSON形成ACK。

验收必须证明route/executor/fence/session/nonce/ordinal/exchange root exact，且accepted closure继续复用既有Claim、Job、
Reservation、Lease门卫。由于本批market source不可达，该正向动态链后移；source contract可以写全，但不得据此宣称
真实ACK、Lease或Runner已通过。

## 8. Admission blocker 与明确后移

V254 #13-#18不开放，故normal Offer→Job→Reservation/Lease→v211/v213 producer在V278批次不可达。以下不得计入
V278动态passed：

- `eligible_rows>0`；
- 真实Pool/Offer/Job/Reservation/Attempt source；
- production ELTP/ACK；
- accepted Lease/Runner；
- usage、market、settlement、余额、付款或链上效果。

这些正向验收只能由独立service-managed admission + Runner bridge批次完成。Fixture、seed、直接SQL、mock receipt、
临时删/缩fence均为无效证据。

## 9. 正式结论

V278当前可声明的是：exact一表/零view/零revocation、77列、四UDF、renewal 11+1、outbound 2+1、V254
permit/deny矩阵、historical/current/runtime分型、default-off接线与typed ingress源码已经落盘并完成source review。

当前不能声明migration可用、route已续签、V274已刷新、worker已运行、ACK/Lease/Runner已接入或任何经济效果。
sealed semantic ingress尚无production validator/caller。正式状态保持`design_frozen / source_written /
source_review_only / implementation_uncompiled / implementation_unrun`、`passed=0 / failed=0`、
Provider=`registering`、`eligible_rows=0`。
