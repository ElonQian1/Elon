---
title: 联邦核心历史因果引用 Carrier ABI 验收
reviewed_at: 2026-08-21
status: current
owners: backend, node, ai-economy, pc
design_status: design_frozen
design_scope: federation_core_historical_causal_reference_carrier_abi_v1
implementation_status: implementation_uncompiled
verification_status: source_review_only
---

# 联邦核心历史因果引用 Carrier ABI 验收

## 1. 当前证据与状态

唯一规范是 [Carrier authority](federation-historical-causal-reference-abi-authority.md)。本页当前只记录 source review；
Domain、Store resolver、source-contract、golden与negative test，以及 Service、HTTP/MCP、PC 客户端的 additive read
adoption 源码已存在。table、migration、writer 仍不存在，也没有 compile 或 runtime 证据：

```text
federation_core_historical_causal_reference_carrier_abi=design_frozen
carrier_profiles=execution_source_v1/settlement_source_v1
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

F0 的 read adoption 源码已落盘，但 compile、runtime 与已运行合同测试退出门仍未达到；因此 F0 仍是
`not_met`。不得把本文 matrix 写成已运行结果。

## 2. Canonical acceptance matrix

| case | 必须结果 |
|---|---|
| envelope | exact 6 keys；schema/kind/JCS/SHA常量逐字相等 |
| kind | 仅 `execution_source_v1\|settlement_source_v1`；null/unknown拒绝 |
| self digest | 保留 `lineage_digest=""` 后 domain+NUL+JCS；删除key、普通serde SHA或native digest代入拒绝 |
| bytes | UTF-8 I-JSON、parse→JCS byte-equal、≤262144；duplicate/unknown/missing/float/trailing拒绝 |
| numbers | revision/version/epoch/fencing为 `1..=2^53-1` integer；0、负数、float、超界拒绝 |
| JSON types | 顶层常量/ref ID/digest为string、`lineage`为object；数字字符串、number ID、array/boolean替代拒绝 |
| strings | 不trim、不Unicode normalize、不case fold；source exact byte漂移拒绝 |
| forbidden | id/status/time/actor/current/replayed/effects/null/extra profile字段拒绝 |

source-written golden为两种 profile各固定至少一份完整 literal bytes/digest，并以未运行测试源码覆盖同一 native refs在
kind、role或 source/terminal位置变化时得到不同 Carrier digest；当前没有通过计数。

## 3. Reference shape acceptance

| ref | exact keys / 必须边界 |
|---|---|
| Provider | ID+policy revision+opaque native digest；历史 owner exact 1 |
| Pool | ID+epoch+revision+opaque native digest；epoch遗漏/折叠拒绝 |
| Offer | Provider ID+Offer ID/version/native digest |
| Snapshot | ID+native digest；不得发明 revision |
| Job / Reservation / Claim | 各自 ID+revision+native digest |
| source Lease | ID+v189 source revision/native digest+fencing；current或v194 terminal替换拒绝 |
| Execution Receipt | execution receipt ID+native digest |
| Finalization | finalization ID+native event digest |
| Attempt settlement | settlement receipt ID/native digest+outer event digest；只给 inner pair拒绝 |

所有 native digest只作 owner-opaque exact value。Carrier owner自行重算 Provider/Offer/Claim/Receipt digest、转小写，或让
Carrier digest替代 owner digest，均必须失败。SKU只有 exact Offer/Snapshot embedded qualifier；独立 SKU resolver/revision拒绝。

## 4. Execution source matrix

`execution_source_v1.lineage` 必须 exact 9 object keys且无null。Store negative test源码至少覆盖（当前未运行）：

- Provider↔Offer metadata、Pool↔Offer/Claim、Offer↔Snapshot/Job/Reservation逐字相等；
- Snapshot↔Job/Reservation、Job↔Reservation、Reservation↔Claim逐字相等；
- v189/v192 的 Job、Reservation、Claim、source Lease和fencing逐字相等，v193 owner audit解析回同一组 refs；
- Execution Receipt 的 Job/Reservation/Lease/attempt/fencing/Provider/executor/Offer回指同链；
- v185、v188-v193 native owner audit全部通过后才产生 sealed validated view。

必须拒绝 cross-Provider、cross-Offer、cross-Snapshot、cross-Job、cross-Reservation、cross-Claim splice；同 Lease ID换
revision/digest/fencing、用读取时current Lease、用 v194 terminal Lease、只解析 receipt JSON猜父链也必须拒绝。

Plan/route/capability/budget/usage/evidence/artifact缺省不是 null key；它们根本不属于 V1 shape，仍由各 owner证明。

## 5. Settlement source matrix

`settlement_source_v1.lineage` 必须 exact 9 object/string keys。Store negative test源码至少覆盖（当前未运行）：

- v195 inner Settlement Receipt与 outer event双摘要同时通过；
- v193 exact pair重新生成 `execution_source_v1`，其 digest逐字等于 `execution_lineage_digest`；
- `source_job = v194.terminal_job = v195.source_job`、`terminal_job = v195.terminal_job`、
  `terminal_reservation = v194.terminal_reservation`；误取 v194 running Job或 active Reservation source版本拒绝；
- Provider ID必须同时等于 v193 receipt与 historical Offer；revision/digest必须同时等于 v195 outer与 Offer
  registration metadata；任一同 ID换 revision/digest拒绝；
- terminal Reservation ID必须同时等于 v195 inner与 v193 receipt，revision/digest必须等于 v194 terminal
  Reservation；同 ID换 terminal revision/digest拒绝；
- source Job=`verification_pending`、terminal Job=`settled`，同 Job ID且revision/native digest由 owner审计；
- terminal Reservation不替换 execution carrier的 source Reservation/Claim/Lease；
- Snapshot、Provider historical metadata、价格计算、posting与pending balance各自 owner audit通过。

必须拒绝 execution lineage digest照抄不重算、v193/v195交叉拼接、finalization event漂移、source/terminal Job互换、
terminal ref relabel为execution source、Snapshot或Provider漂移、inner settlement digest正确但outer event错误。

Carrier成功也必须继续报告 internal settlement=`pending`；`available/withdrawn/external_paid/onchain_final`推断一律拒绝。

## 6. Historical/current 与兼容 matrix

| case | 必须结果 |
|---|---|
| canonical-only DTO | 只能证明bytes/digest自洽，不能进入writer |
| historical exact 1 | 全部 retained refs与owner JSON/columns重审后才可sealed |
| historical 0/multi/drift | integrity failure，禁止latest/current fallback |
| expired/revoked source | 可按历史审计；不能恢复current或授权fresh action |
| fresh action | 继续执行其原有currentness/auth/TTL/CAS；Carrier不跨事务保权 |
| old NodeComputeRun | 缺完整链继续partial，禁止backfill伪造 |
| Provider kind | user_node/managed_cluster/external_pool共用core shape；kind-specific roots留在owner组合层 |
| legacy storage | v169-v195 JSON/digest/table/migration零改写 |

source-contract源码锁定 raw HTTP/DTO/caller不能构造 validated view；重复读取必须重建同一 JSON/digest，且 resolver不得
包含row、idempotency replay、状态或资金 writer。该源码当前未编译、未运行，不能据此声称行为已验证。

## 7. Additive read response 与 by-Lease root matrix

| case | 必须结果 |
|---|---|
| public input | 只接受 `lease_id`；query/body=`FEDERATION_LINEAGE_INVALID_REQUEST_INPUT`/400，invalid Lease shape=`FEDERATION_LINEAGE_INVALID_LEASE_ID`/400，且都不查 owner |
| execution root | 只调用 `resolve_compute_execution_source_lineage_for_lease(lease_id)`，由 retained source Lease 闭合 v189-v193 |
| settlement root | 只调用 `resolve_compute_settlement_source_lineage_for_lease(lease_id)`，由同一 source Lease 闭合 v193-v195 并重建 execution carrier |
| fallback | attempt/current/latest、receipt ID、caller supplied digest/revision/kind/currentness 均拒绝 |
| success keys | exact `schema,lineage_kind,lineage_digest,canonical_carrier_json,read_effect`；五值全为 string，无 null/extra |
| schema | exact `compute_federation.core_historical_causal_reference.read.v1` |
| kind | execution endpoint exact `execution_source_v1`；settlement endpoint exact `settlement_source_v1` |
| carrier | `canonical_carrier_json` 是 Store sealed view 的完整 canonical JCS string，不 parse 后重排或摊平 |
| equalities | response kind/digest 分别逐字等于 carrier inner kind/digest |
| effect | exact `read_effect="none"`；不得增加 scope/identity/time/current/replay/status/owner/effect 字段 |

Store resolver 成功后才可形成 private、non-Clone、non-Serde access scope。以下等式必须全部通过：

```text
scope.consumer_account_id
  = historical execution-source Job.job.consumer_account_id
