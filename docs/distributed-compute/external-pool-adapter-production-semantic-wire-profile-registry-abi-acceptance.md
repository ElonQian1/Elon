---
title: 外部矿池 Adapter production semantic wire profile registry ABI 验收
reviewed_at: 2026-08-21
status: current
owners: backend, security, ai-economy
design_status: design_frozen
design_scope: external_adapter_production_semantic_wire_profile_registry_abi_v1
implementation_status: implementation_unwired
verification_status: design_review_only
---

# 外部矿池 Adapter production semantic wire profile registry ABI 验收

## 1. 当前证据与状态边界

唯一权威是 [registry authority](external-pool-adapter-production-semantic-wire-profile-registry-abi-authority.md)。本页只验设计，
不得登记compile/test/migration/SQLite/child/TLS/Runner通过数。当前必须同时为：

```text
external_adapter_semantic_wire_profile_registry_abi=design_frozen
external_adapter_semantic_wire_profile_approval_evidence_abi=design_frozen
initial_external_adapter_semantic_wire_profile_approval_evidence_set=unselected
initial_external_adapter_semantic_wire_profile_inventory=unselected
external_adapter_semantic_wire_profile=unselected
current_external_adapter_semantic_wire_profile_authority=unconstructible
implementation=unwired/uncompiled/unrun
passed=0 failed=0
```

Migration最大值仍V279；本页planned Rust目录、catalog、profile/evidence instance、request builder/authorizer、五validator impl与production
child均不存在。V273六表/八根、Tx-A/Tx-B rows、V254 #13-#18与Provider eligibility零变化。

## 2. Canonical、ID 与 release/base-protocol matrix

| case | 必须结果 |
|---|---|
| profile shape | exact 7-key envelope、5-key nested profile；unknown/missing/type substitution拒绝 |
| profile ID | exact V249 release triple+implementation+operation name/code+revision七键；revision=1 |
| profile digest | self key保留空串，domain+NUL+JCS；普通serde/hash或删除key拒绝 |
| review material | final profile digest与review source digest双blank，无环且不删key |
| V249 binding | release 9-key projection逐字来自同一deep-audited receipt；cross-release/implementation splice拒绝 |
| ELTP | magic/version/flags=`ELTP/1/0`，operation count=5，ordinal=64，global ceilings=`262144/65536/262144/262144` |
| protocol root | task-protocol triple来自同一retained V272 server-owned conformance/base profile；synthetic semantic fixture拒绝 |
| bytes/types | exact UTF-8 JCS/I-JSON、safe integer、64 lowercase digest、canonical UTC nanos；duplicate/float/trailing拒绝 |

Golden必须分别覆盖五个operation，并证明同release不同operation产生不同profile ID；同operation任一V249 root、implementation、code或
revision漂移也产生不同expected ID而不能当replay。

## 3. Operation/ref 与未选择载荷 matrix

| case | 必须结果 |
|---|---|
| operation set | `prepare=1,idempotent_commit=2,cancel_no_start=3,reconcile=4,authenticated_events=5`，无缺失/重复/重排冒充 |
| operation shape | exact 16 keys；每个ref exact `id,revision,digest`，非空/positive/lowerhex |
| modes | 三个actual mode均unselected；未来各自只允许`canonical_json|opaque_bytes`，fixture不能代选 |
| bounds | 四项positive且不超global ceiling，timeout `1..=15000ms` |
| JSON | deny duplicate/unknown/missing/trailing、显式null、safe integer、parse→JCS byte-equal |
| opaque response | raw只在Zeroizing custody；按selected observation mode交approved contract，Store只收canonical semantic projection/len/SHA |
| placeholder | fixture、zero digest、待填ref、caller/env selector、未获批contract全部拒绝 |

每个ref还必须验fresh exact-one approved/current与historical exact-one retained resolution；0/multi、bytes/digest drift、owner替换、
current ref解释old send或悬空三元组都失败。Profile approval不能替代referenced item各自的purpose-specific authority。

当前五类actual key set、ref values、vendor mode、bound values与profile bytes必须报告`unselected`；schema/reference ABI冻结不得被写成
concrete wire已选择或validator已可实现。

Timeout必须覆盖`1/14999/15000` accepted与`0/15001` rejected，并证明Tx-preflight按paired clock把selected duration纳入existing cutoff
的min；final reproof值/cutoff漂移、child/TLS后relative reset、BEGIN/gate2/validator/receipt各自新建15s均拒绝。Historical cleanup使用
target operation retained timeout且整次exchange只共享一个absolute deadline。

## 4. Approval evidence 与 DAG matrix

| case | 必须结果 |
|---|---|
| approval shape | exact 7-key envelope、15-key nested approval、3-key ID material；revision=1 |
| subject | evidence逐字段绑定paired profile ID/revision/review-material、V249 triple、implementation与operation，并重算approval ID |
| actors | 两个distinct authenticated `admin|owner` session；caller/body/env/service actor/Provider owner/fixture拒绝 |
| times | submit/approve各自server clock，V249 registered/ref approval `<=submitted_at<=approved_at`；replay不采新钟 |
| DAG | profile ID→approval ID→双blank review material→blank-self evidence→source digest→final profile digest |
| splice | Profile A ID、Profile B material、另一release/operation/evidence source任一交叉拼接拒绝 |
| final digest | evidence不得携带final profile digest；fixed-point或把final digest塞回evidence拒绝 |

当前 evidence catalog exact empty；canonical字符串不算真实登录审批，market-profile/V249/V268/V272 evidence均不得复用。

## 5. Current set 与 historical replay matrix

