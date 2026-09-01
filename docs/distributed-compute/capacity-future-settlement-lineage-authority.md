---
title: capacity_future pricing-mode 交付结算谱系桥 V1 权威
status: current
reviewed_at: 2026-09-02
owners: backend, ai-economy
feature_id: compute-capacity-future-settlement-lineage-bridge-v1
registration_status: blocked
design_status: design_frozen
implementation_status: store_resolver_source_written_uncompiled
verification_status: source_review_only
---

# `capacity_future` pricing-mode 交付结算谱系桥 V1 权威

## 1. 唯一结论与当前现实

本已登记 V1 冻结一份 API-free、reference-only 的 source projection：把一份已经
`exercised` 的 v228 DeliveryAllocation，沿既有 v238 CapacityInstrument、v225
CapacityCommitment、v171 immutable Price Snapshot、F0 execution/verification/settlement
carrier，连接到 v195 SettlementReceipt ref；若调用方提供 v198 release carrier，则用独立封闭
分支引用。Lease-rooted Store resolver source 已能从 outer v195 event 与历史 owner bodies 重建并封存 exact
retained Attempt Settlement，但该路径仍未编译或运行。

它解决的是“这笔已验证用量结算来自哪一个未来容量合约、交付窗口、锁价快照和 whole-only
行权”的历史可解释性缺口。它不是新的价格、计量、结算或资金权威，不创建 ClearingReceipt，
不计算短缺、奖励、处罚或差额，不证明 Provider 已收款，也不把 source validator 升级成 Store
owner proof。

本批在既有 Domain、canonical/shape validator 与跨来源等式投影之上，写入 Lease-rooted Store retained
resolver、历史 CapacityInstrument/DeliveryAllocation owner reader、crate-visible、private-field Store seal 和未运行的
source-contract guard。Service、HTTP、MCP、PC、table、migration、writer 与资金动作仍不存在；状态严格为
`registered_blocked / design_frozen / store_resolver_source_written / source_review_only /
implementation_uncompiled / implementation_unrun`、`passed=0 / failed=0`。Feature Registry 已通过
`project_feature_workflow` 登记、复审和独占认领后按本批动态证据缺口收口为 `blocked`；注册表仍禁止手改。

## 2. 复用的单一 owner

| 事实 | 唯一 owner | bridge 行为 |
|---|---|---|
| Instrument、activation、Offer adoption/publication | v238 CapacityInstrument | 只保存 opaque exact ref；不重算 owner digest，不要求历史 Instrument 当前 active。 |
| Commitment 与父 Claim | v225 CapacityCommitment + Capacity ledger | 只引用 immutable revision 1 与 Claim version；不复制 meter lines 或余额。 |
| Grant、exercise、父 release、子 Reservation Claim | v228 DeliveryAllocation | 只接受 exact `granted r1 -> exercised r2` 来源；不创建第二份 Allocation。 |
| 锁价 | v171 Price Snapshot + v223 reference binding | 只钉住 ID/digest；不得用当前曲线或任务结束价替换。 |
| Provider/Pool/Offer/Job/Reservation/Lease/Execution Receipt | F0 `execution_source_v1` | 复用既有 typed refs 与 carrier digest；不定义第二套 core refs。 |
| verified/compensable verification-role digest | v192 Verification + `execution_verification_source_v1` | 复用 VerificationDecision ref；不把 declared/observed 用量替代成 verified。 |
| verified/compensable readings | v193 Execution Receipt | source projection 对齐同一 ExecutionReceiptRef；不在 carrier 复制 readings。 |
| settlement-role usage digest、金额与 pending origin | v195 Settlement + `settlement_source_v1` | 单独保存 v195 两个 usage digest；不与 v192 digest 判相等，不重新计算金额。 |
| internal available release | v198 + `settlement_release_source_v1` | 仅 available 分支引用；不代表提现或外部付款。 |

F0 现有四种 profile、canonical bytes 与 digest domain 保持逐字不变。该 bridge 使用独立 schema/domain，
因此不会让按 Lease root 的 F0 endpoint 意外返回容量市场对象，也不会让容量聚合语义污染通用历史 carrier。

