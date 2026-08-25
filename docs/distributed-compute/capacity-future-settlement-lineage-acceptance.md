---
title: capacity_future pricing-mode 交付结算谱系桥 V1 验收草案
status: draft
reviewed_at: 2026-08-25
owners: backend, ai-economy
proposed_feature_id: compute-capacity-future-settlement-lineage-bridge-v1
registration_status: unregistered_feature_workflow_unavailable
design_status: draft_frozen
implementation_status: source_draft_uncompiled
verification_status: source_review_only
---

# `capacity_future` pricing-mode 交付结算谱系桥 V1 验收草案

## 1. 当前证据强度

唯一规范是
[`capacity-future-settlement-lineage-authority.md`](capacity-future-settlement-lineage-authority.md)。
当前只存在 API-free Domain、canonical/shape validator、跨来源等式投影与未运行的 source-contract guard：

```text
proposed_feature=compute-capacity-future-settlement-lineage-bridge-v1
registry=unregistered
claim=none
design=draft_frozen
domain_implementation=source_draft_written
source_equations=source_draft_written
source_contract_guard=source_written_not_run
verification=source_review_only
compile_status=implementation_uncompiled
runtime_status=implementation_unrun
compiled=0
run=0
passed=0
failed=0
migration/table/writer=none/none/none
store/service/http/mcp/pc=none/none/none/none/none
state/capacity/money/withdrawal_effects=0/0/0/0
acceptance=deferred
```

本批没有编译、测试、执行 migration、SQLite、runtime、network、HTTP、MCP 或 PC；旧 v171/v192/v195/
v198/v225/v228/v238/F0 的任何通过计数都不能冒充本 bridge 的 positive evidence。当前工具目录不含
`project_feature_workflow`，因此没有登记或 claim 证据，且注册表保持未修改。

## 2. Canonical 与 shape matrix

| Case | 必须结果 |
|---|---|
| envelope | exact 6 keys；schema/kind/JCS/SHA-256/domain 逐字固定。 |
| self digest | `lineage_digest=""` 后 domain+NUL+JCS；普通 serde SHA、native digest 或删除字段拒绝。 |
| bytes | UTF-8 I-JSON、parse→JCS byte-exact、最大 262144 bytes；unknown/missing/extra/trailing 拒绝。 |
| refs | ID 非空且无首尾空白/控制字符；opaque digest 只接受 64 位小写 SHA-256。 |
| numbers | revision/version 为 `1..=2^53-1`；0、负数、float、字符串数字和超界拒绝。 |
| window | exact UTC nanoseconds，`starts_at_utc < ends_at_utc`。 |
| fixed values | pricing mode=`capacity_future`、currency=`CNY`、reference effect固定，其余五 effect=`none`。 |
| trust | parse/shape/self-digest 通过仍是 untrusted carrier；不得授予 Store、执行或经济权限。 |

## 3. Source-equation matrix

| Case | 必须结果 |
|---|---|
| Instrument | registration/activation/adoption identity一致，activation不晚于adoption。 |
| Commitment | revision 1 committed root；Instrument、Offer、Snapshot、Provider、window exact。 |
| Allocation | Grant r1 granted、owner/cutoff exact；terminal r2 exercised、consumer actor exact；Claim 1→2/child r1、Reservation r2、reserved Job exact ref=quoted+1。 |
| execution | supplied carrier 的 Provider/Pool/Offer/Snapshot 对上容量来源；running Job、active Reservation/Claim 分别严格+1；v193 inner receipt identity/Offer/Lease 同源且 accepted。 |
| verification | exact ExecutionReceiptRef；verification的execution digest等于 supplied execution carrier。 |
| usage | v192 verification-role 与 v195 settlement-role digest 分字段保存，禁止判相等；两套公式到同一 v193 readings 的 owner proof 延后 Store。 |
| settlement | v195 inner receipt、carrier、ExecutionReceipt、Snapshot、Reservation 同源且 Job/Reservation revision 严格递进；untrusted outer audit view 对齐 event/finalization/budget/provider/job，但 owner 构造尚未证明。 |
| release | available 分支必须有 exact v198 carrier，并引用同一 settlement ref/digest。 |
| projection | 已表达的 source ref、exact exercise revision 或 digest 漂移必须拒绝；后续版本的完整 lifecycle drift 由未来 Store owner resolver 拒绝。 |

上述是源码应满足的 matrix，不是已运行结果。当前 builder 返回 `Projected...` 而不是 Store-sealed validated
authority。未来动态测试必须从 Store retained owner bodies 构造 positive
fixture；直接构造 serde 对象、任意 64-hex、seed SQL 或复制文档示例不能充当 owner proof。

## 4. Economic-stage matrix

| 分支 | 可宣称 | 禁止宣称 |
|---|---|---|
| `pending_settlement_source_v1` | 当前 Projected 只携带 v195 pending-origin ref；未来 Store seal 后才可证明该历史存在。 | 当前仍 pending、没有 challenge/release、金额可提现。 |
| `available_release_source_v1` | 当前 Projected 只携带 v198 release ref；未来 Store seal 后才可证明 internal available release 历史。 | 当前余额未提现、v200/v201 完成、外部付款真实发生。 |

serialized Domain 不得出现 `Option`/`null` release。pending 与 available 的字段集合必须由 closed tagged union
固定；unknown stage 或跨分支字段拒绝。ProviderShortfall、BuyerUnused、未执行、未结算或 disputed 终局都不在
本 V1，不能用空 ref 或 reason string 伪造。

## 5. Future Store acceptance gates

本 bridge 只有同时通过以下后续证据才能从 `source_review_only` 升级：

1. 同一 Deferred read transaction 从单一 Lease/settlement root 重建 v238→v225→v228→F0→v195，调用方
   不能提交任一 owner ref；
2. historical Instrument/Offer/publication 在后来 retired/draining 后仍可审计，fresh-admission currentness 不被
   错用为历史读取门；
3. Claim lines 与 Instrument contract units 的共同 multiplier、meter exact set、父 release→子 hold 守恒通过
   Store/SQLite positive 与 negative；
4. v192 accepted 与 v195 settlement digest 必须分别从同一 v193 verified/compensable arrays 按各自 owner
   公式重算；两套 role-specific digest 不得直接判相等，declared/observed 替代必须失败；
5. pending/available 两分支 golden、negative、corruption、cross-splice、reopen 与 participant scope 测试；
6. Service/HTTP/MCP 若后续采用，必须保持 F0 脱敏、project isolation、响应 exact shape 和零业务写入；
7. 完整 Rust target 编译、fresh/repeat migration（预期仍无新 migration）、定向 Store 测试与真实历史文件
   重开留下独立指纹和通过计数。

## 6. 当前交付矩阵

| 轴 | 状态 |
|---|---|
| implementation | `unregistered_source_draft_written`（仅 Domain/source equations） |
| source review | `done_with_red_team_corrections`；不等于编译或测试。 |
| compile/test/runtime | `not_run/not_run/not_run`；仅允许本批结束时记录 formatter 与静态源码守卫，不计为测试通过。 |
| delivery | 提交推送前为 `not_delivered`；推送后才可改报 `pushed`。 |
| acceptance | `deferred`；Store/API/runtime/product acceptance 均未开始。 |

因此，本批完成后也只能宣称“unregistered reference-only bridge ABI 草案与来源等式源码已写入”。不能宣称
feature 已登记/认领、F0 exit gate 已满足、`capacity_future` 市场可用、交付清算完成、Provider 收益可提取或
真实资金已结算。
