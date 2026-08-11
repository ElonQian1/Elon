---
title: 外部矿池 Adapter Release Admission 生命周期权威
status: current
reviewed_at: 2026-08-12
owners: backend, security, ai-economy
design_status: design_frozen
implementation_status: source_not_written
---

# 外部矿池 Adapter Release Admission 生命周期权威

## 1. 冻结结论

本文冻结 v229 的负向权威：为一份 immutable v222 `staged` admission 追加唯一终态，并从不可变根与终态 receipt 派生 currentness。当前状态严格为 `design_frozen/source_not_written`；没有 Rust、迁移、Store、Service、HTTP、测试或生产数据可以证明该合同已经生效。

v229 只有在同一实现批次同时完成 terminal producer、current view，以及 v227 artifact PUT 的 pre-CAS、Store transaction 和数据库 trigger 三层 currentness 门卫时，才构成真实纵切面。只增加表、视图或管理 API 而不让现有 consumer 拒绝终态 admission，属于 producer-less 状态 staging，必须判定为 NO-GO。

这是一项完整的负向终态，而不是新的正向准入：终态一旦写入就永久取消该 admission 的后续消费资格，但不会让任何 artifact、Adapter、verifier、credential 或 route 获得信任。

## 2. 状态机与不可变历史

v222 request 与 admission 的历史状态继续固定为 `staged`，不得为了 currentness 更新或删除既有行。v229 只允许从一份尚无 terminal 的 admission 追加一次：

```text
staged -> withdrawn | revoked | superseded
```

- `withdrawn`：平台主动撤回仍处于候选阶段的 admission；
- `revoked`：平台因安全、策略或治理原因撤销该 admission；
- `superseded`：平台明确指定另一份 exact、仍 current 的 staged admission 取代旧候选。

三种终态对 consumer 的效果完全相同：该 admission 永久不再 eligible。首版没有 `expired`、`restored`、`reactivated` 或自动 fallback。终态 admission 不能重新变回 staged；需要继续准备时必须形成另一份独立 admission，旧历史不复活。

`superseded` 必须绑定 successor `admission_id/admission_digest/release_version`，并满足：

- successor 与旧 admission 的 `adapter_id` 相同；
- admission ID 与 release version 不同；
- successor 本身仍为 immutable `staged`，且尚无 terminal；
- successor 的 `applied_at` 不早于旧 admission；
- future consumer 不得自动跟随 successor，必须从 successor 重新验证全部来源与 currentness。

successor 后续被撤回、撤销或替代时，旧 admission 仍保持 `superseded`，不会自动回退。release version 是 opaque 标识，v229 不解析或比较 SemVer。

## 3. 唯一账本与 current view

首版只新增：

- `compute_external_pool_adapter_release_admission_terminal_receipts`：每份 admission 最多一行 append-only terminal；
- `compute_external_pool_adapter_release_admission_current`：显式 LEFT JOIN immutable admission 与可选 terminal，派生 `current_status`。

terminal 表必须对 `terminal_receipt_id`、`terminal_receipt_digest`、`admission_id` 和 `(idempotency_scope,idempotency_key)` 保持唯一，并具有 exact JSON projection/source、no-replace、no-update 与 no-delete trigger。数据库不得以用户表外键证明管理员角色；`local-owner` 可能是认证层的 synthetic owner。DDL 只固定 `actor_kind=platform_admin` 与 actor ID 形状，真实 `admin|owner` 身份由 Service/API 的认证会话证明。

terminal canonical JSON 固定为：

```text
{
  schema, terminal_receipt_id, terminal_receipt_digest, request_digest,
  canonicalization, digest_algorithm,
  terminal: {
    admission: { admission_id, admission_digest, adapter_id, release_version },
    prior_status, terminal_status, successor_admission,
    actor_kind, actor_id, reason, confirmation,
    idempotency_scope, idempotency_key, occurred_at, recorded_at,
    currentness_effect, artifact_intake_effect, existing_artifact_source_effect,
    adapter_effect, route_effect
  }
}
```

固定值为 `prior_status=staged`、`actor_kind=platform_admin`、`currentness_effect=admission_terminal`、`artifact_intake_effect=blocked`、`existing_artifact_source_effect=historical_only`、`adapter_effect=none` 与 `route_effect=none`。`reason` 必须是 trim 后 8..2000 字符。`withdrawn/revoked` 的 successor 组全部为 NULL；`superseded` 的 successor 组全部非 NULL。`occurred_at=recorded_at`，均由服务端产生 canonical UTC nanoseconds。

current view 必须同时保留 immutable `admission_status=staged` 与派生 `current_status=COALESCE(terminal_status,'staged')`，并投影可选 terminal ID/digest/time 与 successor binding。`current_status=staged` 只表示“尚无负向终态”，不表示 trusted、verified、attested、可执行或可路由。

## 4. 管理 API 与 actor

首版只开放平台管理员 HTTP，不开放 MCP、PC、SDK 或 Provider owner 入口：

- `POST /api/admin/compute/external-pool-adapter-release-admissions/:admission_id/terminal`：追加唯一终态；
- `GET /api/admin/compute/external-pool-adapter-release-admissions/:admission_id/currentness`：读取 exact immutable root、可选 terminal 与派生 current status。

POST body 必须 `deny_unknown_fields`，且只接受：