## 3. Envelope 与 canonical ABI

顶层固定 exact 6 keys：

```text
schema = compute_federation.capacity_future_settlement_lineage_bridge.v1
lineage_kind = capacity_future_settlement_bridge_v1
lineage_digest = lowercase sha256
canonicalization = rfc8785_jcs
digest_algorithm = sha256
lineage = exact object
```

摘要固定为：

```text
sha256(
  "ELON-COMPUTE-CAPACITY-FUTURE-SETTLEMENT-LINEAGE-BRIDGE-V1"
  || 0x00
  || RFC8785_JCS(envelope with lineage_digest="")
)
```

输入最多 262144 bytes；所有对象 `deny_unknown_fields`，revision/version 使用
`1..=2^53-1`，opaque digest 只接受 64 位小写十六进制。parse 后必须通过完整 shape/self-digest
校验，并与 canonical JSON 逐字相等。ID 不 trim、不 normalize、不改大小写；native owner digest
不能由 bridge 统一重算或被 bridge digest 替代。

## 4. Lineage exact shape

`lineage` 固定为以下职责字段：

```text
pricing_mode = capacity_future
settlement_currency = CNY
price_snapshot
reference_price_binding
delivery_window
capacity_instrument
instrument_activation
instrument_offer_adoption { adoption receipt + exact Offer + publication }
capacity_commitment { immutable root + parent Claim ref }
delivery_allocation_grant { immutable Grant + quoted Job ref }
delivery_allocation_exercise { exercised terminal + parent released Claim + child Claim + exercise Reservation + reserved Job }
terminal_reservation
execution_source_lineage_digest
execution_receipt
execution_verification_lineage_digest
verification_decision
settlement_usage_digests
economic_lineage
effects
```

Provider、Pool、Offer、Job、Reservation、Claim、Execution Receipt、VerificationDecision、
Attempt Settlement 与 Settlement Release 继续复用 F0 typed refs。`verification_decision` 保存 v192
verification-role digests，`settlement_usage_digests` 另存 v195 settlement-role digests；二者使用不同 owner
摘要域，绝不要求字节相等。bridge 不复制 SKU、price components、meter arrays、金额、Attempt Lease、actor、
时间、current status 或 mutable head。`delivery_window` 只保存
Instrument/Commitment/Snapshot 已共同锁定的 exact ID/digest 与 UTC nanosecond 半开区间。

固定 effect 为：

```text
reference_effect = retained_references_only
capacity_effect = none
verification_effect = none
settlement_effect = none
money_effect = none
withdrawal_effect = none
```

## 5. 封闭 economic lineage

`economic_lineage` 是带 `economic_stage` 标签的封闭 union，禁止 `Option`、`null` 或调用方自定义状态：

- `pending_settlement_source_v1`：只携带 exact-shape AttemptSettlementRef 与
  `settlement_source_v1` digest；当前 `Projected...` 只能表达 pending-origin ref，未来 Store-sealed view
  才能证明 v195 创建时的 immutable pending origin；
- `available_release_source_v1`：除上述两项外，必须携带 exact SettlementReleaseRef 与
  `settlement_release_source_v1` digest；当前只表达 release ref，未来 Store-sealed view 才能证明 v198
  曾把内部 pending 转为 available。

不得从 pending 分支缺少 release 推导当前仍为 pending、未被挑战或尚未释放。即使未来由 Store seal，
available 分支也只证明平台内部 release 历史，不证明余额仍未提现，不证明 v200/v201 withdrawal，更不证明
`external_paid_attested` 所述外部付款真实发生。

## 6. 本批源码已表达的跨来源等式

API-free source projection 对调用方提供的对象失败关闭检查：

1. Instrument、activation、adoption 的 ID/revision/digest 完全一致，activation 不晚于 adoption；
2. Commitment 固定 `capacity_future` 来源、revision 1 `committed` root，Instrument、窗口、Offer、
   Snapshot 与 Provider 精确绑定；
3. Grant 固定 revision 1 `granted`，Provider owner、exercise cutoff 与 Commitment 一致；terminal 固定
   revision 2 `exercised`，actor 必须是 exact consumer，且发生在 cutoff 前；
