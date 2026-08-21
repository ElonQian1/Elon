---
title: 外部矿池 service-managed market projection identity ABI 验收
reviewed_at: 2026-08-21
status: current
owners: backend, security, ai-economy
design_status: design_frozen
design_scope: market_projection_identity_and_legacy_owner_mapping_abi_v1
implementation_status: implementation_unwired
verification_status: design_review_only
---

# 外部矿池 service-managed market projection identity ABI 验收

## 1. 当前证据边界

本页只验收 [projection identity authority](external-pool-service-managed-market-projection-identity-abi-authority.md) 的设计冻结。
当前没有V280 Domain/table/UDF/trigger/writer/Gateway/validator/Runner源码，migration registry最高V279，V254 fence零开放，
初始profile inventory未选择、current profile authority不可构造。因此正式状态只能是：

```text
market_projection_identity_abi=design_frozen
implementation=unwired/uncompiled/unrun
passed=0
failed=0
```

本批不得用静态文档检查冒充compile/test/migration/runtime evidence。

## 2. Identity 覆盖矩阵

| object | 必须逐字冻结并审计 |
|---|---|
| resource scope | server-only key prefix/domain与4-key material；legacy digest仍由capacity owner生成 |
| Pool | prefix/domain、5-key ID material、epoch/revision1、registering→active与shared time |
| window/bucket | 两个prefix/domain、6/10-key material、单bucket identity与初始/供给后balance分型 |
| supply | 11-key transaction ID、sequence1、单movement、两条ordered leg及9-key leg ID |
| lifecycle | 7-key event ID、14-key request digest、registering→active固定source/time |
| Offer | legacy 5-key deterministic ID、SKU digest、draft1/active2 exact clone与legacy Offer digest |
| publication | V280 ID domain/material、legacy digest、scope/key、profile approver user与published-at |
| v171 snapshot | legacy snapshot/quote IDs、active Offer/window/source tuple、观察窗、expiry与legacy digest |

Wrong prefix/domain、missing/extra key、material顺序/类型漂移、随机UUID、第二次clock read或caller/env提供identity都必须拒绝。

## 3. Legacy digest 不改域矩阵

以下输出必须调用所属 owner helper并由golden vector锁byte preimage，禁止换成RFC8785/domain digest：

- resource scope/profile/meter policy、Pool、window、bucket；
- capacity supply request与17-key transaction；
- SKU与draft/active Offer；
- 13-key publication；
- v171 snapshot。

新domain只可用于authority明确列出的V280-only resource key、Pool/window/bucket/supply/leg/event/publication identity及activation
request。Publication的service actor与真实approver user必须分离；profile审批未覆盖exact review-material digest、final digest
与approval digest形成环、缺少真实`approved_by_user_id`或把Provider owner/service actor硬塞进user字段都失败关闭。

Tx-B `fence_digest`不属于本ABI。Fixture fence、random digest、route credential、Pool/admission digest都不能升格；该项继续阻断
Gateway/session/validator source-written。

## 4. 单时钟、事务与 private seam

动态/source-contract验收必须覆盖：

- fresh只读一次`SecondsFormat::Nanos,true` server clock；authority指定的create/update/occurred/recorded/quoted/published time逐字
  使用同一checked-at，window end与各expiry严格按§3公式派生；
- supply request内部legacy RFC3339 normalization不改变stored nanos time；
- replay不采新时钟，历史readback不拿mutable balance或rolling route head作恒等；
- 所有 leaf writer使用同一outer `BEGIN IMMEDIATE`，无nested transaction/commit；
- `_on` kernel不调用`now()/Utc::now()/new_id()`，只消费sealed planned material；
- active Offer state validator使用传入checked-at，不得在同一transaction偷读第二只墙钟；
- pending plan按父权威固定写序完全消费，任何漏写、错序、额外写、跨connection、rollback或drop均清空并失败关闭。

Owner-facing bucket/supply/withdraw、Offer create/revise/revoke/publication、owner/v223 snapshot 对external_pool必须拒绝；V280
private `_on`没有standalone facade、HTTP/MCP/fixture/seed/admin caller。

## 5. Replay、orphan 与 readback负例

验收至少包含：

- admission三路lookup exact命中时0 current read、0 write、0新clock；
- admission不存在但任一planned child identity已存在时拒绝，不能leaf replay后继续拼事务；
- scope/key、ID、approver、time、legacy digest、profile/allocation或bucket inventory任一漂移拒绝；
- fresh supply必须transaction sequence1、两条leg、守恒且bucket revision/through-sequence从0/null到1/1；
- historical receipt在Reservation改变available/held后仍可由immutable supply+legs重建genesis snapshot；
- publication回放逐字审scope/key/approver/published-at，而不只审Offer版本；
- snapshot source四元组、observation start/end、quote ID与physical created-at全部readback；
- Pool/profile wrapper与meter array必须exact-shape、deny unknown；full-column reader重算Pool/window/bucket/supply/event/Offer/
  publication/snapshot owner digest，不得以省列public view或basic shape validator代替；
- Pool/bucket/window/add-supply/transaction legacy schema必须逐字等于authority五个常量；wrapper、meter policy或legacy preimage的
  schema漂移及任何extra key都失败；
- region必须通过legacy Pool owner的trim/nonempty/最多80 Unicode scalar约束；81字符即使低于profile byte上限也拒绝；
- bucket/window legacy Value map必须由owner helper+golden产生；按struct顺序手拼、key order漂移拒绝；Offer curve version省略、
  与profile revision不等或与snapshot source tuple混用拒绝；
- publication fresh/replay flag与`active/none/none/none`四effect逐字匹配，但不得把ephemeral字段塞进13-key persisted digest；
- snapshot完整22字段、JSON-only source window/sample-count/rounding与physical created-at均逐字审计；active Offer校验若偷读第二只
  墙钟，或publication replay漏scope/key/approver/time，均失败；
- external_pool Pool/bucket immutable binding/config或create-time的UPDATE/REPLACE/delete失败；Pool root只接受保持registering的
  registry projection update及event-backed registering→active CAS，其他status拒绝；合法ledger/claim mutable CAS继续可达，
  historical readback不得因余额/status合法漂移而失败；
- transaction任一步失败后Pool/ledger/event/Offer/publication/snapshot/admission均无部分提交。

## 6. Source-written 晋级门

以下条件缺一不得登记物理migration、打开fence或声明source-written：

1. 首个profile payload及其真实审批用户完成产品/经济/安全批准；
2. authority中所有prefix/domain/material、legacy helper、time mapping、sealed planned token、full-column readback与immutable-column
   guards有owner-local实现和golden vectors；
3. admission Domain/77列DDL/UDF/ordered guards/Store read-write与本ABI同批实现；
4. Gateway/session/validator ABI补齐production fence、task DTO与Runner caller；
5. source-contract证明public/admin/fixture/direct-SQL无旁路，且full vertical positive/recovery矩阵可运行。

物理版本必须在实施时重读max并取next-free；不得因为阶段名V280预占migration 280。

## 7. 本批静态验收

只允许执行Markdown frontmatter、authority↔acceptance互链、本地链接、状态入口一致性、line/byte size、`git diff --check`、
source-zero与migration-max静态检查。不得编译、测试、运行migration、打开SQLite、启动runtime或联网。

完成时各状态入口必须同时写明：三个V280 ABI均为design-frozen、inventory unselected、current profile unconstructible、
table/migration/source absent、implementation unwired/uncompiled/unrun、passed=0/failed=0，并不得宣称fully frozen、implemented或
production reachable。
