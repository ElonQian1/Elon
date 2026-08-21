---
title: 联邦执行验证因果引用 Carrier ABI 权威
reviewed_at: 2026-08-22
status: current
owners: backend, node, ai-economy, pc
design_status: design_frozen
design_scope: federation_execution_verification_causal_reference_carrier_abi_v1
implementation_status: implementation_uncompiled
verification_status: source_review_only
---

# 联邦执行验证因果引用 Carrier ABI 权威

## 1. 唯一结论与状态边界

本页冻结 Provider-neutral、endpoint-only additive 的 `execution_verification_source_v1`。它把 exact v188 最终
Provider 声明用量、v189 终态候选、v190 消费者审核、v191 平台观测、v192 accepted Verification 与 v193
Execution Receipt 组成一条只读历史因果链，并通过重新生成的 `execution_source_v1` digest 与既有核心 Carrier
闭合。

它复用 [core carrier authority](federation-historical-causal-reference-abi-authority.md) 的 schema、domain、JCS、
SHA-256、262144-byte 上限与 exact-5 read response，不修改旧三种 profile 的 JSON、digest、endpoint 或客户端合同。
本页不新增 migration、table、writer、缓存、current authority、状态或经济效果。

当前状态逐字为：

```text
federation_execution_verification_causal_reference_carrier_abi=design_frozen
profile=execution_verification_source_v1
domain_implementation=source_written
store_resolver=source_written
service=source_written
http_mcp=source_written
pc_client=source_written
migration/table=none
verification=source_review_only
compiled=0 run=0
passed=0 failed=0
native_digest_rewrites=0
state_or_money_effects=0
f0_exit_gate=not_met
```

这些状态只描述本 profile。v188-v193 原 owner 的既有成熟度仍以各自专题为准；本批没有复用旧编译或测试结果来
证明新 Carrier。验收矩阵见 [对应 acceptance](federation-execution-verification-causal-reference-abi-acceptance.md)。

## 2. 为什么这是引用层而不是第二份 Verification

v193 `ComputeExecutionReceipt` 继续是执行事实唯一 owner。它已经保存 declared、observed、verified、compensable
四类 meter、三项 attestation、Verification policy 与 receipt self digest。本页不复制这些数组、outcome、policy、
reason code、actor 或时间，也不重新计算任何 v188-v193 native digest。

新 profile 只解决两个 F0 问题：

1. 用稳定、角色明确的最小 ref 表达 v193 背后的六个 retained owner；
2. 让 HTTP/MCP/PC 可以证明所读验证链与同一 `execution_source_v1` 属于同一 Execution Receipt。

因此本 Carrier 证明的是“服务端在一个历史 read snapshot 内重新审计并引用了哪些事实”，不是 Provider 自报真实性、
平台计量签名、可信时钟、独立复算、付款授权或争议终局。只有 accepted v192 才能生成 v193，所以本 profile 有意不
表示 rejected/disputed 决定；不得以 null Execution Receipt 扩大 V1。

v200/v201 提款从 Provider account 的聚合 available 余额冻结任意金额，没有 source release/Lease 分配关系，不能接到
本链或 `settlement_release_source_v1` 后伪造单笔来源。

## 3. Canonical envelope 与 exact lineage

常量逐字复用核心 ABI：

```text
SCHEMA=compute_federation.core_historical_causal_reference.v1
DIGEST_DOMAIN=ELON-COMPUTE-CORE-HISTORICAL-LINEAGE-V1
CANONICALIZATION=rfc8785_jcs
DIGEST_ALGORITHM=sha256
MAX_JSON_BYTES=262144
```

顶层仍为 exact 6 keys：

```text
schema
lineage_kind
lineage_digest
canonicalization
digest_algorithm
lineage
```

`lineage_kind` 逐字为 `execution_verification_source_v1`；`lineage` exact 7 keys：

```text
execution_receipt
execution_lineage_digest
provider_declared_usage
terminal_candidate
consumer_review
platform_observation
verification_decision
```

`execution_lineage_digest` 必须是同一 read transaction 内重新生成 exact `execution_source_v1` 得到的 64 位小写
SHA-256；不得从数据库、caller、缓存或旧响应照抄。其余字段全部为 §4 exact object。禁止 null、extra/missing key、
generic `causes[]`、status、actor、time、current、replay、effect 或完整 meter 数组。

