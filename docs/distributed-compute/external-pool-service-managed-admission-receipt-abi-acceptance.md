---
title: 外部矿池 service-managed admission receipt canonical 与 physical ABI 验收
reviewed_at: 2026-08-21
status: current
owners: backend, security, ai-economy
design_status: design_frozen
design_scope: admission_receipt_canonical_and_physical_schema_abi_v1
implementation_status: implementation_unwired
verification_status: design_review_only
---

# 外部矿池 service-managed admission receipt canonical 与 physical ABI 验收

## 1. 当前证据边界

本页只验收 [admission receipt ABI authority](external-pool-service-managed-admission-receipt-abi-authority.md)。完整 V280
纵切见 [父权威](external-pool-service-managed-admission-runner-authority.md)，Profile 见
[market profile authority](external-pool-service-managed-market-profile-authority.md)。当前必须报告：

```text
canonical_abi=design_frozen
physical_schema_abi=design_frozen
market_projection_identity_abi=unfrozen
table/migration/source=absent
physical_migration_registration=absent
migration_registry_max=279
implementation=unwired/uncompiled/unrun
passed=0
failed=0
```

文档静态检查不算 implementation evidence；V273/V274/V277/V278 历史通过数也不能计入本页。

## 2. Canonical matrix

| Case | 必须结果 |
|---|---|
| envelope | exact 6 top keys、7 admission groups、72 direct group keys（bucket array计1）及其唯一25-key元素；extra/missing/null substitution拒绝。 |
| ID | sequence1+Provider ID经ID domain派生固定prefix；caller ID、wrong prefix或把V249/current/time放进ID拒绝。 |
| request | exact 10-key stable material；current roots、profile、market IDs/time加入或遗漏固定actor/scope/key均拒绝。 |
| integrity | exact admission object经integrity domain；raw SHA、重复material字段或错误domain拒绝。 |
| receipt | self digest key保留为空串后receipt domain；删除key、uppercase、非JCS bytes拒绝。 |
| metadata | schema/JCS/SHA-256与4 MiB边界exact；最大1 MiB profile经JSON string转义后仍可容纳；unknown、duplicate、float、unsafe integer、非UTC-nanos拒绝。 |
| profile | canonical中是完整profile JSON原文的escaped string、物理列是decoded raw canonical TEXT，两者byte-equal后由profile pure validator重算；object/双重编码/legacy digest改域均拒绝。 |
| event | exact `pool_activation_event_id + request_digest`；不得伪造不存在的event digest。 |

## 3. Bucket、time 与 idempotency matrix

| Case | 必须结果 |
|---|---|
| inventory | exact length-1 array且唯一元素为25-key bucket object；count=1，array digest可重算；裸object、canonical字段漂移、immutable binding/config漂移或sequence1 supply+2 legs无法重建genesis均拒绝，合法mutable head漂移允许。 |
| genesis balance | reusable attempt_slot、quantum1、issued=available=allocation、其余0、revision/ledger sequence=1。 |
| window | starts=checked-at、ends=profile inflight；bucket/receipt/source snapshot任一时间不一致拒绝。 |
| receipt time | valid-from=created-at=checked-at，expires=snapshot expiry且checked<expires<Offer valid-until。 |
| replay | Provider/receipt/(scope,key)三查均先于current read；0/1必须同row，exact replay 0 write。 |
| conflict | split identity、同Provider第二request、same key不同material、sequence2/predecessor非NULL全部拒绝。 |

## 4. Physical schema matrix

Authority 的 77 列是唯一顺序真源；验收不得复制另一份可漂移列清单。

| Case | 必须结果 |
|---|---|
| table | planned exact一张 `WITHOUT ROWID` immutable table；0 head/view/revocation/session/Secret/payload表。 |
| projection | Domain getters、DDL、INSERT params、SELECT columns、row index 0..76与canonical scalar逐字相同。 |
| types/checks | TEXT/INTEGER/NULL、safe revision、lowercase digest、JSON size、fixed constants、genesis/time关系exact。 |
| parent keys | 10组required exact parent keys中9组新增，publication pair复用既有exact index；重复建publication、两个独立FK或只FK ID均拒绝。 |
| FK | V277 triple、V274 pair、V278 pair及Provider/V249/route/Pool/ledger/event/Offer/publication/snapshot exact key全覆盖。 |
| unique | Provider、binding、allocation、Pool、supply、event、Offer、publication、snapshot/quote不得被两份admission复用。 |
| immutable | UPDATE/DELETE/REPLACE/backfill/seed/fixture/direct SQL拒绝，失败不留partial row或permit。 |
| index | expires/provider仅为selector；不得把索引或view当current authority。 |

## 5. UDF、trigger 与 readback matrix

| Case | 必须结果 |
|---|---|
| canonical UDF | arity1、deterministic+innocuous、只重算receipt内可派生digest；外部pair由owner join重证；每connection注册，migration-only注册、missing或false均失败关闭。 |
| pending UDF | variable-arity、non-deterministic、connection-local、ordered、one-shot、RAII；不能与canonical UDF合并。 |
| source trigger | no-replace/exact-source均自带identity拒绝且不依赖同类trigger顺序；plan→canonical→77列投影→parent joins→bucket→fresh time/currentness全部exact。 |
| fresh readback | parse→deep validate→本页派生digest重算→JCS byte equality→77列逐项→owner external-pair audit→server read-at currentness→fully-consumed→commit。 |
| replay/historical | 0 current read；immutable source与supply+legs重建初始bucket，禁止比较已变化的mutable balance/head。 |
| rollback | wrong order、extra/missing write、connection ABA、guard drop、commit failure均无row/permit残留。 |

## 6. Source-written 前置门

后续源码批还必须同时具备：

- 独立 `market_projection_identity_abi`，冻结resource scope、Pool/bucket/meter/window、supply/event、Offer/publication
  approver、snapshot/quote、`fence_digest`、legacy deterministic IDs/times与共享server checked-at，并落实fixed source observation window；
- 首个产品/经济/安全审批的 byte-exact profile JSON/digest；
- Domain/DDL/Store exact 77-column source-contract、10组required parent keys（9新增+1复用）与全部guard；
- 完整 service-managed market transaction、Gateway A/B、task session、validator、Runner和recovery production caller；
- fresh/replay/reopen/旧库升级/中途失败动态矩阵。

缺任一项不得登记任何物理migration或预占280、创建table/UDF、打开V254 fence或把worker常量改成伪正向。

## 7. 本批静态验收

本批只允许 Markdown。必须证明父子双向链接、frontmatter、文档尺寸、本地链接、`git diff --check`、`server/src`零diff、
V280 source/table/UDF/migration注册为0、migration最高V279。禁止编译、测试、执行migration/SQLite/runtime/network；最终报告
继续写 `implementation_unwired/uncompiled/unrun` 与 `passed=0/failed=0`。
