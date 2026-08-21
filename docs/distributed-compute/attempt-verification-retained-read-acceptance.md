---
title: Attempt Verification retained read 验收
reviewed_at: 2026-08-22
status: current
owners: backend, ai-economy, pc
design_status: design_frozen
implementation_status: implementation_uncompiled
verification_status: source_review_only
---

# Attempt Verification retained read 验收

## 1. 验收对象与真实状态

本页只验收 [authority](attempt-verification-retained-read-authority.md) 冻结的 v192 native retained read，不重新验收原
v192 writer，也不验收依赖 v193 的 `execution_verification_source_v1` Carrier。

| 层 | implementation | verification | evidence |
|---|---|---|---|
| Authority/ABI | `design_frozen` | `source_review_only` | current docs |
| Retained Store/view | `source_written` | `uncompiled/unrun` | `passed=0 failed=0` |
| Service/HTTP/MCP | `source_written` | `uncompiled/unrun` | `passed=0 failed=0` |
| PC parser/conditional equality | `source_written` | `uncompiled/unrun` | `passed=0 failed=0` |
| Migration/table/writer | `none` | `not_applicable` | zero effect |

本批不编译、不运行测试、服务、SQLite、浏览器、迁移或网络。旧 v192 GET、writer 或 PC 构建证据不能继承；F0 gate 固定
`not_met`。

## 2. A — Native response ABI

### A1. Exact-52

源码合同必须证明成功响应仍为 `ComputeAttemptVerificationDecisionReceipt` 原生 exact 52 keys，schema 固定
`compute_federation.attempt_verification_decision.v1`，没有额外 envelope、scope、Carrier、`read_effect` 或 v193 字段。

验收至少检查：

- exact key count=52，missing/unknown/null-for-required 均失败关闭；
- ID trim-stable，revision/sequence/fencing 为安全正整数，摘要为原 v192 native exact value；
- reason codes、verified/compensable meter 与 digest 同时重建；
- `accepted/rejected/disputed` 全部可读，read 返回 `replayed=false`；
- effect 字段保持原 v192 历史内容，但本次 read 本身零副作用。

### A2. 不造第二份 owner

读取只能复用原 v192 normalize/audit/digest policy。禁止新 schema、新 canonical digest、字段重命名、meter copy owner、
nullable Execution Receipt 或 current projection。

## 3. B — Historical retained reconstruction

### B1. Lease-only root

Store public facade 只接收 `lease_id`，在一个 deferred read snapshot 内读取 exact-zero-or-one v192。源码扫描必须排除：

- caller-supplied decision/evidence ID 或 digest；
- receipt-ID、latest/current、cache、materialized view 或 repair fallback；
- v193、Execution Receipt、finalization、settlement 依赖；
- 任何 `INSERT/UPDATE/DELETE`、migration、backfill 或 read-side idempotency。

### B2. v188-v192 exact historical owners

源码合同必须证明：

```text
v192 terminal candidate pair = historical v189 id/event
v192 consumer review pair = historical v190 id/event
v192 platform observation pair = historical v191 id/event
v189 final snapshot ID/sequence/provider digest = exact v188
v191 final provider digest = exact v188 cumulative usage digest
v192 final provider digest = exact v188 cumulative usage digest
v192 platform observed digest = historical v191 cumulative observed usage digest
v189-v192 Lease/Provider/consumer/Job/Reservation/Claim/source Lease/fencing = same history
historical Reservation Job/Offer/Provider/Claim = v192 Job/selected Offer/Provider/Claim
historical source Lease id/revision/digest/fencing/Job/Reservation/Provider/consumer = v192 source bindings
```

historical v189 必须决定 exact v188 sequence；v190-v192 与 Reservation 使用 historical seam。任何 missing、multi-row、
row/JSON/native digest 或 cross-source drift 均失败关闭；source Lease 必须是 owner-audited running version，不得只相信
candidate 保存的状态字符串。

### B3. Validated view 与 scope

Store 输出必须是 private-field、non-Clone、non-Serde validated view，同时携带 historical consumer、optional project、
Provider owner scope。Scope 不得进入 exact-52、Debug、日志、MCP 或 PC；raw DTO/parser/caller 不能取得授权能力。

## 4. C — Service、HTTP、MCP 与错误

源码必须保留且只开放 exact 2 GET：

```text
GET /api/me/compute/attempt-leases/:lease_id/verification-decision
GET /api/admin/compute/attempt-leases/:lease_id/verification-decision
```

