---
title: 外部矿池 Adapter Provider active successor 验收
status: current
reviewed_at: 2026-08-16
owners: backend, security, ai-economy
design_status: design_frozen
implementation_status: implementation_uncompiled
verification_status: source_review_only
---

# 外部矿池 Adapter Provider active successor 验收

## 1. 当前证据强度

V274 当前只接受 authority合同的静态复核。本批没有编译 Rust、执行 migration、运行 SQLite/HTTP/startup/
filesystem/Linux child/network/HMAC/crash/concurrency/restart/V275 transaction，也没有动态指纹。正式计数为
`passed=0 / failed=0`，状态为 `source_review_only / implementation_uncompiled / implementation_unrun`。

本页只定义验收门；唯一语义来源是
[`external-pool-adapter-provider-active-successor-authority.md`](external-pool-adapter-provider-active-successor-authority.md)。

## 2. Static shape 与 reachability 门

静态合同必须同时证明：

- durable namespace exact只有 `compute_external_pool_adapter_provider_active_successor_receipts`、
  `_revocations` 两张完全 immutable表与 `_current` 一个诊断 view；无 mutable head/queue；
- migration不得 seed row，V275前两表和view均为零行；Store-private `_on(&Transaction, ...)` ABI没有 HTTP、MCP、
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

Provider target还必须验证 `policy_revision=source+1`、status=`active`、`updated_at=checked_at`，除 adapter
projection与这三个transition字段外其它 typed Provider字段 exact保持。wrong owner/release/config/settlement、跳
revision或 logical-active
一律失败关闭。

## 4. Renewable evidence 与 circularity 矩阵

| Case | 必须结果 |
|---|---|
| V275 genesis prepare | registering source、planned adjacent active target、fresh V253 registering transition proof、pending V270-equivalent genesis observation与fresh V272 genesis carrier形成仅进程内non-authorizing overlay；数据库仍零V274 row。 |
| V275 commit | 同一 `BEGIN IMMEDIATE` 原子闭合 exact active Provider/route/executor、V274 genesis与18-fence replacement；不承诺行级先后，一次commit后purpose seal才pending→committed。 |
| genesis pair distinction | credential-observed registering Provider pair与runtime/task evidence planned-active pair不强等；transition由activation root+V275 transaction证明。 |
| future active refresh | 只在future V275/V276 transaction-bound path中，live Provider必须active且adapter=projection；fresh V253 active receipt、fresh V270-equivalent observation、fresh V272 carrier与stable root同checked-at重验；V274无独立producer。 |
| runtime freshness | observation完成后最长15秒，且不晚于任何输入到期；历史V270 registering receipt不能冒充active observation。 |
| V272 neutrality | public/canonical V272 receipt保持Provider-neutral；genesis private carrier绑定planned target，restart/refresh carrier直接消费durable V275 witness+historical root再由V274 wrapping，绝不能依赖current V274形成递归。 |

任何 SQLite transaction、connection、Prepared/Store authority跨 filesystem/network/child/await，或在外部观察后不做
final same-connection reproof，均失败。

## 5. Purpose seal、restart 与 lineage 矩阵

| Case | 必须结果 |
|---|---|
| rollback | 零durable successor且pending永不授权；允许TTL prune/best-effort cleanup，不能promote，安全性不依赖显式删除。 |
| commit/promote gap | 仅同进程同exact pending entry可exact readback后promote；不得造第二行。 |
| restart | 旧seal/epoch立即historical，即使TTL未过；fresh Prepared、active observation、V272与新successor全部必需。 |
| exact replay | 同actor/idempotency与全部bytes exact只readback；任何差异冲突。 |
| refresh | sequence单调、predecessor exact head；Provider任意active `policy_revision`变化都使旧receipt historical。 |
| revoke | append-only revocation阻止future consumption但不改Provider/route/market/历史task facts；revoked/expired head本身永不current。 |
| disabled status | `draining|quarantined|disabled` 均拒绝prepare/refresh；诊断view不能绕过。 |

## 6. Narrow bridge 验收

V253 registering path保持live adapter exact等于logical。Genesis必须走独立Store-private transition-proof helper，
只消费current registering V253 receipt、planned projected adjacent target与pending activation closure，不得要求预先存在
V274 row。普通active current/challenge/DDL guard直接以durable V275 activation witness+historical activation root为门，
要求live adapter exact等于`route_adapter_projection_id`；release/credential lineage仍指向logical。任何
`logical==projection`断言、V253↔V274递归或未受activation witness/root约束的active分支都是P0失败。

V249-V270既有 receipt/API/current view保持历史语义。V274只能新增显式 Store-private active carrier/reproof，不能把
registering row广泛解释为active，不能让历史 V270 receipt续命。V272 canonical ABI不变；active carrier缺失、过期、
wrong process/root或Provider revision漂移均拒绝。

本批只能发现dormant ABI：durable V275 witness不存在时，V270-equivalent committed-active minter、V253 ordinary
projected-active branch（含旧logical-active形状）、active carrier、refresh/revoke/current consumer都必须失败关闭。

## 7. V275/V276 后继门

V275正向验收必须证明 stable executor、route projection Adapter/version与v213 route credential、V253
projected-active transition proof、service actor、route authorization、六 capability、seal、紧邻 active Provider、
V274 genesis及18-fence replacement同一
`BEGIN IMMEDIATE`/同commit，任一 fault全部 rollback。V274本批不得用 mock row提前证明该事务。

V276才验收 V273 worker/ingress到真实v213 eligible rows、ELTP ACK/event与downstream closure；V274/V275成功都不能
倒推 transport动态通过。Pool/Offer admission、usage、market、settlement、部署与跨进程可携带外签 authority也不属于
V274 passed。

## 8. 正式结论

V274 当前只能声明“两张 immutable表、一个非权威诊断view、stable activation root、renewable active evidence与
V275原子消费边界的合同已冻结”。它不能声明 row已产生、Provider active、route/executor存在、fence已打开或任务可
派发。正式状态保持 `source_review_only / implementation_uncompiled / implementation_unrun`、
`passed=0 / failed=0`、Provider=`registering`、`eligible_rows=0`、18 fences unchanged。