4. Commitment/父 prior Claim 固定 r1，exercise parent result 固定 r2 `released`，子 Reservation Claim
   固定 r1，Broker active Reservation 固定 r2，reserved Job 固定为 quoted Job+1；两条 ledger event/causal
   transaction 也必须按 parent release→child hold 相连；
5. 传入的自摘要 execution carrier 之 Provider/Pool/Offer/Snapshot 与 Commitment 一致，Job、Reservation、
   child Claim ID 一致，且 running Job、active Reservation、active Claim 分别严格为 exercise refs+1；另要求
   v193 inner receipt 的 ID/digest、Job、Reservation、Lease、Provider、Offer 与该 carrier 相等且
   verification status 为 accepted，并且 v193 `decision_digest` 精确等于 v192
   `verification_event_digest`；
6. verification carrier 的 ExecutionReceiptRef 与 execution carrier 相等，且
   `execution_lineage_digest` 等于传入 execution carrier digest；
7. settlement carrier 的 execution digest/ref、Snapshot、Provider、Job、Reservation 与同一 execution
   chain 一致，并固定 verification-pending Job=running Job+1、settled Job=前者+1、terminal Reservation=
   active Reservation+1；
8. v195 inner receipt 的 ExecutionReceipt、Snapshot、Reservation 与 settlement carrier 一致，consumer 与
   Allocation consumer 一致；v192
   VerificationDecision 的两个 digest 和 v195 inner receipt 的两个 digest 分别保留为 role-specific refs，
   不判相等，也不在 Domain 复制两套 owner digest 算法；v195 payee 不直接等同 Provider owner，Store
   historical audit 按 exact Provider `settlement_account_id`、缺省回退 owner 的既有规则证明；
9. Domain-owned untrusted v195 audit view 还必须对齐 event digest、Lease、Finalization、Provider、source/terminal
   Job，以及 v228 budget ID/amount；它表达 outer 字段等式但不证明 view 确由 Store owner body 构造；
10. available 分支的 release carrier 必须引用同一 AttemptSettlementRef 和 settlement carrier digest。

这些等式只说明调用方传入的 DTO/carrier projection 内部一致。Domain builder 返回类型刻意命名为
`Projected...`，不是 Store-sealed authority；parser 也无法独立证明任一 native owner digest、outer v195
event、历史行存在性或调用者权限。本批另写的 Store retained resolver source 才在同一 Deferred read
transaction 中重建并审计 owner bodies，再封装为字段与构造器私有、crate-visible 的 `Validated...`；该路径仍未编译或运行。

## 7. Lease-rooted Store resolver V1

唯一 Store facade 固定为
`resolve_compute_capacity_future_settlement_lineage_for_lease(lease_id)`。它只接收 settlement Lease/root，
在一个 `BEGIN DEFERRED` read snapshot 内自行解析全部 owner ID；调用方不能提交 Instrument、Commitment、
Allocation、Snapshot、Receipt、Carrier、digest 或 access scope。返回值是 private-field、non-Clone、non-Serde
的 `ValidatedComputeCapacityFutureSettlementLineageV1`，只暴露 canonical JSON、bridge digest 以及 Store-sealed
participant/project 判定；`Projected...`、parser 结果和任意 caller DTO 都不能构造该类型。

profile 与错误语义固定为：

- settlement Lease 不存在、Snapshot 不是逐字 `capacity_future`，或该 Reservation 确实没有 exercised v228
  owner 时返回 `None`，表示本 profile 不适用；普通 future Broker Reservation 不得被误收；
- 一旦该 Reservation 存在 exercised v228 owner，owner cardinality 不为一或 Claim index 不同即为 integrity
  failure；不得把 cross-splice 降格为 `None`。其后任一 Instrument/activation/adoption/publication、Commitment/Claim、F0 carrier、
  v195 outer owner 缺失、错接、非 canonical 或摘要漂移均为 integrity failure；若存在 v198 owner，
  则它的缺失依赖、错接、非 canonical 或摘要漂移同样属于 integrity failure；
