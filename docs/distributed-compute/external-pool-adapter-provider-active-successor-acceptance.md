---
title: 外部矿池 Adapter Provider active successor 验收
status: current
reviewed_at: 2026-08-20
owners: backend, security, ai-economy
design_status: design_frozen
implementation_status: implementation_uncompiled
verification_status: source_review_only
---

# 外部矿池 Adapter Provider active successor 验收

## 1. 当前证据强度

V274 pre-change dormant overlay 曾完成 7/7 本地验收，指纹为
`9c363ccc6271005b6154d6ae230a34ed2da97b8335c130e9d67998d7632c9ffe`；它仅是 historical/superseded evidence，
不能证明当前双时间、V274→V277 immediate witness、private append/current或planned genesis新ABI。

当前源码状态为`source_written / source_review_only / implementation_uncompiled / implementation_unrun`，
`passed=0 / failed=0`；HTTP/startup/filesystem/Linux child/network、process HMAC custody、crash、concurrency、
V277 transaction与production runtime均未运行。

本页只定义验收门；唯一语义来源是
[`external-pool-adapter-provider-active-successor-authority.md`](external-pool-adapter-provider-active-successor-authority.md)。

## 2. Static shape 与 reachability 门

静态合同必须同时证明：

- durable namespace exact只有 `compute_external_pool_adapter_provider_active_successor_receipts`、
  `_revocations` 两张完全 immutable表与 `_current` 一个诊断 view；无 mutable head/queue；
- migration不得 seed row，V277前两表和view均为零行；Store-private `_on(&Transaction, ...)` ABI没有 HTTP、MCP、
  WebSocket、owner/admin、startup、worker、public DTO或通用 Store facade调用方；
- insert后 exact scalar/canonical readback；`UPDATE`、`DELETE`、`INSERT OR REPLACE`、lineage fork、跨 root target、
  canonical drift与非 exact replay均被拒绝；
- view只声称 `relationally_current_requires_process_custody_and_active_root_reproof`，不能被任何 producer/currentness
  constructor消费；
- Provider=`registering`、V273 `eligible_rows=0`、V254 18 deny逐字不变、打开 fence=`0`。

任何“临时 admin prepare POST”“先插 pending successor”“migration success等于active”都是 P0 失败。

## 3. Stable activation-root 矩阵

| Case | 必须结果 |
|---|---|
| exact structural origin | uppercase V1 domain、outer RFC8785 envelope及全部 V249/V254/V255/V258/V259 pair、launch/target/supervisor/entrypoint policy、V272 static task profile与V273 lane/carrier policy exact，摘要稳定。 |
| Provider serialization | source与target使用 exact `serde_json::to_string(&ComputeProvider)` bytes和现有 Provider digest；不得改成 Provider JCS、caller JSON或字段重排。 |
| logical→projection | source registering adapter=logical；initial/adjacent active adapter=`route_adapter_projection_id`；compatibility digest exact且永远不比较 logical==projection。 |
| structural drift | release/binding/candidate/profile/target/companion/policy/image/task profile/lane/carrier任一变化产生不同 root并阻止旧 route复用。 |
| forbidden input | V250/V252/V253/V268/V270/V272 renewable receipt、process seal/TTL、Secret/session、executor/route/capability/fence/attempt/lease任一进入 stable root即失败。 |

Provider target还必须验证 `policy_revision=source+1`、status=`active`、
`updated_at=activation_target_updated_at`，除 adapter
projection与这三个transition字段外其它 typed Provider字段 exact保持。wrong owner/release/config/settlement、跳
revision或 logical-active 一律失败关闭。双时间必须满足
`source.updated_at <= activation_target_updated_at <= observation_started_at <= observation_completed_at <= evidence_checked_at < observation_expires_at`；preflight冻结target/root，final才生成`evidence_checked_at`并重验，不得折叠为单一
`checked_at`或用final时间改写target。

## 4. Renewable evidence 与 circularity 矩阵

| Case | 必须结果 |
|---|---|
| V277 genesis prepare | registering source、planned adjacent active target、fresh V253 registering transition proof、pending V270-equivalent genesis observation与fresh V272 genesis carrier形成仅进程内non-authorizing overlay；数据库仍零V274 row。 |
| V277 commit | 同一 `BEGIN IMMEDIATE` 原子闭合 exact active Provider/route/executor、V277 receipt、V274 genesis与`9 pending-plan permits / 9 absolute denies`；commit后same-connection readback才promote purpose seal。 |
| genesis pair distinction | credential-observed registering Provider pair与runtime/task evidence planned-active pair不强等；transition由activation root+V277 transaction证明。 |
| active refresh | V278设计要求dedicated ordered plan追加sequence>1 successor；live Provider必须active且adapter=projection，fresh V253/V268/V272、active observation与stable root在`evidence_checked_at`重验；不要求旧V274 current。 |
| runtime freshness | observation完成后最长15秒，且不晚于任何输入到期；历史V270 registering receipt不能冒充active observation。 |
| V272 neutrality | public/canonical V272 receipt保持Provider-neutral；private digest用`ELON-EXTERNAL-POOL-ADAPTER-TASK-PROTOCOL-ACTIVE-CARRIER-V1` exact十字段material，genesis/refresh走不同typed constructors；refresh直接消费V277 witness+root，绝不依赖current V274。 |
| one-way witness | V277 canonical/DDL不含V274 receipt identity或反向FK；V277自身receipt/root三元组为UNIQUE parent，V274 witness/root三元组以immediate FK引用它，先写V277、后写V274。 |
| V278 genesis reference | 每张route receipt引用exact sequence-1 historical id/digest；字段为`activation_genesis_successor_*`，不加UNIQUE、不要求current/process V274，普通refresh pair不得写入。 |

