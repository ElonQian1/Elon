---
title: 联邦结算释放历史因果引用 Carrier ABI 权威
status: current
reviewed_at: 2026-08-22
owners: backend, pc, ai-economy
implementation_status: implementation_uncompiled
verification_status: source_review_only
design_scope: federation_settlement_release_historical_causal_reference_abi_v1
---

# 联邦结算释放历史因果引用 Carrier ABI 权威

## 1. 唯一结论与当前状态

本页冻结 `settlement_release_source_v1`：它只读地把既有
[`settlement_source_v1`](federation-historical-causal-reference-abi-authority.md) 与 v198 Release Receipt、原/释放
Posting 及释放时已封存的 challenge gate 串成一份确定性历史 Carrier。它证明“某笔 v195 pending 结算已按 v198
移动到平台内部 available”，不证明 withdrawn、银行/钱包/链上付款或外部到账。

```text
abi=design_frozen
carrier_profile=settlement_release_source_v1
domain/store/service/http_mcp/pc_client=source_written
implementation=uncompiled/unrun
compiled=0
run=0
passed=0
failed=0
verification=source_review_only
migration/table/cache/backfill=none
native_digest_rewrites=0
writer/state/money_effects=0
f0_exit_gate=not_met
```

上述 `source_written` 只表示静态源码已落盘；本批没有运行 Cargo、前端构建、测试、迁移、HTTP/MCP、浏览器或
生产数据库。v198 原业务权威和余额效果不因本 Carrier 获得新的运行证据。

## 2. Additive profile 与唯一 envelope

本 profile 逐字复用核心 Carrier 的：

- `schema=compute_federation.core_historical_causal_reference.v1`；
- `canonicalization=rfc8785_jcs`、`digest_algorithm=sha256`；
- `DIGEST_DOMAIN=ELON-COMPUTE-CORE-HISTORICAL-LINEAGE-V1`；
- exact 6-key envelope、自摘要空串投影、UTF-8 RFC 8785 JCS 与 262144-byte 上限；
- exact-5 read response，`schema=compute_federation.core_historical_causal_reference.read.v1`、
  `read_effect=none`。

这是 endpoint-only additive enum，不是静默改写旧合同：

1. execution endpoint 永远只返回 `execution_source_v1`；
2. settlement endpoint 永远只返回 `settlement_source_v1`；
3. 只有本页新 endpoint/tool 可返回 `settlement_release_source_v1`；
4. 旧 execution/settlement Carrier JSON、digest、response、route 与 client bytes 必须零变化；
5. 不调用新入口的旧 strict client 可以继续拒绝未知 kind；服务端每个入口仍以 expected-kind 失败关闭；
6. 未来新增其他 kind 必须另立 authority，不能把本扩展改成开放 enum 或 generic causal graph。

## 3. Exact lineage 与引用 primitive

Envelope `lineage_kind` 必须逐字是 `settlement_release_source_v1`，`lineage` 必须 exact 6 keys：

```json
{
  "attempt_settlement": {},
  "settlement_lineage_digest": "<lowercase sha256>",
  "source_settlement_posting": {},
  "release_gate": {},
  "settlement_release": {},
  "release_posting": {}
}
```

六项均 required、non-null、无 extra：

| field | exact shape |
|---|---|
| `attempt_settlement` | `{settlement_receipt_id,settlement_receipt_digest,settlement_event_digest}` |
| `settlement_lineage_digest` | 重新生成 exact `settlement_source_v1` 得到的 lowercase 64-hex digest |
| `source_settlement_posting` | `{settlement_posting_id,settlement_posting_digest}` |
| `settlement_release` | `{settlement_release_id,settlement_release_event_digest}` |
| `release_posting` | `{settlement_release_posting_id,settlement_release_posting_digest}` |
| `release_gate` | §4 三个 structural variant之一 |

所有 ID 都是 nonempty、无首尾空白、无 control 的 string；所有 `*_digest` 都是 lowercase 64-hex string。
这些 native digest 按所属 v195-v199 owner 算法作为 opaque bytes逐字携带；Carrier不得 trim、case-fold、JCS重算或替换。
本 profile 不含 balance、amount、revision-after、actor、time、status、current、replayed、effect 或付款字段。

