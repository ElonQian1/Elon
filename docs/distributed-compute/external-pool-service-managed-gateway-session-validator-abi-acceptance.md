---
title: 外部矿池受管 Gateway、task session 与 production validator 内部 ABI 验收
status: current
reviewed_at: 2026-08-21
owners: backend, security, ai-economy
design_status: design_frozen
design_scope: gateway_session_validator_internal_and_attempt_fence_abi_v1
implementation_status: implementation_unwired
verification_status: design_review_only
---

# 外部矿池受管 Gateway、task session 与 production validator 内部 ABI 验收

## 1. 当前证据边界

唯一权威是 [authority](external-pool-service-managed-gateway-session-validator-abi-authority.md)。当前只验内部设计；不得登记
compile/test/migration/SQLite/child/runtime/network/Runner通过数。本批静态真值必须同时为：

- migration最大值V279，V280 table/UDF/trigger/source为0；
- Gateway final builder、production fence、8-root production child/session、5 concrete validator impl与worker caller为0；
- V254 #13-#18打开数0，Provider仍不能由本文变成eligible；
- market-profile approval evidence ABI已design-frozen，但initial evidence、market inventory与external semantic wire profile均`unselected`；
- semantic wire registry/approval-evidence元ABI已design-frozen，但initial wire evidence/inventory与actual五operation profile仍`unselected`；
- `implementation_unwired/uncompiled/unrun`，`passed=0/failed=0`。

## 2. Fence canonical matrix

| case | 必须结果 |
|---|---|
| golden | exact 7-key material、attempt 6-key、execution_plan 4-key经domain+NUL+JCS得到固定lowercase 64hex |
| source | 全字段来自同一audited Plan+seal；executor/stable runner、capability、Offer与route binding交叉一致 |
| generation | 首批attempt_no/fencing_generation只接受1；0、2、caller/retry自增拒绝 |
| no cycle | command/outbox/ref/hint/Lease/V273/session/time/rolling route均不进入preimage |
| substitution | Provider/executor/stable binding/route binding/Attempt/Plan/seal任一漂移拒绝 |
| fake root | fixture fence、random、admission digest、route credential或V254 inventory拒绝 |
| replay | fresh/Tx-B replay/reopen/historical cleanup/accepted closure/Runner handoff均重算同一值 |
| durable | 仅V273 attempt/receipt/reconcile-poll/event-poll直存；batch/event沿FK继承，v1 command/outbox/LeaseBinding不增第二真源列 |

类型门必须证明`VerifiedComputeAttemptProductionFence`私有、non-Clone/non-Debug/non-Serde；Store recompute加载完整
Plan+seal+capability+historical Offer/admission/V277 root，Domain constructor只接受这组audited sources并仅被V280 orchestrator
调用。只传Plan+seal或V273 generic factory未比较重算fence，source_written门失败。

## 3. Gateway A/B matrix

| case | Tx-A | Tx-B |
|---|---|---|
| lookup first | 先查attempt lease与job/attempt | 先查provider/key与job/attempt双unique |
| exact replay | historical full audit，0 current read、0 ID/time、0 write | stored command/actor/outbox/ref-hint/fence readback，0 current route、0 ID/time、0 write |
| fresh | current admission+business+fresh V274/V278后才构造final validated inputs | none后才fresh route/actor/issuer/fence并构造final dispatch |
| owner token | builder直接返回`ValidatedComputeAttemptExecutionPlanInputs` | inner owner tokens后最终返回`ValidatedComputeAttemptStartDispatch` |
| write rows | `3+2A`，首批A=0为3 | actor+prepare outbox+command=3；总计6 |
| conflict | 双identity split、same identity different material拒绝 | 两把unique不同row或material漂移拒绝 |

必须有transaction-aware Store `_on` kernel；调用现有自开事务facade、A+B暗并、复制SQL、token-before-lookup、raw DTO拼私有
字段或只有builder没有worker caller均失败。Tx-B还要覆盖provider-owner activation actor≠service actor、issued_at不早于seal、
ref/hint锁同一historical issuer与typed fence；route owner必须由同一五份canonical envelope一次deep audit后返回不可复制的
`AuthorizedComputeRouteDispatchSources`，start-outbox整体消费，独立route/actor输入、通用`into_parts`、raw actor重建及
extra/missing route capability均拒绝。

## 4. Session、custody 与 deadline matrix

