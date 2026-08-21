---
title: Attempt Verification retained read 权威
reviewed_at: 2026-08-22
status: current
owners: backend, ai-economy, pc
design_status: design_frozen
design_scope: attempt_verification_retained_read_v1
implementation_status: implementation_uncompiled
verification_status: source_review_only
---

# Attempt Verification retained read 权威

## 1. 唯一结论与状态

本页冻结 v192 `ComputeAttemptVerificationDecisionReceipt` 的 lease-only retained read。它保留原生 exact-52
response，不新增 envelope 或第二份 Verification；Store 必须在一个历史 read snapshot 内从 exact Lease root 重新审计
v188-v192，并以 private scope 完成 participant、admin 与 MCP project 隔离。

当前状态逐字为：

```text
attempt_verification_retained_read_v1=design_frozen
store/service/http/mcp/pc=source_written
verification=source_review_only
implementation=implementation_uncompiled/implementation_unrun
compiled=0 run=0 passed=0 failed=0
migration/table/writer/cache/backfill=0
state/scheduling/capacity/money_effects=0
f0_exit_gate=not_met
```

以上状态只属于 retained read 增量。旧 v192 writer、旧 GET 或 PC 历史编译/测试证据不能外推为本页已编译、已运行或已
验收；验收矩阵见 [对应 acceptance](attempt-verification-retained-read-acceptance.md)。

## 2. 原生 exact-52 response

成功响应继续使用 schema `compute_federation.attempt_verification_decision.v1`，且 exact key set 固定为：

```text
schema
verification_decision_id
terminal_candidate_id
terminal_candidate_event_digest
consumer_review_id
consumer_review_event_digest
platform_observation_id
platform_observation_event_digest
lease_id
provider_id
consumer_account_id
source_lease_revision
source_lease_digest
fencing_generation
job_id
job_revision
job_digest
reservation_id
reservation_revision
reservation_digest
capacity_claim_id
capacity_claim_revision
capacity_claim_digest
final_usage_snapshot_id
final_usage_sequence_no
final_provider_usage_digest
platform_observed_usage_digest
candidate_outcome
consumer_decision
observed_outcome
policy_id
policy_version
decision
reason_codes
reason_codes_digest
decision_ref
verified_usage
verified_usage_digest
compensable_usage
compensable_usage_digest
request_digest
event_digest
decided_by_user_id
decided_at
verification_effect
execution_receipt_effect
lease_effect
job_effect
capacity_effect
reservation_effect
money_effect
replayed
```

不得添加 `read_effect`、Carrier、access scope、current head 或 v193 字段。read 返回的 `replayed=false`；
`verification_effect` 等 effect 字段是原 v192 历史回执内容，不表示本次 GET/MCP 产生效果。

读取必须复用 v192 原生规范化、policy、reason-code、meter、request/event digest 与 receipt audit，不重命名字段、不另造
摘要公式。`accepted`、`rejected`、`disputed` 三种决定都可读；rejected/disputed 的 verified/compensable readings 按原
v192 policy 为零，但绝不能因为不存在 v193 而被隐藏。

## 3. Lease-only retained reconstruction

唯一 Store facade 只接收 `lease_id`。在一个 `BEGIN DEFERRED` snapshot 内必须：

1. 按 Lease 选择 exact-zero-or-one v192 immutable decision；缺失与 row/JSON/digest drift 分型，禁止多值；
2. 从 v192 保存的 terminal candidate、consumer review、platform observation pair 读取 v189、v190、v191 historical owner；
3. 从 historical v189 保存的 `final_usage_sequence_no` 读取 exact v188 snapshot，不调用 latest；
4. 读取 exact historical Reservation、Capacity Claim 与 source Lease，逐字证明 Reservation 的 Job/Offer/Provider/Claim
   绑定和 Lease 的 revision/digest/fencing/Job/Reservation/Provider 都属于同一条 v192 历史链；
5. 仅从该链的 historical Job consumer/project 与 historical Provider owner 构造 access scope；
6. 复用 v192 audit 重算 policy inputs、reason codes、verified/compensable readings 及全部 native digest；
7. 逐字证明 Lease、Provider、consumer、Job、Reservation、Claim、source Lease revision/digest、fencing 与 v188-v192
   evidence pair 全部一致；
8. 构造 private-field、non-Clone、non-Serde validated read view，再线性交给 Service 做授权与 exact-52 transport。

禁止 current/latest fallback、receipt-ID root、caller-supplied evidence/digest、缓存、materialized projection 或 repair-on-read。
v193、Execution Receipt、finalization、settlement 与 Carrier 均不是 resolver 依赖；accepted v192 即使尚无 v193 仍可读。

## 4. Private access scope

validated view 内部只携带授权所需的 historical scope：

- v189/v192 绑定的 consumer account；
- historical Job 的 optional project；
- historical Provider 的 owner user。

participant 仅当当前 user 是 historical consumer 或 Provider owner 时可见。Scope 不进入 exact-52 response、serde、Debug、
日志或客户端；Store commit 后只允许随 non-Clone validated view 线性交给 Service 消费，不得跨 await、cache 或
persistence 保留。raw DTO、parser 或 caller 不能自行构造 validated view。

