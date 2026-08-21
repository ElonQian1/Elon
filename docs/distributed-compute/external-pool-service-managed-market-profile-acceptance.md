---
title: 外部矿池 service-managed market profile canonical ABI 验收
reviewed_at: 2026-08-21
status: current
owners: backend, security, ai-economy
design_status: design_frozen
design_scope: market_profile_schema_and_canonical_abi_v1
implementation_status: implementation_unwired
verification_status: design_review_only
---

# 外部矿池 service-managed market profile canonical ABI 验收

## 1. 当前验收结论

本页只验收 [market profile authority](external-pool-service-managed-market-profile-authority.md) 的 schema/canonical ABI。
完整 V280 纵切见 [父权威](external-pool-service-managed-admission-runner-authority.md) 与
[父验收](external-pool-service-managed-admission-runner-acceptance.md)。
[Projection identity ABI](external-pool-service-managed-market-projection-identity-abi-authority.md)另行冻结Pool/Offer/Snapshot
identity、legacy owner helper与checked-at映射。

```text
market_profile_schema_abi=design_frozen
market_profile_inventory_approval_evidence_abi=design_frozen
initial_profile_approval_evidence=unselected
initial_profile_inventory=unselected
current_profile_authority=unconstructible
external_adapter_semantic_wire_profile_registry_abi=design_frozen
external_adapter_semantic_wire_profile_approval_evidence_abi=design_frozen
initial_external_adapter_semantic_wire_profile_approval_evidence_set=unselected
initial_external_adapter_semantic_wire_profile_inventory=unselected
external_adapter_semantic_wire_profile=unselected
implementation=unwired/uncompiled/unrun
passed=0
failed=0
```

本批没有 enabled profile JSON/digest，不产生正向 market authority，也不证明任何 Pool/Offer/Snapshot/Plan/Start/Lease/
Runner 可达。

## 2. 正式来源与 payload gate

| 项 | 当前可验收事实 |
|---|---|
| capacity | scope/unit/单bucket结构与Claim exact 1已冻结；真实总量未选择。 |
| execution ceiling | 十个字段及正数/safe-integer关系已冻结；九个数值与network bool未选择。 |
| price | spot/CNY/fallback/half_up/空fee与结构关系已冻结；价格、max amount、TTL、curve/source identity未选择。 |
| workload/runtime | artifactless/no model/no checkpoint与Offer projection边界已冻结；task kind、runtime、accelerator、authorization、output与重试策略未选择。 |
| transport | 524288/262144/65536/262144/262144/256/64/15000逐字段取正式production常量。 |
| lease issuer | schema/domain、kind/mode/scope/ref-hint派生冻结；正式owner/review approval未选择。 |
| catalog | planned typed compiled inventory与0/1 current选择合同已冻结；当前逻辑empty且无源码，不能构造authority。 |

以下来源必须负向拒绝：test/support/fixture/seed、Provider/Offer raw declaration、V270 readiness、user-node manifest、
历史 receipt、route credential、caller bool/raw JSON，以及“0/1作为安全默认”。

## 3. Canonical matrix

| Case | 必须结果 |
|---|---|
| valid envelope | exact 7 top-level keys、17 profile keys、deny unknown、I-JSON与RFC8785-JCS byte equality。 |
| profile ID | 只由revision projection与ID domain派生；caller ID、错误prefix、wrong revision拒绝。 |
| profile digest | digest key保留为空串后domain-separated SHA-256；删除key、raw JSON SHA、uppercase拒绝。 |
| review material | 完整7/17-key投影同时保留并清空`profile_digest`与`review_source.source_digest`，审批evidence绑定material digest后才填source digest并计算final；循环绑定final digest、删blank key或三类digest互换拒绝。 |
| nested objects | key set、nullable、array排序/唯一与safe integer逐字匹配；extra/missing/null substitution拒绝。 |
| review source | exact source tuple包含真实`approved_by_user_id`并覆盖profile ID/revision/review-material/approved-at；final digest传递绑定source digest，service actor、Provider owner或caller猜值拒绝。 |
| times | UTC nanos且`approved<=valid<=checked<new-plan<=expires<inflight`；边界倒置拒绝。 |
| capacity | exact一个attempt_slot/reusable/quantum1 bucket；issued/allocation/concurrency相等，0或multi拒绝。 |
| region | `sku.region_or_data_zone`同时满足profile byte规则与legacy Pool trim/nonempty/最多80 Unicode scalar；81字符拒绝。 |
| price | exact一个attempt_slot component且max_units=1；provider<=consumer、单Job max amount等于单价；half_even拒绝。 |
| Offer curve | `curve_id`逐字投影且`curve_version=Some(curve_revision)`；None、revision/version漂移或与snapshot source tuple混用拒绝。 |
| transport | outer/upstream request分离且所有常量exact；任一放大、缩小或塞进ceiling拒绝。 |
| issuer policy | self-digest、fixed kind/mode/audience/sorted singleton scope、ref/hint exact material、96/97-byte fixed shape与512/160-byte上限均exact。 |
| allocation | typed Provider/V249 pair+profile+scope/unit/total exact material；跨Provider同ID、raw pair或global余额解释拒绝。 |
| readback | parse→deep validate→重算→canonical bytes逐字相等；仅serde parse成功不足。 |

## 4. Catalog/currentness matrix