scope.project_id
  = historical execution-source Job.job.project_id
scope.provider_owner_account_id
  = historical Provider.provider.owner_account_id

settlement.scope = rebuilt execution.scope
settlement.source_job.consumer_account_id = settlement.terminal_job.consumer_account_id
settlement.source_job.project_id = settlement.terminal_job.project_id
```

settlement source/terminal Job 的 consumer/project 还必须分别等于 rebuilt execution scope；任一 0、multi、owner
drift，或 `rebuilt_v193.attempt_lease_id / v194.lease_id / v195.lease_id` 三者任一不等，都失败关闭。scope 不得进入
response、Carrier、Debug、serde 或 client，也不得跨
transaction/await 保留 authorization。

## 8. Service、HTTP/MCP 与 project isolation matrix

Service 只允许以下四个 facade：

```text
read_execution_for_participant
read_settlement_for_participant
read_execution_for_admin
read_settlement_for_admin
```

HTTP 必须恰好是四个只读 `GET`，只有 `:lease_id` path 参数，无 query/body：

```text
/api/me/compute/attempt-leases/:lease_id/execution-source-lineage
/api/me/compute/attempt-leases/:lease_id/settlement-source-lineage
/api/admin/compute/attempt-leases/:lease_id/execution-source-lineage
/api/admin/compute/attempt-leases/:lease_id/settlement-source-lineage
```

MCP 必须恰好是四个工具：

```text
compute_get_my_execution_source_lineage
compute_get_my_settlement_source_lineage
compute_admin_get_execution_source_lineage
compute_admin_get_settlement_source_lineage
```

每个 MCP input schema 都是 exact object：只有 string `lease_id`、`required=["lease_id"]`、
`additionalProperties=false`。participant 成功 predicate 必须逐字等价于
`user_id == scope.consumer_account_id || user_id == scope.provider_owner_account_id`。普通 MCP 在 predicate 成功后
还必须满足 `scope.project_id == Some(caller_project_id)`；无 current project 或不等都拒绝。`/api/me` 不接受
project override。Admin HTTP 依赖已有 platform-admin role gate；admin MCP 先通过现有 project-scoped MCP transport
membership gate，再通过 platform-admin role gate。Membership 只授予 transport reachability，不能单独授予 admin lineage authority。

## 9. Error/redaction matrix

| caller / condition | code | HTTP | 必须脱敏 |
|---|---|---:|---|
| unauthenticated / any request | `FEDERATION_LINEAGE_UNAUTHENTICATED` | 401 | 不解析 Lease 或 owner |
| authenticated HTTP caller after route role gate / query 或非空 body | `FEDERATION_LINEAGE_INVALID_REQUEST_INPUT` | 400 | 不查 owner |
| authenticated caller after route role gate / invalid Lease ID shape | `FEDERATION_LINEAGE_INVALID_LEASE_ID` | 400 | 不查 owner |
| participant / missing、owner drift 或 scope 未形成 | `FEDERATION_LINEAGE_NOT_VISIBLE` | 404 | 不区分 missing/integrity |
| participant / scope 成功但非 consumer/provider owner | `FEDERATION_LINEAGE_NOT_VISIBLE` | 404 | 防枚举 |
| ordinary MCP / participant 成功但 project 缺失或不等 | `FEDERATION_LINEAGE_PROJECT_FORBIDDEN` | —（JSON-RPC tool error） | 不回显期待 project |
| admin MCP / project membership gate失败 | 既有 MCP project-access error | 403 | lineage tool/Service 未执行 |
| admin / role 非 platform admin | `FEDERATION_LINEAGE_ADMIN_FORBIDDEN` | 403 | 不产生成功 payload |
| admin / historical root missing | `FEDERATION_LINEAGE_NOT_FOUND` | 404 | 不回显 scope/owner |
| admin / owner/native digest/JSON/column drift | `FEDERATION_LINEAGE_INTEGRITY_CONFLICT` | 409 | 只返回稳定 code |

participant 路径不得通过 status、正文或差异化 detail 暴露 Lease、Provider、project、owner 或 drift 层级。Admin 409
也不得回显 row、native JSON、expected/actual digest、account/project/provider/receipt ID 或 SQL。成功响应始终只有 exact 5 keys。
数字 HTTP status 只验四个 GET 或 MCP transport 的认证/project 前置拒绝；进入 `tools/call` 后的 MCP failure必须以
JSON-RPC tool error携带同一稳定 lineage code，不伪装成 HTTP 403/404/409。

## 10. PC runtime 与 zero-effect acceptance

PC 每次显示前必须在 runtime 逐项证明：

1. response exact 5 keys、全 string type、schema/read_effect 常量与 endpoint-specific exact kind；
2. `canonical_carrier_json` strict parse 后 RFC 8785 JCS UTF-8 byte-equal；
3. inner `lineage_digest` 置空后，重算 authority §3 的 domain+NUL+JCS SHA-256 并与 inner digest 相等；
4. inner schema/canonicalization/digest-algorithm 常量正确，inner kind/digest 与 response 的
   `lineage_kind/lineage_digest` 逐字相等，且 inner shape exact 命中对应 profile。

同一次 settlement 历史展示必须并行读取 execution 与 settlement endpoint，分别证明 exact
`execution_source_v1`/`settlement_source_v1`，再证明 settlement carrier 的
`lineage.execution_lineage_digest == execution response.lineage_digest`。任一 endpoint 缺失、kind、JCS、domain digest、
response-inner 或跨响应等式失败，整个卡片都失败关闭且不得展示部分链。
Lease、scope 或 Provider 选择变化还必须使在途 generation 失效并清空旧证据；任何旧响应写回新 subject/scope
页面都算失败。

本 adoption 不得持久 Carrier，不得 create/update/delete/CAS/replay/audit/last-seen，不得新增或修改
migration/table/view/index/trigger/UDF，也不得改写 v169-v195 owner JSON/native digest。所有读入口必须保持
`read_effect="none"`，不产生 current authority、状态、Ready、route、Offer、Job、Lease、Receipt、posting、balance、
release、withdrawal、external payment 或链上效果。

## 11. Source-written 与最终报告门

只有 authority §10 五类源码门全部具备后，才能把 Domain/Store 记为 `source_written`；Service、HTTP/MCP、PC client
每层必须独立记录 adoption，不能从 Domain 存在外推。本批各层所需源码已同时落盘，但验证强度仍严格是
`source_review_only / implementation_uncompiled / implementation_unrun`、`passed=0 / failed=0`。

任何后续报告必须分别写：

```text
design=design_frozen
domain/store/service/http_mcp/pc_client=<absent|source_written|verified>
migration/table=none
compiled=<0|1>
run=<0|1>
passed=<count> failed=<count>
native_digest_rewrites=0
state_or_money_effects=0
f0_exit_gate=<not_met|met>
```

本批唯一诚实结果是 Domain/Store/Service/HTTP-MCP/PC client=`source_written`，migration/table=`none`，
compiled/run=`0/0`，passed/failed=`0/0`，native digest rewrite=`0`，state/money effect=`0`，F0 gate=`not_met`。