普通 MCP 还必须证明 historical project 等于 transport 当前 project。平台 admin read 复用 `admin/owner` 角色门；admin MCP
不能绕过 transport project membership。授权使用 historical scope，不以 Provider、Job 或 project 当前 head 代替。

## 5. HTTP、MCP 与错误 ABI

HTTP exact 2 GET：

```text
GET /api/me/compute/attempt-leases/:lease_id/verification-decision
GET /api/admin/compute/attempt-leases/:lease_id/verification-decision
```

管理员 GET 与既有 POST 共用 path，但方法语义完全分离。两条 GET 只有 path `lease_id`，无 query/body。MCP exact 2：

```text
compute_get_my_attempt_verification_decision
compute_admin_get_attempt_verification_decision
```

MCP input exact `{lease_id}`，`required=[lease_id]`、`additionalProperties=false`。成功均返回 §2 native exact-52，MCP 不另
包裹 Carrier、project 或 effect。

错误码与映射固定为：

| 场景 | HTTP | code |
|---|---:|---|
| 非法 path Lease | 400 | `ATTEMPT_VERIFICATION_RETAINED_INVALID_LEASE_ID` |
| query/body/MCP shape 非法 | 400 | `ATTEMPT_VERIFICATION_RETAINED_INVALID_REQUEST_INPUT` |
| 未认证 | 401 | `ATTEMPT_VERIFICATION_RETAINED_UNAUTHENTICATED` |
| participant 非本人、缺失或 integrity drift | 404 | `ATTEMPT_VERIFICATION_RETAINED_NOT_VISIBLE` |
| 普通 MCP historical project 不匹配 | 403 | `ATTEMPT_VERIFICATION_RETAINED_PROJECT_FORBIDDEN` |
| admin 无 v192 | 404 | `ATTEMPT_VERIFICATION_RETAINED_NOT_FOUND` |
| admin 发现 integrity drift | 409 | `ATTEMPT_VERIFICATION_RETAINED_INTEGRITY_CONFLICT` |
| 非 `admin/owner` 调用 admin surface | 403 | `ATTEMPT_VERIFICATION_RETAINED_ADMIN_FORBIDDEN` |
| connection/transaction/SQLite operational failure | 500 | `ATTEMPT_VERIFICATION_RETAINED_INTERNAL_ERROR` |

分类必须来自 typed Store/Service error，不按 `anyhow` 文本猜测。participant 有意把 missing、integrity 和
nonparticipant 合并为脱敏 404；connection、begin/commit、busy/I/O 与 SQL/schema operational failure 不得伪装成 404/409，
HTTP 返回稳定 500 code，MCP 只返回同一脱敏 code。row/JSON/column/native digest 与 owner-chain drift 仍属于 integrity。

## 6. PC exact parsing 与条件 14 等式

PC `/compute-verification` 必须以 exact-key parser 读取 native exact-52，继续展示 accepted/rejected/disputed；没有 v193 或
`execution_verification_source_v1` 不得使这项读取失败。

仅当某张因果卡已取得 v193 对应的 validated `execution_verification_source_v1` 时，PC 还必须在展示前证明以下 14 条
逐字等式：

```text
native.final_usage_snapshot_id = carrier.provider_declared_usage.usage_snapshot_id
native.final_usage_sequence_no = carrier.provider_declared_usage.usage_sequence_no
native.final_provider_usage_digest = carrier.provider_declared_usage.cumulative_usage_digest

native.terminal_candidate_id = carrier.terminal_candidate.terminal_candidate_id
native.terminal_candidate_event_digest = carrier.terminal_candidate.terminal_candidate_event_digest

native.consumer_review_id = carrier.consumer_review.consumer_review_id
native.consumer_review_event_digest = carrier.consumer_review.consumer_review_event_digest

native.platform_observation_id = carrier.platform_observation.platform_observation_id
native.platform_observation_event_digest = carrier.platform_observation.platform_observation_event_digest
native.platform_observed_usage_digest = carrier.platform_observation.cumulative_observed_usage_digest

native.verification_decision_id = carrier.verification_decision.verification_decision_id
native.event_digest = carrier.verification_decision.verification_event_digest
native.verified_usage_digest = carrier.verification_decision.verified_usage_digest
native.compensable_usage_digest = carrier.verification_decision.compensable_usage_digest
```

任一 shape、digest 或等式失败时该因果卡失败关闭，并保留有界重试和 stale-response guard。Carrier absent 只表示没有
v193 因果卡，不能把独立 native v192 的成功读取伪装成 integrity failure。PC 不新增 writer、导航、持久化、日志或导出。

## 7. 信任与零副作用边界

retained read 只证明“原 v192 决定可从 exact historical owners 重建且调用者有权读取”。它不使人工
`conservative_min_v1` 输入可信，不证明 Provider 声明或平台观测签名、可信时钟、自动采集、独立复算、挑战、争议裁决
或真实执行。

本批 migration/table/writer/cache/backfill 均为 0；不创建 v193，不推进 Lease/Job/Reservation/Claim，不消费或释放
容量，不扣款、退款或确认 Provider 收益，也不改变任何 native digest。源码落盘不能提高 F0 gate 或生产成熟度。