- connection、begin/commit、busy、I/O 与 SQL/schema operational failure 单独分类；未来 participant surface
  必须把 missing/integrity/nonparticipant 合并脱敏，admin 才能区分 not-found、integrity 与 operational；
- release 缺席只选择 `pending_settlement_source_v1` 的历史 origin 分支，绝不推导当前仍 pending；release
  存在时必须重建同一 settlement 的 exact v198 owner 和 F0 release carrier。

resolver source 必须证明：

- v238 Instrument registration/activation/adoption/publication 均为 exact historical owner facts；Instrument 后来
  retired 不得否定合法历史，不能复用仍要求 current active 的 fresh-admission helper；
- v223 reference binding 与 Price Snapshot 的审批/application 历史完整，不能读取 mutable latest curve；
- Commitment Claim lines 与 Instrument contract units 是同一个正整数 multiplier，meter 集合、顺序、粒度和
  quantity 全部一致；父 Claim whole release 与子 Reservation Claim whole hold 守恒；
- v192 decision 是 accepted，v193 `decision_digest` 精确绑定该 v192 event；v193 verified/compensable arrays
  是共同 readings owner；必须分别按 v192
  `compute_attempt_verification_usage` 公式与 v195 `settlement_usage_digest.v1` 公式重算并核对各自 digest，
  禁止直接比较两套 digest；v195 payee 由 historical Provider settlement-account fallback 审计，outer event、
  owner audit 和 posting 仍由现有 Store 完成；
- historical Offer/publication 读取不能因后来 bucket head、Offer draining 或 Instrument retirement 误拒；
- participant/admin scope、missing/integrity/nonparticipant 脱敏与 project isolation 复用 F0 retained read 边界。

当前 source 通过既有 historical owner audit 分别重建 v192 与 v195：v192 owner audit 按
`compute_attempt_verification_usage` 复算 verification-role digests；historical v193/v195 owner audit 从同一
verified/compensable readings 按 `settlement_usage_digest.v1` 复算 settlement-role digests。两套摘要仍不得
直接比较。v228 historical reservation authority 重审 whole-only parent release→child hold；resolver 另复用
v225 的共同整数 multiplier 校验，并补验 meter 顺序与 `unit_size == quantum_units`。这些源码尚未编译或运行，
因此仍不能宣称可调用，也不能成为任何 admission、dispatch、verification、settlement 或 release gate。

## 8. 明确 NO-GO

- 不新增 Order、Trade、Position、ClearingReceipt、指数、mark、真实 price source 或自动撮合；
- 不新增短缺、买方未消费、替代容量、奖励、处罚、重试净额化或 delivery penalty 公式；
- 不修改 v195 SettlementReceipt、v192 Verification、F0 四 profile 或它们的 native/canonical digest；
- 不把所有 `capacity_future` Reservation 视为 DeliveryAllocation；普通 Broker Reserve 不属于本 profile；
- 不使用缺少 receipt/ref 的空分支表达 ProviderShortfall、BuyerUnused、未执行或未结算；缺席事实需要
  带 cutoff 的独立 Store authority；
- 不接入 `capacity_forward`、spot 或 index_locked；本 V1 只接受逐字 `capacity_future`；
- 不触碰 V279 user_node Ready/route/Offer/Attempt 边界，也不打开 V280 external-pool #13-#18 deny 或
  `eligible_rows=0`；
- 不新增 table、migration、writer、Service、HTTP/MCP、PC、网络、运行时或资金动作。

## 9. 冻结状态

正式需求见 [`compute-capacity-future-settlement-lineage-bridge-v1.md`](../requirements/compute-capacity-future-settlement-lineage-bridge-v1.md)。源码文件只建立已登记的独立 Domain ABI、JCS/SHA-256、shape validator、source projection equations、
Lease-rooted Store retained resolver 与 crate-visible、private-field Store seal。验收证据以
[`capacity-future-settlement-lineage-acceptance.md`](capacity-future-settlement-lineage-acceptance.md)
为准。Feature Registry 当前为 `blocked`；在没有编译或运行证据时，只能称“已登记的 `capacity_future`
交付结算谱系 bridge 与 Store resolver source 已写入并静态复核”，不能称容量期货清算、验证计量或资金结算已生产闭环。
