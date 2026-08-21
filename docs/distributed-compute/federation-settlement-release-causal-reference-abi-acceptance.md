---
title: 联邦结算释放历史因果引用 Carrier ABI 验收
status: current
reviewed_at: 2026-08-22
owners: backend, pc, ai-economy
implementation_status: implementation_uncompiled
verification_status: source_review_only
design_scope: federation_settlement_release_historical_causal_reference_abi_v1
---

# 联邦结算释放历史因果引用 Carrier ABI 验收

## 1. 当前证据与诚实状态

本批只验收 `settlement_release_source_v1` 的 Domain→Store→Service→HTTP/MCP→PC 源码闭合。没有编译、运行、迁移、
HTTP/MCP、浏览器或数据库证据；因此唯一允许的状态为：

```text
abi=design_frozen
profile=settlement_release_source_v1
domain/store/service/http_mcp/pc_client=source_written
implementation=uncompiled/unrun
compiled/run=0/0
passed/failed=0/0
verification=source_review_only
migration/table/cache/backfill=none
native_digest_rewrites=0
writer/state/money_effects=0
f0_exit_gate=not_met
```

任一源码层或 §8 source-contract缺失时，对应层必须降级，不能保留 `source_written`。

## 2. Envelope、compatibility 与 canonical matrix

| item | acceptance |
|---|---|
| envelope | exact 6 keys=`schema,lineage_kind,lineage_digest,canonicalization,digest_algorithm,lineage` |
| constants | core V1 schema、JCS、SHA-256、domain与262144-byte上限逐字复用 |
| kind | 新入口只接受/返回 `settlement_release_source_v1`；null/unknown/base kind全部拒绝 |
| digest | `SHA256(domain UTF-8 || 0x00 || UTF8(JCS(envelope with lineage_digest="")))`，lowercase 64-hex |
| bytes | input/output必须UTF-8 RFC8785 JCS逐字byte-equal；invalid UTF-8、whitespace、key order/escape漂移拒绝 |
| shape | deny unknown/duplicate/missing/null/float/unsafe integer/trailing bytes；disallowed variant字段必须 absent |
| compatibility | 旧 execution/settlement golden bytes/digest零变化，旧endpoint永不返回新kind |
| response | exact 5 string keys、固定read schema、outer kind/digest等于inner、`read_effect=none` |

source golden必须为 release profile保存至少一份独立 literal canonical JSON与literal digest；expected digest不得由被测 helper
现场生成。旧两份 golden必须继续逐字相同。Negative至少覆盖 self-digest key删除而非空串、错误domain、旧endpoint kind
substitution、新endpoint base-kind substitution与跨profile lineage splice。

## 3. Exact refs 与 lineage shape

`lineage` exact 6 keys且顺序只由JCS决定：

```text
attempt_settlement
settlement_lineage_digest
source_settlement_posting
release_gate
settlement_release
release_posting
```

| object | exact keys |
|---|---|
| Attempt Settlement | `settlement_receipt_id,settlement_receipt_digest,settlement_event_digest` |
| source Posting | `settlement_posting_id,settlement_posting_digest` |
| Release | `settlement_release_id,settlement_release_event_digest` |
| release Posting | `settlement_release_posting_id,settlement_release_posting_digest` |
| Challenge | `settlement_challenge_id,settlement_challenge_event_digest` |
| Resolution | `settlement_challenge_resolution_id,settlement_challenge_resolution_event_digest` |
| Correction | `settlement_correction_id,settlement_correction_event_digest` |
| correction Posting | `settlement_correction_posting_id,settlement_correction_posting_digest` |

全部ID必须nonempty/trim-stable/control-free，全部digest必须lowercase SHA-256。Native digest只按exact string携带；uppercase、
trim、Carrier JCS替代、ID相同digest漂移全部拒绝。Lineage含任何balance/amount/revision-after/actor/time/status/current/
replayed/effect/payment字段都拒绝。

## 4. Gate variant matrix

| kind | exact keys | required owner truth |
|---|---|---|
| `no_challenge` | `gate_kind,challenge_gate_digest` | v198 gate none/unblocked/no correction且v196 absent |
| `resolved_challenge` | `gate_kind,challenge_gate_digest,resolution_action,challenge,resolution` | action exact `rejected|withdrawn`，v196/v197 exact，v199 absent |
| `accepted_corrected` | `gate_kind,challenge_gate_digest,challenge,resolution,correction,correction_posting` | v197 accepted、v199 exact receipt/posting、gate corrected/unblocked |

Negative必须覆盖：open、accepted但无v199、rejected/withdrawn携correction、none携ref、resolved action accepted、
accepted_corrected引用非accepted resolution、cross-settlement/challenge/resolution/correction splice、gate digest漂移、
correction posting ID/digest splice、extra/null字段与unknown kind/action。

## 5. Historical Store reconstruction matrix

一个 Deferred read transaction内必须全部成立：

