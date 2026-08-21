---
title: 联邦执行验证因果引用 Carrier ABI 验收
reviewed_at: 2026-08-22
status: current
owners: backend, node, ai-economy, pc
design_status: design_frozen
implementation_status: implementation_uncompiled
verification_status: source_review_only
---

# 联邦执行验证因果引用 Carrier ABI 验收

## 1. 验收对象与真实状态

本页只验收 [authority](federation-execution-verification-causal-reference-abi-authority.md) 冻结的
`execution_verification_source_v1` Domain、retained Store read、Service、HTTP/MCP 与 PC source adoption。

当前证据矩阵：

| 层 | implementation | verification | delivery/acceptance |
|---|---|---|---|
| Authority/ABI | `design_frozen` | `source_review_only` | current docs |
| Domain DTO/canonical/parser | `source_written` | `uncompiled/unrun` | `passed=0 failed=0` |
| Retained Store resolver | `source_written` | `uncompiled/unrun` | `passed=0 failed=0` |
| Service/HTTP/MCP | `source_written` | `uncompiled/unrun` | `passed=0 failed=0` |
| PC parser/cross-response | `source_written` | `uncompiled/unrun` | `passed=0 failed=0` |
| Migration/table/writer | `none` | `not_applicable` | zero effect |

旧 owner、旧三 profile 或 PC 既有构建的结果不能计入本 profile。按用户要求，本批不编译、不运行测试、服务、浏览器、
迁移或真实网络，因此 F0 gate 仍为 `not_met`。

## 2. A — Domain ABI

### A1. Exact envelope/profile

静态源码必须同时具备：

- kind `ExecutionVerificationSourceV1` ↔ JSON `execution_verification_source_v1`；
- variant `ExecutionVerificationSource(ExecutionVerificationSourceLineageV1)`；
- builder `build_execution_verification_source_carrier`；
- 与核心 ABI 相同的 schema/domain/JCS/SHA-256/max bytes；
- exact 7-key lineage 与 exact 6 refs，全部 `deny_unknown_fields`；
- `usage_sequence_no` 为 I-JSON safe positive integer；
- profile 内全部 receipt/event/usage digest 为 64 位小写 SHA-256。

kind/variant 不匹配、missing/unknown key、null、float、数字字符串、非 canonical bytes、duplicate key、错误 digest 或
超限输入必须失败关闭。旧 `execution_source_v1`、`settlement_source_v1`、`settlement_release_source_v1` DTO、builder、
literal golden 与 digest 不得改写。

### A2. Literal golden

源码必须保存一份手工固化的完整 canonical JSON 与 literal digest：

```text
lineage_kind=execution_verification_source_v1
lineage_digest=5a9f2d79b7bbe2e503ee636d19edeb2e019269cfa65ad50e77a93753a23780c7
```

expected 不能在测试内调用 production canonical/digest helper生成。golden fixture 的 ID/digest/sequence、key order 与
完整 envelope 都是 reviewable literal；builder、parser、canonical output 与 digest未来运行时必须逐字命中。

### A3. Negative matrix

源码负例至少覆盖：

- wrong kind、missing lineage digest、missing/extra ref key；
- extra `decision/provider` 等非引用字段；
- `usage_sequence_no=0`、超 `2^53-1`、float；
- trim-unstable ID、uppercase/short digest；
- leading whitespace、reordered equivalent JSON；
- ref role值变化导致 Carrier digest变化。

## 3. B — Retained Store reconstruction

### B1. Unique root 与 historical seam

公开 resolver 只收 `lease_id`，在一个 deferred read snapshot 内从 exact-one v193 root重建。源码合同必须证明：

- v188 使用 `compute_attempt_usage_declaration_on(lease, exact_sequence)`；
- v189 historical path使用保存的 final sequence，不调用 `latest_compute_attempt_usage_declaration_on`；
- v190/v191/v192/v193 使用各自 historical source policy；
- v192 使用 historical Reservation，v193 使用 historical activation/Job/Reservation；
- 没有 current/latest/receipt-ID fallback、caller refs或缓存。

### B2. Exact causal equalities

Store 验收源码必须逐项检查：