Carrier self digest 与 canonical byte rules 完全沿用核心 ABI：把 `lineage_digest` 设为空字符串后做 RFC 8785 JCS，
计算 `SHA256(DIGEST_DOMAIN UTF-8 || 0x00 || canonical projection UTF-8)`，再把 64 位小写十六进制结果写回。
最终 JSON 必须逐字 canonical；duplicate、unknown、float、非 I-JSON、trailing byte 或超过上限全部拒绝。

## 4. 六个 exact historical refs

| ref | exact keys | native owner |
|---|---|---|
| `ExecutionReceiptRef` | `execution_receipt_id,execution_receipt_digest` | v193 inner `ComputeExecutionReceipt` |
| `ProviderDeclaredUsageRef` | `usage_snapshot_id,usage_sequence_no,cumulative_usage_digest,usage_event_digest` | v188 exact final sequence |
| `TerminalCandidateRef` | `terminal_candidate_id,terminal_candidate_event_digest` | v189 immutable candidate |
| `ConsumerReviewRef` | `consumer_review_id,consumer_review_event_digest` | v190 immutable review |
| `PlatformObservationRef` | `platform_observation_id,platform_observation_event_digest,cumulative_observed_usage_digest` | v191 immutable observation |
| `VerificationDecisionRef` | `verification_decision_id,verification_event_digest,verified_usage_digest,compensable_usage_digest` | v192 immutable decision |

唯一字段映射逐字为：

```text
ExecutionReceiptRef.execution_receipt_id = v193 receipt.receipt_id
ExecutionReceiptRef.execution_receipt_digest = v193 receipt.receipt_digest

ProviderDeclaredUsageRef.usage_snapshot_id = v188 snapshot_id
ProviderDeclaredUsageRef.usage_sequence_no = v188 sequence_no
ProviderDeclaredUsageRef.cumulative_usage_digest = v188 cumulative_usage_digest
ProviderDeclaredUsageRef.usage_event_digest = v188 event_digest

TerminalCandidateRef.terminal_candidate_id = v189 terminal_candidate_id
TerminalCandidateRef.terminal_candidate_event_digest = v189 event_digest

ConsumerReviewRef.consumer_review_id = v190 consumer_review_id
ConsumerReviewRef.consumer_review_event_digest = v190 event_digest

PlatformObservationRef.platform_observation_id = v191 platform_observation_id
PlatformObservationRef.platform_observation_event_digest = v191 event_digest
PlatformObservationRef.cumulative_observed_usage_digest = v191 cumulative_observed_usage_digest

VerificationDecisionRef.verification_decision_id = v192 verification_decision_id
VerificationDecisionRef.verification_event_digest = v192 event_digest
VerificationDecisionRef.verified_usage_digest = v192 verified_usage_digest
VerificationDecisionRef.compensable_usage_digest = v192 compensable_usage_digest
```

`usage_sequence_no` 必须为 `1..=2^53-1` JSON integer。全部 event/usage/receipt digests 在本 profile 中必须是 64 位
小写 SHA-256；ID 必须 nonempty、trim-stable、无 control character。字段名不同只表示 role-specific projection，不能
重命名 native owner 字段或另造摘要公式。

## 5. Retained-only Store reconstruction

唯一公开 root 是 `lease_id`。Store 在一个 `BEGIN DEFERRED` snapshot 内：

1. 按 Lease 读取 exact-one v193 historical envelope；缺失返回 not-found，row/JSON/digest drift 返回 integrity failure；
2. 用 v193 self pair 重新生成 `execution_source_v1`，线性取得其 private access scope 与 digest；
3. 从 v189 保存的 `final_usage_sequence_no` 读取 exact v188，不读取 latest；
4. 分别调用 v189-v192 retained historical owner audit，并逐字重证同一 Lease、Provider、consumer、Job、Reservation、
   Claim、source Lease revision/digest 与 fencing；
5. 证明 `v189.final_usage_snapshot_id/sequence/digest == v188`，v190/v191 candidate pair等于 v189，
   `v191.final_provider_usage_digest == v188.cumulative_usage_digest`；
6. 证明 v192 的 candidate/review/observation pair 等于 v189-v191，final provider/observed usage digest 等于 v188/v191；
7. 证明 v193 outer verification ID/event 等于 v192；inner receipt `verification.decision_digest == v192.event_digest`，
   declared/observed/verified/compensable arrays 逐字等于 v188/v191/v192，三项 attestation digest逐字等于 v189-v191；