1. Lease命中exact一份 retained v198；0是not-found，multi/drift是integrity，禁止 current/latest fallback；
2. 同一 transaction调用既有 settlement historical resolver；`attempt_settlement`三字段与 rebuilt subject相等，
   `settlement_lineage_digest`与 rebuilt digest相等；
3. v198 lease、settlement ID/event与source posting pair逐字等于 v195 owner receipt；
4. v198 request/receipt/gate raw JSON逐字等于owner serializer，数据库列、request/event/gate digest全部重算通过；
5. Release Posting ID/digest、receipt金额/after-balance/revision snapshot与exact ordered四腿相等；缺腿、额外腿、换序、
   account/direction/state/amount/revision漂移均拒绝；
6. policy、72小时截止、released_at与原金额/accepted-corrected净额由owner historical audit重证；
7. §4所需 v196/v197/v199由各自 retained owner seam解析，0/multi/raw JSON/native digest/posting drift均拒绝；
8. resolver只返回 private-field/non-Clone/non-Serde view与线性复用的private scope；raw DTO/caller不能构造。

历史 resolver不得读取或要求：current challenge gate、current account balance head、withdrawal head、current Job/Lease/
Reservation/Claim、部署current inventory。源码负例必须锁定普通 public/fresh owner audit仍保留其原 current/head policy，不能因
新增 historical seam而被全局关闭。

重复读取同一 retained roots必须生成逐字相同 JSON/digest；resolver不能INSERT/UPDATE/DELETE、开Immediate事务、写cache、
回填receipt或把validated view传给任何writer。

## 6. Service、HTTP/MCP 与权限 matrix

Service只新增 participant/admin两个release read facade。HTTP exact routes：

```text
GET /api/me/compute/attempt-leases/:lease_id/settlement-release-source-lineage
GET /api/admin/compute/attempt-leases/:lease_id/settlement-release-source-lineage
```

MCP exact tools：

```text
compute_get_my_settlement_release_source_lineage
compute_admin_get_settlement_release_source_lineage
```

每个入口只接受 `lease_id`；HTTP无query/body，MCP schema exact object/required/additionalProperties=false。Participant必须
命中 historical consumer或Provider owner；普通MCP还必须匹配current project。Admin HTTP/MCP分别复用既有admin gate，
admin MCP不得用project membership替代platform role。

| caller/condition | public result |
|---|---|
| malformed lease/query/body | stable invalid-input / HTTP 400；不查owner |
| unauthenticated | 401 |
| ordinary participant不匹配、owner drift或无v198 | 脱敏404，不区分存在性 |
| ordinary MCP project缺失/不等 | stable project-forbidden tool error，不泄露expected project |
| admin role/project gate失败 | 403 |
| authorized admin无v198 | stable not-found / 404 |
| authorized admin owner integrity drift | stable integrity-conflict / 409，detail脱敏 |

错误必须由typed enum映射，禁止匹配 `anyhow`文本。成功只能是 exact-5、kind固定release、无scope/actor/replayed。

## 7. PC 三响应与 hardening acceptance

Released history row必须读取 execution、settlement、release三份响应；pending/no-release row只读前两份。前端必须：

- 对三份响应分别做 exact5、UTF-8/JCS byte-equal、domain digest、inner/outer kind+digest验证；
- 证明 settlement→execution digest/receipt两项既有等式；
- 证明 release→settlement digest与Attempt Settlement三字段等式；
- 只在全部成功后显示“已核验”；任一失败整卡失败并提供可重复、有界的重试；
- 防重复点击、scope/Lease切换与unmount后的stale response；不在错误中回显隐藏project/owner/integrity细节；
- 完整digest与canonical JSON可折行/详情查看，不因长文本撑破现有布局。

Negative/source-contract覆盖：released漏第三请求、pending误把404当integrity、release kind错、两个跨响应digest splice、
Attempt Settlement任一字段漂移、单项失败仍部分显示、旧请求覆盖新Lease、增加route/navigation/persistence/export/writer。

## 8. Zero-effect、source-written 与最终报告门

Static source-contract必须证明：

1. 新profile无migration/table/cache/backfill/native digest rewrite；
2. resolver只开Deferred read transaction，无writer/fresh action/caller digest；
3. 两GET/两MCP路径、tool名称、exact input、role/project gate与typed error均存在且唯一；
4. PC release合同在独立<500行叶中，现有base合同不继续膨胀；
5. v195-v199普通owner current/head policy未被削弱；
6. legacy execution/settlement Carrier与v195-v199 receipt/digest bytes零变化。

最终报告必须逐项写：

```text
abi=<design_frozen|changed>
profile=<settlement_release_source_v1|other>
domain/store/service/http_mcp/pc_client=<absent|partial|source_written|verified>
implementation=<uncompiled|compiled>/<unrun|run>
compiled=<0|1>
run=<0|1>
passed=<N>
failed=<N>
migration/table/cache/backfill=<none|details>
native_digest_rewrites=<N>
writer/state/money_effects=<N>
f0_exit_gate=<not_met|met>
```

在本批禁止编译/运行的约束下，任何 `verified`、非零passed、F0 met、v198新运行证据、真实付款或production-ready
表述都失败。