```text
v189.final_usage_snapshot_id = v188.snapshot_id
v189.final_usage_sequence_no = v188.sequence_no
v189.final_cumulative_usage_digest = v188.cumulative_usage_digest

v190.candidate_pair = v189
v191.candidate_pair = v189
v191.final_provider_usage_digest = v188.cumulative_usage_digest

v192.candidate_pair = v189
v192.consumer_review_pair = v190
v192.platform_observation_pair = v191
v192.final_provider_usage_digest = v188.cumulative_usage_digest
v192.platform_observed_usage_digest = v191.cumulative_observed_usage_digest

v193.outer.verification_pair = v192
v193.inner.verification.decision_digest = v192.event_digest
v193.inner.declared/observed/verified/compensable = v188/v191/v192 arrays
v193.inner.attestation digests = v189/v190/v191 event digests
carrier.execution_receipt = rebuilt_execution.execution_receipt
carrier.execution_lineage_digest = rebuilt_execution.lineage_digest
```

任何 missing row、row/JSON/native digest drift、cross-Lease/source mismatch或多值都必须失败关闭。

### B3. Scope 与零副作用

validated Store view 必须 private-field、non-Clone、non-Serde，并线性消费 rebuilt execution scope。源码扫描必须证明：

- response/Carrier 无 consumer、project、Provider owner scope；
- resolver 没有 `INSERT/UPDATE/DELETE`、migration、cache、idempotency或 replay write；
- 不修改 v188-v193 native digest公式、meter、state或money；
- raw parser/DTO 不能构造 Store validated view或进入 writer。

## 4. C — Service、HTTP 与 MCP

必须新增且只新增以下 read surface：

```text
GET /api/me/compute/attempt-leases/:lease_id/execution-verification-source-lineage
GET /api/admin/compute/attempt-leases/:lease_id/execution-verification-source-lineage

compute_get_my_execution_verification_source_lineage
compute_admin_get_execution_verification_source_lineage
```

验收源码必须证明：

- HTTP 只有 path `lease_id`，没有 query/body；
- MCP input properties exact `lease_id`、required且 `additionalProperties=false`；
- participant 只允许 historical consumer 或 Provider owner；
- ordinary MCP 继续 project isolation；admin MCP 同时有 transport project membership和platform role gate；
- success response exact 5 string keys、`read_effect=none`、kind固定；
- malformed/401/403/participant 404/admin 404|409 的失败语义不按 error message字符串分类。

旧六 GET/六 MCP 不改变路径、工具名、response 或 kind；新增后总数为八 GET/八 MCP。

## 5. D — PC 解析与组合

PC 必须用独立合同文件解析新 profile，复用共享 schema/domain/JCS/SHA helper，但不得扩大基础两-profile union。源码验收
必须证明：

- read response exact-5、Carrier exact-6、lineage exact-7、六 refs exact-key；
- UTF-8 byte limit、JSON parse、JCS byte-equal、domain digest与内外 kind/digest逐项检查；
- `verification.execution_lineage_digest == execution.response.lineage_digest`；
- verification/execution 两份 Execution Receipt ID/digest逐字相等；
- 所有可读 execution行请求 verification；release仍只在 released row请求；
- execution/verification/settlement/release 任一失败时整卡失败，不部分显示“已验证”；
- bounded retry、component unmount/新请求 stale guard保留；
- canonical JSON不进入持久化、日志、导出或 writer。

## 6. E — 兼容与范围负例

源码/文档审查必须明确拒绝：

- 用新 kind替换旧 execution kind，或让旧 endpoint返回新 profile；
- 把 v193 完整 meter、policy、outcome、actor/time复制进 Carrier；
- 为 rejected/disputed v192 发明 nullable Execution Receipt；
- 把 provider declaration、platform observation或本 Carrier称为真实可信计量；
- 把 v200/v201 account-level提款绑定到某个 release/Lease；
- 通过新 table、universal graph、backfill或 current-head projection实现读取；
- 用源码已写或旧 owner验证结果提高本 profile的 compile/run/pass/F0 状态。

## 7. 本批静态证据与延后项

本批只允许的证据是 owned-source rustfmt、diff check、源码符号/禁线扫描、文档本地链接与模块化检查。它们只能支持
`source_review_only`，不能写成编译或运行通过。

未来解除架构期限制后，至少补齐：

1. Domain literal golden与negative测试运行；
2. fresh/reopen SQLite retained reconstruction与每条 splice drift negative；
3. participant/admin HTTP/MCP auth、project isolation、404/409矩阵；
4. PC TypeScript、lint、静态合同、生产构建和浏览器失败/重试/stale验收；
5. 旧三 profile literal bytes/digest回归。

上述运行证据全部缺失时，状态固定为：

```text
implementation=source_written/implementation_uncompiled/implementation_unrun
verification=source_review_only
compiled=0 run=0 passed=0 failed=0
f0_exit_gate=not_met
```
