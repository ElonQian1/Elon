---
title: 联邦核心历史因果引用 Carrier ABI 验收
reviewed_at: 2026-08-21
status: current
owners: backend, node, ai-economy
design_status: design_frozen
design_scope: federation_core_historical_causal_reference_carrier_abi_v1
implementation_status: implementation_unwired
verification_status: design_review_only
---

# 联邦核心历史因果引用 Carrier ABI 验收

## 1. 当前证据与状态

唯一规范是 [Carrier authority](federation-historical-causal-reference-abi-authority.md)。本页只验设计；当前没有
Domain/Store/Service/HTTP/MCP/client source、table、migration、caller、compile或runtime证据：

```text
federation_core_historical_causal_reference_carrier_abi=design_frozen
carrier_profiles=execution_source_v1/settlement_source_v1
implementation=unwired/uncompiled/unrun
passed=0 failed=0
```

F0 的 Store、Service、HTTP/MCP、客户端采用与合同测试退出门仍未达到。不得把本文 matrix写成已运行结果。

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

Future golden须为两种 profile各至少固定一份完整 bytes/digest，并证明同一 native refs在kind、role或 source/terminal
位置变化时得到不同 Carrier digest。

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

`execution_source_v1.lineage` 必须 exact 9 object keys且无null。Future Store tests至少覆盖：

- Provider↔Offer metadata、Pool↔Offer/Claim、Offer↔Snapshot/Job/Reservation逐字相等；
- Snapshot↔Job/Reservation、Job↔Reservation、Reservation↔Claim逐字相等；
- v189/v192 的 Job、Reservation、Claim、source Lease和fencing逐字相等，v193 owner audit解析回同一组 refs；
- Execution Receipt 的 Job/Reservation/Lease/attempt/fencing/Provider/executor/Offer回指同链；
- v185、v188-v193 native owner audit全部通过后才产生 sealed validated view。

必须拒绝 cross-Provider、cross-Offer、cross-Snapshot、cross-Job、cross-Reservation、cross-Claim splice；同 Lease ID换
revision/digest/fencing、用读取时current Lease、用 v194 terminal Lease、只解析 receipt JSON猜父链也必须拒绝。

Plan/route/capability/budget/usage/evidence/artifact缺省不是 null key；它们根本不属于 V1 shape，仍由各 owner证明。

## 5. Settlement source matrix

`settlement_source_v1.lineage` 必须 exact 9 object/string keys。Future Store tests至少覆盖：

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

Future source contract必须证明 raw HTTP/DTO/caller不能构造 validated view；同一 source重复读取只得到同一 JSON/digest，
且不产生row、idempotency replay、状态或资金效果。

## 7. Source-written 与最终报告门

只有 authority §9 五类门全部具备后，才能把 `domain_implementation/store_resolver`改为 source-written；Service、
HTTP/MCP、客户端每层必须独立记录 adoption，不能从 Domain存在外推。

任何后续报告必须分别写：

```text
design=design_frozen
domain/store/service/http_mcp/client=<absent|source_written|verified>
migration/table=none
compiled=<0|1>
run=<0|1>
passed=<count> failed=<count>
native_digest_rewrites=0
state_or_money_effects=0
f0_exit_gate=<not_met|met>
```

本批预期且唯一诚实结果是所有实现层 absent、compiled/run=`0/0`、passed/failed=`0/0`、F0 gate=`not_met`。