| Inventory | Expected |
|---|---|
| 当前逻辑空inventory（尚无源码） | `current_*_authority` 不存在/不可构造，0 durable write、0 market effect。 |
| 0 enabled | 失败关闭。 |
| 2 enabled | 失败关闭。 |
| 1 enabled但未到valid-from/已过new-plan/expired/revoked | 失败关闭。 |
| 1 enabled但Provider非external_pool/非active或V249 pair不匹配 | 失败关闭。 |
| 1 enabled但Provider capability不含task/accelerator/region/data-class | 失败关闭，不能等Offer writer再发现。 |
| historical exact item | 只准pure audit，不恢复current authority。 |
| successor | V1拒绝；不得静默替换相同revision/digest或删除历史item。 |

第一项 enabled inventory 只有在 exact JSON/digest、按
[approval evidence ABI](external-pool-service-managed-market-profile-approval-evidence-abi-authority.md)产生的真实四眼证据和完整纵切实现同批后
才能加入；仅提交一个 profile file、
只实现validator或只把empty改成nonempty都不得晋级。

## 5. Consumer mapping 与 identity ABI

| Consumer | Frozen structural mapping |
|---|---|
| Pool/ledger | allocation total→单attempt_slot bucket issued/available/reservable；每Claim=1。 |
| Offer/SKU | profile SKU/runtime/resource/authorization/price exact投影；Pool resource/SKU digest都沿existing owner legacy helper，model/plugin为空，runner/observed roots另取fresh typed source。 |
| v171 snapshot | quoted-at=checked-at，expiry取TTL/new-plan/profile最小值，source=fallback且观察窗`[checked-at-1s,checked-at]`。 |
| Tx-A capability | 十项ceiling取historical profile；V274/V277/V278 provenance与executor另做fresh重证。 |
| Tx-B lease | issuer root由Plan snapshot→admission→historical profile回溯；ref/hint按Attempt material确定性派生。 |
| replay | profile/snapshot new-plan expiry不回溯撤销已seal Plan；Plan仍受自身hard/inflight截止。 |

Profile 本身直接写 Pool/Offer/Plan、把 V277 identity写死进profile、把price caller DTO当authority，均为失败。
`resource_scope`、Pool/bucket/meter/delivery-window、Offer/publication、v171 snapshot/quote/source的deterministic identity、
legacy digest与单一时钟已由projection identity ABI冻结设计，但仍无writer源码。Tx-B `fence_digest`不属于market projection；
它由Gateway/session/validator内部ABI冻结为Plan+seal派生值，fixture domain不得升格，external semantic wire profile仍未选择。

## 6. Source-written 前的静态门

后续源码批必须逐项锁定：

- owner module、planned symbols、private fields、deny-unknown DTO与non-Clone/non-Serde authority；
- canonical constants/domain、empty-digest projection与pure historical validator；
- compiled inventory exact-one selection、revocation set与typed Provider/V249 constructor；
- checked-in first profile review-material digest、purpose-specific approval evidence及填入source digest后的final JSON/digest；
- source contract 反向禁止 raw profile/capacity/price/ceiling/bool；
- legacy Pool resource-profile、SKU、v171 snapshot digest继续由各owner既有serde/helper验证，禁止改用本页JCS domain；
- 按已冻结projection identity ABI实现resource scope、Pool/bucket/meter/window、Offer/publication、snapshot/quote/source的
  deterministic material、owner helper、single clock、readback与replay；
- deadline consumer逐字段冻结Job retry、reconcile/event scheduler、pre-start cleanup/60秒lease margin与task-session effective
  deadline，不得把Job retry映射为ELTP exchange ordinal；
- 按已冻结的 [admission receipt physical ABI](external-pool-service-managed-admission-receipt-abi-authority.md)与
  [Gateway/session/validator internal ABI](external-pool-service-managed-gateway-session-validator-abi-authority.md)实现，落实fixed
  source observation window，选择external semantic wire profile，并同批实现完整V280 writer/Gateway/validator/Runner。

未满足任一项时，V280 继续 `implementation_unwired/uncompiled/unrun`，migration最高保持V279，V254 fence保持关闭，
worker `eligible_rows=0`。静态文档检查不计入实现 `passed/failed`。

## 7. 本批静态验收

本批只允许：文档链接、frontmatter状态、文档尺寸、canonical单一真源、server零diff、任何物理migration/UDF/table/worker零注册且不预占280
等静态审查。禁止编译、测试、执行migration/SQLite/runtime/network；正式计数保持 `passed=0/failed=0`。

最终报告必须分别写：

```text
vertical_slice_architecture=design_frozen
market_profile_schema_abi=design_frozen
market_profile_inventory_approval_evidence_abi=design_frozen
initial_profile_approval_evidence=unselected
initial_profile_inventory=unselected
admission_receipt_physical_schema_abi=design_frozen
market_projection_identity_abi=design_frozen
gateway_builder/fence/task_session/validator_internal_abi=design_frozen
external_adapter_semantic_wire_profile_registry_abi=design_frozen
external_adapter_semantic_wire_profile_approval_evidence_abi=design_frozen
initial_external_adapter_semantic_wire_profile_approval_evidence_set=unselected
initial_external_adapter_semantic_wire_profile_inventory=unselected
external_adapter_semantic_wire_profile=unselected
implementation=unwired/uncompiled/unrun
```

不得缩写为“V280 fully frozen”或“bootstrap profile ready”。
