---
title: UserNode Provider Binding Root 验收
status: current
reviewed_at: 2026-08-21
owners: backend, security, node-agent, ai-economy
design_status: design_frozen
implementation_status: implementation_uncompiled
verification_status: source_review_only
---

# UserNode Provider Binding Root 验收

## 1. 当前证据强度

V279 当前只能记为 `design_frozen / source_written / source_review_only /
implementation_uncompiled / implementation_unrun`、`passed=0 / failed=0`。本批没有编译、测试、执行 migration、
SQLite、runtime 或 network；Domain、migration、Store/API 与 activation gate 的 `source_written` 不等于它们已经
编译或运行接通，也不代表 Android、Ready 或 task execution 已接通。

Fixture、seed、直接 SQL、任意 `node-binding://...` 字符串、endpoint session、sharing ACK、Provider self-declaration、
V277/V278 receipt 或历史测试计数都不能充当 V279 positive evidence。

## 2. Domain 与 canonical matrix

| Case | 必须结果 |
|---|---|
| metadata | schema、JCS、SHA-256、confirmation 与四个 domain 逐字匹配。 |
| identity | binding ID 只由五字段 identity material 派生；credential/consent rotation 不改变 ID。 |
| request | request digest 只覆盖本人请求六字段，允许 source 漂移后 0-write replay。 |
| material | genesis revision固定1；三个 source revision为I-JSON safe正整数；authorization revision等于consent revision。 |
| time | `bound_at=recorded_at` 且是 canonical UTC nanoseconds。 |
| effects | binding=`identity_binding_recorded`；其余七项逐字`none`。 |
| receipt | material digest和blank-digest receipt digest分别复算；unknown/missing/extra字段拒绝。 |
| exact JSON | `user_node_provider_binding_json_is_canonical` 只有parse、full validate与byte-exact JCS都成功时返回true。 |

Material/receipt 所有字段保持私有；公开 getter 不返回可变引用。Domain 不得新增 current/authority/permit/bool-ready
类型，也不得从 raw JSON 验证结果直接授权 Store 写入或下游执行。

## 3. Durable shape 与 migration matrix

| Case | 必须结果 |
|---|---|
| shape | exact 1表、37列、0 view、0 revocation、0 mutable head。 |
| one-to-one | binding ID、Provider ID、node ID各唯一；相同node/provider不得出现第二条lineage。 |
| idempotency | scope/key唯一；exact replay零写，facts冲突拒绝。 |
| canonical UDF | exact name `elon_v279_user_node_provider_binding_is_exact`、arity 1、deterministic；NULL/0/error fail closed。 |
| source lineage | INSERT逐字段绑定Provider genesis、current endpoint credential、current consent/authorization。 |
| immutability | UPDATE/DELETE/REPLACE全部拒绝；trigger drift由drop/create修复。 |
| reopen | fresh/repeat/reopen获得同一table/index/trigger/UDF inventory，零row migration不伪造binding。 |

Migration 不得改变 node credential、sharing、Provider、activation、V254 或 V273-V278 既有表与 trigger 语义。若需对
activation request 加 guard，只能对 `user_node` fresh INSERT 追加 exact binding 条件，并保留既有状态、幂等及不可变条件。

## 4. Writer、replay 与 failure matrix

Fresh writer 必须在一个 `BEGIN IMMEDIATE` 内依次完成 idempotency absence、Provider current/genesis、endpoint root、
current consent、one-to-one absence、receipt build、单 INSERT、exact readback 与 current reproof。任何一步失败为0 row。

| Case | 必须结果 |
|---|---|
| exact replay | 先完整读回历史request/row/JSON/digests，0 mutation；不要求source仍current。 |
| same key drift | provider/node/owner/confirmation变化拒绝。 |
| another key | 已绑定node或Provider拒绝，不覆盖或“更新”历史root。 |
| source race | Provider、credential、consent在写前漂移时INSERT或final reproof失败并整体rollback。 |
| commit uncertain | 只能按scope/key读取并exact audit；不得再生成新ID或盲重写。 |
| corruption | malformed JSON、digest drift、duplicate/fork、source projection漂移返回错误，不降级为not-current。 |

## 5. Current reproof matrix

| Case | 结果 |
|---|---|
| exact current | 同事务返回不可逃逸、不可Clone/Serde的Store-private authority。 |
| consent revision advances | 当前authorization/install仍精确时可重证；历史source字段不更新。 |
| sharing disabled | `None`；receipt保留，不生成撤销row。 |
| credential rotates | current active credential保持同installation binding时可重证。 |
| credential revoked | `None`。 |
| reinstall | installation identity或endpoint installation binding改变时`None`。 |
| Provider revision advances | 必须从exact genesis连续演进且stable identity不变；consumer另行限制允许状态。 |
| fork/corruption | `Err`，不得伪装成`None`或重绑。 |

Session online、ACK observed、普通 node registry online 或调用方 boolean 不参与 current binding reproof；它们也不能让
失效 binding 重新current。

## 6. Activation integration matrix

Fresh user-node activation request 的 Store 事务必须在 INSERT 前重证 exact binding，并核对申请中的 Provider
revision/digest；任意字符串、他人node/provider、disabled consent、revoked credential、reinstall或binding digest漂移均
拒绝。Existing exact activation-request replay仍保持0-write并先于current source reads。

正确 binding 只关闭 `node_binding_ref` 一项：ReadyCapability digest、route proof、hardware observation仍是待审核材料，
既有 `provider_routing_missing`、`verified_hardware_missing`、`provider_trust_tier_self_declared` 等 blocker 不得因V279消失。
V279 不自动准备、复核或应用激活计划。

## 7. Zero-effect 与排重 matrix

V279 transaction 的生产写集必须只有 `compute_user_node_provider_bindings` 单 INSERT；Provider、Pool、Supply、Offer、
Job、Reservation、Claim、Attempt、Lease、v212/v213、usage、receipt、settlement、balance写入全部为0。

不得读取或写入 external-pool Adapter、V273 exchange、V274 successor、V277 activation或V278 renewal作为binding source。
不得把历史 legacy LLM node run、普通模型列表、capacity label、fixture或Android文案当作统一task-level Provider binding。

## 8. 后移验收

以下全部不计入V279 passed：

- production VFS/A1/A2、v15 endpoint profile、signed work-admission；
- downloader、Sidecar/IPC、Host enforcement、Runtime、动态健康与Ready V2；
- CPU-only resource-ceiling修正、Provider endpoint route、v212 sealed constructor；
- `ValidatedComputeAttemptStartDispatch`/activation-plan producer；
- v213节点wire、ACK/Lease、Runner与Execution/Settlement Receipt；
- Android真实HTTP、浏览器/设备、生产数据库升级、部署或任何经济效果。

只有后续独立阶段完成上述链路，才能声明节点 Ready、Provider 可调度、Offer 可交易、attempt eligible 或任务已执行。