任何 SQLite transaction、connection、Prepared/Store authority跨 filesystem/network/child/await，或在外部观察后不做
final same-connection `evidence_checked_at` reproof，均失败；raw-result wrapping不能替代typed current authority。

## 5. Purpose seal、restart 与 lineage 矩阵

| Case | 必须结果 |
|---|---|
| rollback | 零durable successor且pending永不授权；允许TTL prune/best-effort cleanup，不能promote，安全性不依赖显式删除。 |
| pending order | observation完成仍无plan/seal；final writer lock+`evidence_checked_at`+fresh typed reproof后才注册V277 plan并mint/remember V274 pending seal。 |
| commit/promote gap | 17 mutations/same-tx readback/commit后，仅same connection postcommit exact readback可promote并discard plan；不得造第二行。 |
| restart | 旧seal/epoch立即historical，即使TTL未过；fresh Prepared、active observation、V272与新successor全部必需。 |
| exact replay | 同actor/idempotency与全部bytes exact只readback；任何差异冲突。 |
| refresh | sequence单调、predecessor exact head；Provider任意active `policy_revision`变化都使旧receipt historical。 |
| revoke | append-only revocation阻止future consumption但不改Provider/route/market/历史task facts；revoked/expired head本身永不current。 |
| disabled status | `draining|quarantined|disabled` 均拒绝prepare/refresh；诊断view不能绕过。 |

V278 refresh UDF必须以arity 17、non-deterministic/INNOCUOUS注册，只追加到既有receipt pending trigger；exact WHEN
为`NEW.successor_sequence > 1 AND udf(17 args) IS NOT 1`，purpose与16个NEW字段逐值type/length/bytes exact。
Sequence 1不得调用UDF/消费registry；缺注册、wrong connection/order/sequence/predecessor、replay或试图替代
canonical/process-seal guard均失败关闭。

## 6. Narrow bridge 验收

V253 registering path保持live adapter exact等于logical。Genesis必须走独立Store-private transition-proof helper，
只消费current registering V253 receipt、planned projected adjacent target与pending activation closure，不得要求预先存在
V274 row。普通active current/challenge/DDL guard直接以durable V277 activation witness+historical activation root为门，
要求live adapter exact等于`route_adapter_projection_id`；release/credential lineage仍指向logical。任何
`logical==projection`断言、V253↔V274递归或未受activation witness/root约束的active分支都是P0失败。

V249-V270既有 receipt/API/current view保持历史语义。V274只能新增显式 Store-private active carrier/reproof，不能把
registering row广泛解释为active，不能让历史 V270 receipt续命。V272 canonical ABI不变；active carrier缺失、过期、
wrong process/root或Provider revision漂移均拒绝。

本批只能发现dormant ABI：durable V277 witness不存在时，V270-equivalent committed-active minter、V253 ordinary
projected-active branch（含旧logical-active形状）、active carrier、refresh/revoke/current consumer都必须失败关闭。

## 7. V277/V278 后继门

V277正向验收必须证明 stable executor、route projection Adapter/version与v213 route credential、V253
projected-active transition proof、service actor、route authorization、六 capability、seal、紧邻 active Provider、
V277 receipt、V274 genesis及18-fence exact九项pending permit/九项absolute deny同一
`BEGIN IMMEDIATE`/同commit，任一 fault全部 rollback。#2 active Provider INSERT与#3-#4/#13-#18不得放行。
V274本批不得用 mock row提前证明该事务。

V278顺序必须是V277+sequence-1 historical base → route renewal（不要求current V274/V268/V272）→ current route →
fresh V253/V268/V272+active preparation → dedicated refresh/promote → same-time route+V274 → V273。#13-#18仍deny，
所以本批不得动态声明真实eligible row、ELTP ACK、Lease或Runner；这些后移service-managed admission+Runner bridge。
Pool/Offer、usage、market、settlement、部署与跨进程外签authority也不属于V274 passed。

## 8. 正式结论

V274 pre-change dormant overlay 的统一合同/迁移曾为 7/7，指纹
`9c363ccc6271005b6154d6ae230a34ed2da97b8335c130e9d67998d7632c9ffe`；该结果仅保留为
historical/superseded evidence，不能证明双时间、V274→V277 immediate witness与private append/current新ABI。
V274 当前只能声明“两张 immutable表、一个非权威诊断view、stable activation root、双时间target、private
genesis/refresh append与typed current源码已铺设”。planned genesis real-I/O源码路径可达但未运行；durable active
restart/refresh缺purpose-specific broker/Secret preparation而失败关闭。它不能声明row已产生、Provider active、
route/executor存在、fence已打开或任务可派发。正式状态保持`source_written / source_review_only /
implementation_uncompiled / implementation_unrun`、`passed=0 / failed=0`、Provider=`registering`、
`eligible_rows=0`；V277替换9项fence的源码未执行，当前运行态打开fence仍为0。