两条 GET 只有 path `lease_id`，无 query/body；admin GET 与 POST 同 path 不得串义。新增 exact 2 MCP：

```text
compute_get_my_attempt_verification_decision
compute_admin_get_attempt_verification_decision
```

MCP input properties exact `lease_id`，required 且 `additionalProperties=false`。participant 只允许 historical consumer 或
Provider owner；普通 MCP 继续 project isolation；admin surface 需要 `admin/owner`，admin MCP 仍受 transport project
membership 约束。

typed error/source contract 必须覆盖：

```text
400 ATTEMPT_VERIFICATION_RETAINED_INVALID_LEASE_ID
400 ATTEMPT_VERIFICATION_RETAINED_INVALID_REQUEST_INPUT
401 ATTEMPT_VERIFICATION_RETAINED_UNAUTHENTICATED
404 ATTEMPT_VERIFICATION_RETAINED_NOT_VISIBLE
403 ATTEMPT_VERIFICATION_RETAINED_PROJECT_FORBIDDEN
404 ATTEMPT_VERIFICATION_RETAINED_NOT_FOUND
409 ATTEMPT_VERIFICATION_RETAINED_INTEGRITY_CONFLICT
403 ATTEMPT_VERIFICATION_RETAINED_ADMIN_FORBIDDEN
500 ATTEMPT_VERIFICATION_RETAINED_INTERNAL_ERROR
```

participant 的 missing/integrity/nonparticipant 必须统一为 `NOT_VISIBLE` 404；admin 才区分 404 missing 与 409 integrity。
connection、transaction、SQLite busy/I/O 与 SQL/schema operational failure 必须返回脱敏 500/internal，不能冒充
missing/integrity；row/JSON/column/native digest 与 cross-owner drift 仍归 integrity。分类必须使用 typed error/type chain，
不得按错误消息文本分类。

## 5. D — PC parsing 与 14 等式

PC exact parser 必须拒绝 missing/extra key、非法数组/枚举/数值/摘要，并允许 `/compute-verification` 在无 v193 时读取
accepted/rejected/disputed。

当且仅当页面已有 validated `execution_verification_source_v1` 时，源码必须逐项验证：

1. final snapshot ID；
2. final sequence；
3. final Provider usage digest；
4. terminal candidate ID；
5. terminal candidate event digest；
6. consumer review ID；
7. consumer review event digest；
8. platform observation ID；
9. platform observation event digest；
10. platform observed usage digest；
11. verification decision ID；
12. verification event digest；
13. verified usage digest；
14. compensable usage digest。

右侧字段必须逐字来自 Carrier 的 `provider_declared_usage`、`terminal_candidate`、`consumer_review`、
`platform_observation` 与 `verification_decision` refs。任一等式失败时整张因果卡失败；无 v193/Carrier 时只跳过条件组合，
不能让 native read 失败。bounded retry、unmount/new-request stale guard 必须保留；数据不得持久化、记录或导出。

## 6. E — 零效果与信任负例

源码与文档必须共同拒绝以下外推：

- 把 retained read 或 14 条等式称为可信计量、真实执行或独立验证；
- 把人工 `conservative_min_v1`、Provider 声明或平台观测升级为 signed/trusted；
- 让 rejected/disputed 依赖 nullable v193，或仅允许 accepted 读取；
- 读取时创建 Execution Receipt、推进状态、消费容量或移动资金；
- 修改 v188-v192 native digest、原 writer、policy、meter 或 immutable history；
- 用旧 v192/PC 测试或源码扫描提高 compiled/run/pass/F0 状态。

## 7. 本批静态证据与延后项

本批只允许 apply-patch 后的 diff、源符号/禁线、文件尺寸、文档本地链接与模块化静态检查。这些只能支持
`source_review_only`。

未来解除架构期限制后至少运行：

1. fresh/reopen SQLite accepted/rejected/disputed retained read 与 splice drift negatives；
2. participant/admin HTTP 认证、404/409、GET/POST 同 path 和 body/query 拒绝矩阵；
   另覆盖 connection/begin/query/commit operational 500，且与 owner drift 409/404 分型；
3. ordinary/admin MCP schema、project membership、role 与脱敏矩阵；
4. PC exact-52 parser、14 条等式、Carrier absent、retry/stale 及生产构建；
5. 原 v192 writer/digest 与 existing endpoint compatibility regression。

在这些证据缺失时，状态固定为：

```text
design=design_frozen
implementation=source_written/implementation_uncompiled/implementation_unrun
verification=source_review_only
compiled=0 run=0 passed=0 failed=0
f0_exit_gate=not_met
```