8. 证明新 lineage 的 Execution Receipt ref 与 rebuilt execution subject逐字相等，然后构造 Carrier；
9. 返回 private-field、non-Clone、non-Serde validated view，raw DTO/parser/caller不能进入 writer。

现有 retained seam 的历史边界固定为：

- v188 `compute_attempt_usage_declaration_on(lease_id, exact_sequence)` 只审计存储事实；
- v189 historical candidate 只读取其保存的 exact v188 sequence，不调用 latest；
- v190/v191 historical read 使用 historical v189，v191 另读取 exact v188；
- v192 historical read 使用 historical v189-v191、exact v188 与 historical Reservation；
- v193 historical read 使用上述 owner、historical activation、Job 与 Reservation。

新 resolver 不降低这些 seam，也不增加 current/latest fallback。v188-v193 原生 raw-serde digest 继续是 opaque exact value；
Carrier JCS digest 不替代它们。

## 6. Access scope、Service、HTTP 与 MCP

resolver 唯一 facade：

```text
resolve_compute_execution_verification_source_lineage_for_lease(lease_id)
```

它线性复用 rebuilt execution 的 private access scope：historical consumer account、optional project 与 historical
Provider owner。Scope 不进入 Carrier、response、serde、Debug 或 client，也不跨 transaction/await 保留权限。

Service 只新增 participant/admin 两个 read facade。HTTP 只新增：

```text
GET /api/me/compute/attempt-leases/:lease_id/execution-verification-source-lineage
GET /api/admin/compute/attempt-leases/:lease_id/execution-verification-source-lineage
```

两条路由只接受 path `lease_id`，没有 query/body。普通 caller 必须是 historical consumer 或 Provider owner；admin
复用 platform admin/owner gate。MCP 只新增：

```text
compute_get_my_execution_verification_source_lineage
compute_admin_get_execution_verification_source_lineage
```

input exact `{lease_id}`、`additionalProperties=false`。普通 MCP 还要求 historical project 等于当前 MCP project；admin
MCP 同时要求 transport project membership 与 platform admin role。成功只返回核心 exact-5 response，kind 固定本页值。

失败分类沿核心 ABI：malformed=400；unauthenticated=401；project/admin forbidden=403；participant 不可见、合法但无
v193 或 integrity drift 均脱敏 404；admin 无 v193=404、integrity drift=409。不得按 `anyhow` 文本分类。

## 7. PC 交叉响应闭合

现有结算生命周期因果卡对所有可读 Execution Receipt 同时请求 execution 与 verification；settlement/release 继续按原
生命周期条件请求。PC 必须：

1. 独立对两份 exact-5 response 做 exact-key、UTF-8/JCS byte-equal、domain digest 与内外 kind/digest检查；
2. 证明 `verification.execution_lineage_digest == execution.lineage_digest`；
3. 证明 verification 与 execution 的 `execution_receipt_id/digest` 两字段逐字相等；
4. 再执行既有 execution→settlement→release 等式；
5. 任一 network、permission、shape、digest 或跨响应等式失败时整张卡失败关闭，保留有界重试与 stale-response guard。

PC 不持久、导出或记录 canonical Carrier，不新增 writer/navigation，也不把读取成功显示为可信计量或真实付款。

## 8. Source-written 门与明确禁线

本 profile 保持 `source_written` 必须同时具备 Domain exact DTO/canonical/literal golden/negative、Store retained resolver
与 splice negatives、Service/2 HTTP/2 MCP、PC 双响应校验及 zero-writer/migration/effect source contract。任一层缺失
必须降为 `partial|absent`。

明确禁止：

- 修改旧三种 profile 的 bytes/digest、endpoint 或 parser语义；
- 新 migration/table/cache/backfill，或向 v188-v193 写 Carrier digest；
- current/latest fallback、caller-supplied ref/digest/kind 或复制 owner 摘要公式；
- 复制 meter vectors、outcome、policy、actor/time/status，或把本 profile 变成第二份 Verification；
- 把 rejected/disputed 塞入 nullable v193，或把 provider-declared/platform-observed 称为真实可信计量；
- 把 v200/v201 聚合提款金额虚构为某个 release/Lease 的下游；
- 因源码落盘而提高 compiled/run/passed/failed、F0 gate 或生产成熟度。