| case | 必须结果 |
|---|---|
| roots | V273 exact八根raw32顺序、argv、transcript/KDF golden逐字复用；0第九根 |
| isolation | task child/Secret/TLS fresh；V278 no-work六根child/channel复用拒绝 |
| await | DB、transaction、borrowed authority、Prepared、raw Secret不跨；owned child/session/TLS/deadline/join custody线性跨await |
| lexical host | Host/HostExchange只在dedicated blocking worker内借owned session；不得成为跨await字段或阻塞Tokio executor |
| cleanup | timeout、validator失败、child退出、route loser、panic-safe drop均shutdown/kill/reap/join/zeroize |
| outbound | 同tx claim CAS + 2 INSERT + outbox CAS，总2 INSERT+2 CAS；commit后才socket write |
| deadline | 单一absolute deadline取operation适用cutoff最小；fresh profile/route窗不错误阻断historical cleanup，不重开15s |
| core seam | V280只调用`begin_before(..., absolute Instant)`；relative begin与独立DNS/TCP/TLS timeout不得越界 |
| expiry | preflight已过期且未启child/TLS=0 network；准备中跨越则close/reap且0 application request；durable unknown只reconcile |

验收必须覆盖paired wall/monotonic构造、relative-begin reset负例、各DNS/TCP/TLS stage timeout收紧、cutoff在child/TLS期间
跨越时关闭且application request=0、socket write前跨越、validator进入/
返回跨越与receipt timestamp越界。
Offer/snapshot new-plan expiry不得错误阻断已seal Plan恢复。

## 5. 五类 concrete validator matrix

| operation | output只允许 | 必须失败 |
|---|---|---|
| prepare | semantic view只验证Store-built accepted ACK+remote observation | rejected/commit/event/no-start分支、response↔observation不一致 |
| idempotent_commit | first event poll | 第二remote identity、直接Runner/Lease、prepare outcome |
| cancel_no_start | first reconcile poll | cancel ACK直接no-start/释放 |
| reconcile | exact四分支之一 | 多分支、0分支、与original durable state不相容 |
| authenticated_events | batch+0..256 ordered events+optional successor | gap/fork/cursor rollback/conflicting replay/257 events |

五组context/view必须concrete、sealed、Send、non-Clone/non-Debug/non-Serde；context按值进入validator且`validate(self,...)`一次消费；
view只校验同tx Store-built envelope，不生成ID/time/digest。每个`Verified...Exchange`私有持有HostReceipt+view并只提供一次同步
consume。通用`into_parts`、同一view实现全部五个`validate_*`、Store拿到raw bytes或operation substitution均拒绝。validator必须同时看到actual upstream response与child observation，并核exact length/SHA、exchange/session/command/outbox/
route/executor/fence/source、ordinal、time与对应durable envelope。

Prepare rejected当前必须拒绝或进入unknown→reconcile；在没有同事务rejected observation/ACK/no-start closure前，任何直接rejected
ACK、退款、Lease/activation或finish均失败。

size负例固定覆盖：ELTP request material>262144、upstream request>65536、response>262144、observation>262144；还要覆盖
unknown operation、wrong numeric code、ordinal 0/65、root/transcript/HMAC/nonce/digest mismatch与15秒deadline。Raw bytes只在
Zeroizing buffer。

## 6. External semantic wire payload 未选择门

[Semantic wire registry acceptance](external-pool-adapter-production-semantic-wire-profile-registry-abi-acceptance.md)独立验收selector、
evidence、current/historical与两道pre-send元ABI。本页只保持集成门：actual五operation keys/refs/modes/bounds/evidence仍未选择，
fixture不是production profile，五个validator impl、production child/session、positive ELTP与Runner caller必须为0。

未来实现必须消费该子权威形成的sealed current/historical operation profile；不得回退到response-only validation、latest profile、
同release语义轮换或raw/caller bytes。Durable send后任一道authorizer失败仍只能unknown→reconcile，不能误记local-never-sent。

## 7. Ownership、caller 与状态晋级

Source-contract必须锁唯一production path：worker selector→Tx-A `_on`→Tx-B `_on`→V278 source scan→owned task preflight→
outbound→broker concrete validator→同transaction V273 receipt/terminal ingress→Runner/recovery。必须同时验证：

- fence/Gateway/broker/worker owner-local symbols与module aggregators真实可达；
- generic owner facades、HTTP/env/caller JSON、fixture与第二Store sibling caller均无法进入；
- active-preparation、admission、Gateway或session错误不阻断historical cleanup；
- receipt与semantic closure原子commit，crash只能rollback或形成durable unknown→reconcile。

本页完成只可记各内部ABI `design_frozen/design_review_only`。只有initial market profile、external semantic wire profile、完整
Domain/DDL/Store/Gateway/fence/child/session/validator/worker/Runner/recovery与source-contract同批落盘后，才可记
`source_written/source_review_only`；再分别执行编译、next-free migration、SQLite、child/network与positive end-to-end验收。

## 8. 必须保持为零的副作用

本设计不得创建或激活Provider/Ready/Pool/Offer/Job/Reservation/Attempt/Lease/Runner/usage/settlement，不得预占物理V280编号，
不得写Secret/payload/session表，不得开放V254 fence、HTTP/MCP/WebSocket/callback。静态文档检查通过也不改变上述0/0边界。