- `idempotency_key`；
- `expected_admission_digest`；
- `terminal_status`；
- 仅 superseded 使用的 `successor_admission_id` 与 `expected_successor_admission_digest`；
- 非空有界 `reason`；
- `confirm_terminal=true`。

actor、actor kind、作用域、确认语和服务端时间不能由调用方提交。Service 按 action 派生固定确认语：

- `confirm_external_pool_adapter_release_admission_withdrawal`；
- `confirm_external_pool_adapter_release_admission_revocation`；
- `confirm_external_pool_adapter_release_admission_supersession`。

三种动作都只减少权威，因此任一当前 `admin|owner` 可执行，不要求新的四眼流程。认证必须先于 Service 写入，actor ID 从会话派生；HTTP 回执只返回 terminal/currentness 摘要，不返回 artifact bytes、绝对路径、verifier 细节或 bearer 信息。

## 5. 真实 producer 与现有 consumer

真实上游 producer 是已经存在并通过专项的 v222 stage：它产生 exact immutable admission。v229 的管理员 terminal API/Store 是不可逆负向终态 producer。现有具体 consumer 是 v227 artifact source PUT，虽然后者当前仍为未编译、未迁移、未运行源码，设计上已经具有明确的 admission authority、CAS custody 与 DB-second receipt 写入点。

v229 实现必须同时把 currentness 接入：

1. Service 在读取/流式消费 raw body 和进入 quarantine CAS 前，只能取得无 terminal 的 exact staged intake authority；
2. v227 Store 对 fresh write 和 exact replay 都必须在自己的事务线性化点重新要求 admission 仍 current；
3. v229 新增独立 BEFORE INSERT guard，使 artifact source receipt 在已有 terminal 时无法写入；既有 v227 exact-source trigger 继续重审 request/review/admission lineage；
4. artifact source GET 继续允许读取 immutable 历史 receipt，并重开复核 bytes，但必须与“当前仍可消费”分开表达。

竞争裁决固定为：

- terminal 先提交：随后 pre-CAS authority、PUT replay 或 DB receipt insert 失败关闭；
- CAS 已安装但 terminal 在 DB-second 前提交：receipt insert 失败，只留下不可采用的 unreferenced blob，沿 v227 既有 saga 边界等待未来 GC；
- artifact source receipt 先提交：terminal 仍可随后成功，历史 receipt/blob 保留，但 future consumer 必须拒绝该 admission；
- GET 历史读取不恢复 currentness，也不能被当作 registry authority。

因此 v229 的真实效果是终结现有 intake 资格，而不是继续生成候选材料。

## 6. 幂等、并发与 exact replay

terminal Store 写入必须使用 `BEGIN IMMEDIATE`。domain-separated request digest 必须覆盖 base admission、可选 successor、terminal status、reason、confirmation、actor、idempotency scope/key。规则固定为：

- 同 scope/key、同 exact request 返回原 terminal receipt，并标记 replay；
- 同 scope/key 任一材料变化都冲突；
- 不同 key 竞争同一 admission 时只允许一份 terminal 成功；
- terminal exact replay 读取既有 receipt 与 immutable source，不因 base 已终态而自我拒绝；
- supersede 与 successor terminal 竞争时，在同一写锁内重审 successor currentness，只有一种顺序成功；
- 一份 successor 可以被多份旧 admission 显式引用，但不能由 consumer 自动选择或提升。

v227 PUT 的幂等语义与 terminal receipt 不同：终态之后，即使存在旧 artifact source receipt，同 PUT 也必须因 currentness 失败；历史读取只走 GET。这避免成功 replay 被误读为 admission 仍可继续消费。

## 7. Future registry consumer 门卫

未来 Adapter registry producer 必须保存 exact admission ID/digest 与当时 terminal/currentness binding，并在创建、采用、重放和 route currentness 检查时重新读取 current view。任何非 `staged` 状态都必须失败关闭；不得按 adapter ID、release version 或摘要相似性推断来源，也不得自动跟随 superseded successor。

该要求只是后续合同。v229 不创建 Adapter registry、verified artifact、credential verifier 或 v213 source companion。

## 8. v213 禁线与非目标

v229 不得创建、修改或删除 v213 Adapter/version、credential、route authorization、capability、seal 或 source trigger，不得按 adapter ID/release version 猜测既有 v213 行与 admission 有 lineage，也不撤销 route、outbox 或 Lease。当前 v213 没有 admission companion，不能用字段相似性补造关系。

本批还明确不做：

- 删除、覆盖或移动 v227 receipt/blob；
- candidate artifact ref resolution、provenance、signature、SBOM、供应链或 sandbox conformance；
- verifier registry/currentness/revocation、service actor 或 Provider-specific credential proof；
- Adapter registry/version、自动 promotion、rollback 或 successor activation；
- 外部网络、worker、prepare/commit/reconcile/cancel、authenticated ACK/event；
- Provider/Pool/Supply/Offer/Job/Reservation/Plan/usage/资金/结算效果；
- MCP、PC、生产部署或生产迁移。

## 9. 实现与验收门槛

当前状态只允许描述为 `design_frozen/source_not_written`。未来实现完成后仍须分别记录编译、fresh/upgrade/repeat migration、Store 重开、HTTP 鉴权、三终态、successor 失败路径、exact replay、双 terminal 竞争、terminal↔v227 CAS/DB 两种顺序，以及终态后的 PUT/GET 差异。没有这些证据时不得升级 implementation 或 verification 状态。
