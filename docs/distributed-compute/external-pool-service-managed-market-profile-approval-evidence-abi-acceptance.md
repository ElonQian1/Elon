---
title: 外部矿池 service-managed market profile approval evidence ABI 验收
reviewed_at: 2026-08-21
status: current
owners: backend, security, ai-economy
design_status: design_frozen
design_scope: market_profile_inventory_approval_evidence_abi_v1
implementation_status: implementation_unwired
verification_status: design_review_only
---

# 外部矿池 service-managed market profile approval evidence ABI 验收

## 1. 当前验收结论

本页只验收 [approval evidence authority](external-pool-service-managed-market-profile-approval-evidence-abi-authority.md)。Profile schema、
完整纵切与 projection approver 映射分别见
[market profile authority](external-pool-service-managed-market-profile-authority.md)、
[V280 parent authority](external-pool-service-managed-admission-runner-authority.md) 与
[projection identity authority](external-pool-service-managed-market-projection-identity-abi-authority.md)。

正式状态只能是：

```text
market_profile_inventory_approval_evidence_abi=design_frozen
initial_profile_approval_evidence=unselected
initial_profile_inventory=unselected
current_profile_authority=unconstructible
external_adapter_semantic_wire_profile=unselected
implementation=unwired/uncompiled/unrun
passed=0
failed=0
```

不得写成 approval completed、first profile approved、inventory selected、source-written、market reachable 或 Runner ready。

## 2. 静态 ABI inventory

| Case | 必须成立 |
|---|---|
| envelope | exact 7 keys；schema/canonicalization/algorithm/revision逐字固定，unknown/missing拒绝 |
| approval material | exact 10 keys；review scope、approved-only decision与confirmation逐字固定 |
| ID | exact 3-key material；domain+NUL+JCS；ID prefix固定；不得含review-material/actor/time造成循环 |
| digest | envelope保留`approval_digest` key并置空后domain digest；普通serde、删key、无domain SHA拒绝 |
| source projection | Profile六项`review_source`逐字映射evidence；source kind固定，不从caller推断 |
| canonical bytes | UTF-8 RFC8785/I-JSON、<=1MiB、parse→JCS byte-equal；duplicate/extra/float/trailing拒绝 |
| types | envelope仅`approval`为object、两revision为positive safe integer，其余字段均non-null string；类型替换拒绝 |
| time | 两次issuance step各自server clock产生canonical UTC nanos；input time拒绝，`submitted_at<=approved_at<=profile.valid_from` |
| actors | builder输出session-derived IDs；submitter与approver来自两个distinct authenticated admin/owner session；input actor/local-owner拒绝 |
| replay | current exact pair 0/1；historical exact pair必须1；same ID不同bytes、source split、删除/改写retained pair拒绝 |
| effects | compiled-consumption不查latest review DB；approval table/API/migration/current authority/market/Runner写入均为0 |

## 3. 无环 golden 与负例

Golden 必须按以下顺序生成并逐字回读：Profile ID→approval ID→双 blank review-material digest→approval evidence digest→填入
`review_source.source_digest`→final Profile digest。验收至少覆盖：

1. evidence 尝试绑定 final Profile digest、ID preimage加入review-material或三类digest互换；
2. 删除任一 blank key、只清空一个 digest、approval digest填回后不重算 final Profile；
3. approval source ID/revision/digest、approver/time与 Profile `review_source`任一漂移；
4. submitter=approver、service actor/Provider owner/caller/local-owner冒充 reviewer；
5. decision=`rejected|changes_requested`、scope/confirmation/schema/domain/revision漂移；
6. 同approval ID不同canonical bytes/digest、同Profile两个positive evidence或latest替换historical pair；
7. 普通JSON receipt通过shape validation后被直接当成 sealed current authority。
8. Profile A的approval ID/revision与Profile B的review-material或source tuple交叉拼接。

任一负例都必须失败关闭且零写入、零 current authority、零 market/Runner effect。

## 4. Compiled catalog 与 source-contract

Source-written 阶段必须证明：

- canonical DTO只在owner内解析，verified evidence与Profile/evidence pair为private-field、non-Clone/non-Debug/non-Serde；
- `profile_approval/catalog.rs`只读取checked-in canonical bytes，exact empty/one/multi选择与source tuple一致；
- Profile/evidence pair append-only retained；historical audit 0/多项或漂移报integrity failure，expired/revoked只产pure-audit authority；
- `policy.rs`只消费sealed pair，没有raw constructor、通用`into_parts`、env/DB runtime config或第二caller；
- purpose-specific issuance builder消费两个sealed authenticated actor tokens并分别采server clock，不能接raw actor/time或复用其他receipt；
- compiled字符串只能证明历史证据bytes，不能重新铸造session authority；
- first production evidence有可审计 issuance provenance、exact golden、source review和distinct真实users。

本批没有上述源码或实例，因此只运行文档链接、状态、尺寸、canonical单一真源、`server/src`零行为diff、migration/source零注册与
`git diff --check` 等静态门禁；不得编译、测试、执行migration/SQLite/runtime/network。

## 5. 状态与晋级规则

- 本页完成：只证明 `market_profile_inventory_approval_evidence_abi=design_frozen`；
- 未提交真实 evidence：`initial_profile_approval_evidence=unselected`；
- 未提交真实 Profile payload：`initial_profile_inventory=unselected`，current authority仍不可构造；
- 未选择 concrete Adapter semantic descriptor/encoder/parser：external wire仍unselected，禁止网络请求；
- 只有 evidence instance、首个 Profile、external wire与完整V280源码/动态验收同批闭合，才可按父权威评估下一状态。

最终报告必须逐项写明上列状态、`implementation_unwired/uncompiled/unrun`、`passed=0/failed=0`，并明确本批未产生审批、
Provider/Ready、Pool/Offer、Execution Receipt、Lease、Runner 或经济效果。