## 4. Release gate 三个 exact variant

`release_gate` 是封闭的 internally tagged union；disallowed key必须 absent，不能用 null占位。

### 4.1 `no_challenge`

```json
{"gate_kind":"no_challenge","challenge_gate_digest":"<digest>"}
```

它只在 v198 封存的 gate 为 `status=none,blocked=false,correction_required=false` 且六个 challenge/resolution/
correction引用全部 absent时成立；同一 Settlement Receipt 必须没有 v196 row。

### 4.2 `resolved_challenge`

```json
{
  "gate_kind":"resolved_challenge",
  "challenge_gate_digest":"<digest>",
  "resolution_action":"rejected",
  "challenge":{"settlement_challenge_id":"<id>","settlement_challenge_event_digest":"<digest>"},
  "resolution":{"settlement_challenge_resolution_id":"<id>","settlement_challenge_resolution_event_digest":"<digest>"}
}
```

`resolution_action` 只允许 `rejected|withdrawn`。v196/v197 exact retained receipts必须相互绑定并逐字等于 gate refs；
gate 必须 `blocked=false,correction_required=false`，correction refs absent，同一 resolution不得有 v199。

### 4.3 `accepted_corrected`

```json
{
  "gate_kind":"accepted_corrected",
  "challenge_gate_digest":"<digest>",
  "challenge":{"settlement_challenge_id":"<id>","settlement_challenge_event_digest":"<digest>"},
  "resolution":{"settlement_challenge_resolution_id":"<id>","settlement_challenge_resolution_event_digest":"<digest>"},
  "correction":{"settlement_correction_id":"<id>","settlement_correction_event_digest":"<digest>"},
  "correction_posting":{"settlement_correction_posting_id":"<id>","settlement_correction_posting_digest":"<digest>"}
}
```

v197 action必须 `accepted`；v199 exact retained receipt/posting必须绑定同一 v195/v196/v197，gate 必须逐字是
`status=accepted_corrected,blocked=false,correction_required=false`。open、未纠正 accepted、未知 action或混合 variant
全部失败关闭，不能产生 Carrier。

三个 variant 的 `challenge_gate_digest` 都必须等于 v198 owner 对完整 gate JSON 使用既有 raw-serde SHA-256 得到的
摘要；不能用 Carrier JCS/domain替代。

## 5. Store retained-only reconstruction

唯一入口只接 `lease_id`。Store 在一个 `BEGIN DEFERRED` read snapshot 内：

1. 按 Lease exact-1 读取并执行 v198 historical owner audit；0 返回 not-found，multi、row/JSON/digest drift返回
   integrity failure，禁止 latest/current fallback；
2. 通过现有 historical resolver重建同一 Lease 的 `settlement_source_v1`，线性消费其 private access scope；
3. 证明 v198 的 Lease、v195 receipt ID/event、source posting ID/digest逐字等于 rebuilt settlement；
4. 证明 `attempt_settlement` 等于 rebuilt settlement subject，`settlement_lineage_digest` 等于 rebuilt digest；
5. 重算 v198 request/event digest，逐字审计 request/receipt/gate JSON、数据库列、Release Receipt与 immutable
   Release Posting；账本腿必须 exact ordered 4 条且金额/after-balance/revision snapshot与 receipt一致；
6. 证明 `released_at >= settled_at + 72h`、policy ID/version固定且 Release Receipt中的金额来自 v195或合法 v199；
7. 按 §4 exact variant调用 v196/v197/v199各自 retained historical owner audit；不得把 gate JSON中的 caller-shaped
   ID当成已验证引用；
8. 返回 private-field、non-Clone、non-Serde validated view；raw DTO/parser/caller不能构造或进入 writer。