| case | fresh first-send | durable send后cleanup |
|---|---|---|
| selector | V249 exact triple+implementation+operation | 从V273 exchange+exact-one renewed或genesis historical route branch重建五元组与base protocol |
| cardinality | current index 0=unconstructible，1=deep audit，multi/split拒绝 | retained pair必须exact 1；0/multi/drift=integrity failure |
| five-set | 同release五operation各1且base_protocol逐字相同才产sealed current set | original receipt审original op；follow-up cleanup按sealed target op选retained pair |
| currentness | current V249/V272+approved index | 不要求current/unrevoked；与sealed cleanup authority组合后只授权目标cleanup op |
| fallback | 禁same-release轮换、latest、adapter-ID-only fallback | 禁用新release/profile解释旧exchange |
| mutation | current membership只控制本build新send；本页不承诺anti-rollback | pair retained；删除、改写、重绑release/operation全部拒绝 |

Typed catalog必须逐项验retained item exact 8 fields与current index exact 11 fields；index指向不存在/不同pair、同selector多entry、同pair
不同canonical bytes或把index当新的digest/DB authority均拒绝。

Semantic变化必须新V249 release；同release不同profile revision、同selector不同bytes或五类中一类latest替换均失败。Profile/evidence pair、
selected operation、current five-set与historical authority均non-Clone/non-Debug/non-Serde，无通用`into_parts`。

Fresh 0/multi/missing必须在child/TLS/outbound前失败。Final事务前必须由planned selection唯一产同一non-authorizing prepared body+
`PreparedExternalPoolAdapterTaskRequest`/digest；V213 send-attempt与V273 exchange identity写exact同一digest并commit后，原prepared token才可与committed retained
selection/StartSend authority线性提升，gate2/validator/restart不得重查current。Post-commit retained item缺失或漂移时零application write、
保留unknown；cleanup horizon到期只隔离/人工处置，不得no-start、释放或换profile。Bootstrap必须证明V249受控注册→canonical receipt
export→profile approval→compiled catalog→部署exact replay；runtime-generated release ID不得由source placeholder或deployment latest替代。

Fresh动态/静态合同还必须覆盖两阶段：Tx-preflight只产owned non-authorizing plan，DB/current authority不跨await；child/TLS后先从plan
确定性prepare同一body/digest，再由final outbound事务重新full-audit exact same current pair与prepared commitment。两次间发生index/V249/
V272/ref/body漂移时必须rollback、close/reap、zeroize、0 ELTP/app write/outbound；commit成功才可把原token提升为authorized body。

## 6. 两道 pre-send 与后验 validator matrix

| stage | 必须authority | 失败结果 |
|---|---|---|
| final tx前/ELTP BEGIN前 | planned selection+durable material→non-authorizing prepared body/digest；tx写同一digest，commit后原token+committed authority才提升；begin只返回唯一continuation | rollback zeroize且0 BEGIN；durable send已commit后失败则unknown→reconcile |
| TLS app write前 | 线性消费continuation，重算expected request，并核selector/source/exchange/send/session/deadline+response policy | 0 app write；durable unknown保留 |
| receipt ingress前 | host receipt request len/SHA等于authorized token | mismatch拒绝ingress并reconcile |
| response/observation | 同一profile concrete validator同时消费actual response与child observation | response-only/observation-only拒绝 |

Committed carrier必须逐operation收敛：prepare/idempotent_commit/cancel_no_start只接
`StartSend(CommittedExternalPoolAdapterTaskOutbound)` paired V213+V273，reconcile/authenticated_events
只接PollExchange(claimed poll+V273)；wrong variant、缺任一paired row或跨operation复用均拒绝。

Cancel、reconcile与authenticated-events还必须证明target-op retained builder先产prepared body/digest。Cancel仅在paired V213+V273 commit中
首次持久化digest；poll两类先写intent，再由claim/final reproof/V273核同一stored digest。只有尚无attempt的poll intent可重构body继续；
StartOutbox commit前token loss只rollback后重新prepare，任一attempt commit后token loss必须unknown→新cleanup，禁止原BEGIN/重发。

负例必须覆盖raw/caller body、commit后重建第二body、仅凭stored digest造token、wrong committed carrier、prepared/stored digest不等、profile在response后才选择、first token/continuation复用/Clone/拆分、wrong command/outbox/delivery-attempt/route/executor/
fence/request digest、selector/source/exchange/session/deadline/encoder commitment或expected response漂移、请求已写后标`local_never_sent`、raw bytes进入Store、original protocol
root mismatch，以及historical cleanup改用current profile或original operation代替target cleanup operation。

## 7. Ownership、静态合同与晋级门

Source-contract必须锁唯一production path：worker取得planned current set/operation→server prepared body/digest→final reproof+durable outbound
commit→同一token authorized promotion→child request→broker one-shot authorizer→TLS write→response+observation validator→V273 receipt ingress；
recovery只经retained historical pair确定性重构并匹配stored digest。Domain最宽
`pub(crate)`、Store只`pub(in crate::store)`，generic raw constructor、HTTP/env selector、fixture与第二caller为0。

本页完成只可登记两个ABI `design_frozen/design_review_only`。只有首个五operation profile/evidence set、所有referenced contracts与实现、
issuance provenance、catalog、两道authorizer、five validators、production child、Gateway/worker/ingress/recovery与source-contract同批落盘，
才可进入`source_written/source_review_only`；再分别验compile、next-free migration、SQLite、child/network与positive end-to-end。

## 8. 必须保持为零的副作用

本设计不得新建table/index/UDF/trigger/view，不得预占物理V280编号，不得改V273 carrier/session roots，不得创建或激活Provider、Pool、
Offer、Snapshot、Job、Reservation、Attempt、Lease、Runner、usage或settlement，不得写Secret/raw payload或开放HTTP/MCP/WebSocket/fence。