Historical audit 不读取 current challenge gate、current account balance head、withdrawal head、current Job/Lease/Claim/
Reservation或部署 current inventory。它仍审计 v195/v198/v199 immutable posting与账本腿，但不要求后续 current投影仍
停留在释放发生时的余额。普通 v195-v199 public/fresh read继续保留原 current/head policy，不得因本页被削弱。

同一 source每次解析必须生成逐字相同 canonical JSON/digest。它不写回 v195-v199、不登记 replay/idempotency、不
创建 cache/table/index，也不给下一笔动作保留 current authority。

## 6. Access scope、Service、HTTP 与 MCP

Release resolver必须逐字复用 rebuilt settlement的 private access scope，并重证 v198 consumer/provider account与
scope一致。Scope 只含 historical consumer account、optional project与 historical Provider owner account；不进入
Carrier/response/serde/Debug/client，也不跨 transaction/await保留授权。

Service只新增 participant/admin两个 read facade。HTTP只新增：

```text
GET /api/me/compute/attempt-leases/:lease_id/settlement-release-source-lineage
GET /api/admin/compute/attempt-leases/:lease_id/settlement-release-source-lineage
```

两条路由只收 path `lease_id`，无 query/body。普通 caller必须是 historical consumer或Provider owner；admin复用既有
platform admin/owner gate。MCP只新增：

```text
compute_get_my_settlement_release_source_lineage
compute_admin_get_settlement_release_source_lineage
```

input exact `{lease_id}`。普通 MCP 在 participant predicate后仍要求 historical `project_id == current MCP project_id`；
admin MCP同时要求 transport project membership与platform admin role。成功只返回核心 exact-5 response，kind固定本页值。

失败沿核心 read ABI：malformed=400；unauthenticated=401；project/admin forbidden=403；participant不可见与合法但无
v198=404且不泄露存在性；admin exact v198缺失=404；owner historical drift=admin 409，participant仍脱敏404；不得按
`anyhow` message文本分类。

## 7. PC 三响应闭合

PC 只在结算生命周期已带 v198 Release Receipt时请求本 endpoint；pending row继续只核 execution+settlement，不把
release absent/404冒充 integrity failure。Released row在显示前必须：

1. 分别对 execution、settlement、release exact-5响应做 exact-key、UTF-8/JCS byte-equal、domain digest与内外
   kind/digest检查；
2. 证明 `settlement.execution_lineage_digest == execution.lineage_digest`；
3. 证明 `release.settlement_lineage_digest == settlement.lineage_digest`；
4. 证明 release `attempt_settlement` 与 settlement `attempt_settlement`三字段逐字相等；
5. 任一 network/permission/shape/digest/跨响应等式失败时整张核验卡失败关闭，并提供有界重试；不得部分显示“已验证”。

PC 不持久 Carrier、不把 canonical JSON写日志/导出、不新增导航或writer。长 digest/JSON必须在现有可折行/详情容器内
显示，loading、失败、重试与组件卸载后的 stale response均沿现有 hardened状态机处理。

## 8. Source-written 门与明确禁线

本批静态源码必须同时具备 Domain exact DTO/canonical/golden/negative、v195-v199 retained owner seams、Store by-Lease
resolver与splice negatives、Service/2 HTTP/2 MCP、PC三响应验证、zero-writer/migration/effect source-contract，才可保持
各层 `source_written`。任一层缺失即降为对应 `absent|partial`，不能用文档补源码空洞。

明确禁止：

- 新 migration/table/column/cache/backfill或向 v198 receipt写 Carrier digest；
- 修改任何 legacy digest preimage、账本posting、金额、余额或challenge状态；
- 把 `available`称为withdrawn、external paid、cash settled或链上 finality；
- current/latest fallback、caller-supplied digest/ref/kind、raw SQL复制owner摘要公式；
- 用 generic optional ref soup、null、开放 `causes[]` 或未知 gate/action扩展本 V1；
- 因源码落盘而提高 compiled/run/passed/failed、F0 gate或 v198运行成熟度。

验收矩阵见 [acceptance](federation-settlement-release-causal-reference-abi-acceptance.md)；原业务语义仍以
[v198 release authority](attempt-settlement-release-api.md) 为准。
